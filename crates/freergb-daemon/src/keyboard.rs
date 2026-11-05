//! Optional per-user keyboard input and persistent reactive lighting policy.
//! Captured scan positions stay in bounded memory; never trace/persist events.
use freergb_effects::keyboard::{KeyPulses, ReactiveConfig};
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEvent {
    pub usage: u16,
    pub age_ms: u64,
    pub pressed: bool,
    pub device_id: Option<String>,
}

#[derive(Default)]
pub struct KeyboardReactive {
    pub config: ReactiveConfig,
    pulses: KeyPulses,
    preview: KeyPulses,
    path: Option<PathBuf>,
    error: Option<String>,
    pub capture_allowed: bool,
    pub macro_capture: bool,
    capture: Option<capture::Capture>,
    epoch: Option<Instant>,
}
impl KeyboardReactive {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let read = std::fs::File::open(&path).and_then(|file| {
            let mut bytes = Vec::new();
            file.take(131073).read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        match read {
            Ok(bytes) if bytes.len() <= 131072 => {
                match serde_json::from_slice::<ReactiveConfig>(&bytes) {
                    Ok(config) if config.validate().is_ok() => self.config = config,
                    _ => self.error = Some("Invalid keyboard-reactive.json ignored".into()),
                }
            }
            Ok(_) => self.error = Some("Oversized keyboard-reactive.json ignored".into()),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => self.error = Some(e.to_string()),
            _ => {}
        }
        self.path = Some(path);
        self
    }
    pub fn configure(&mut self, config: ReactiveConfig) -> Result<(), String> {
        config.validate()?;
        let bytes = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
        if bytes.len() > 131072 {
            return Err("Keyboard configuration exceeds 128 KiB".into());
        }
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(temporary, path).map_err(|e| e.to_string())?;
        }
        // Signals cleanup without blocking render/API on native hooks or child exit.
        self.capture = None;
        self.pulses.clear();
        self.preview.clear();
        self.config = config;
        self.error = None;
        Ok(())
    }
    pub fn status(&self) -> Value {
        let state = if !self.config.enabled {
            "disabled"
        } else if self.config.devices.is_empty() {
            "no_targets"
        } else if !self.capture_allowed {
            "blocked_no_hardware"
        } else if !cfg!(windows) {
            "unsupported_platform"
        } else {
            "waiting_for_keyboard"
        };
        let capture = self.capture.as_ref().map(|c| c.status());
        json!({"state":capture.as_ref().map_or(state, |c| c.0.as_str()),"error":self.error.as_ref().or_else(||capture.as_ref().and_then(|c|c.1.as_ref())),"scope":"all local keyboards; mirrored only to selected targets","records_text":false})
    }
    fn now(&mut self) -> u64 {
        self.epoch
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }
    pub fn preview(&mut self, usage: u16) -> Result<(), String> {
        if !((4..=0x73).contains(&usage) || (0xe0..=0xe7).contains(&usage)) {
            return Err("unsupported USB keyboard usage".into());
        }
        let now = self.now();
        self.preview.press(usage, now);
        Ok(())
    }
    pub fn tick(&mut self, has_target: bool) -> Vec<InputEvent> {
        let wanted = (self.config.enabled || self.macro_capture)
            && self.capture_allowed
            && has_target
            && cfg!(windows);
        if wanted && self.capture.is_none() {
            self.capture = Some(capture::Capture::start());
        }
        if !wanted {
            self.capture = None;
            self.pulses.clear();
        }
        let now = self.now();
        let mut captured = Vec::new();
        if let Some(capture) = &self.capture {
            let events = capture.drain();
            if capture.status().0 != "listening" {
                self.pulses.clear();
            } else {
                for event in &events {
                    if let Some(at) = now.checked_sub(event.age_ms).filter(|_| event.pressed) {
                        self.pulses.press(event.usage, at);
                    }
                }
                captured = events;
            }
        }
        self.pulses.expire(now, self.config.fade_ms);
        self.preview.expire(now, self.config.fade_ms);
        captured
    }
    pub fn apply(
        &self,
        id: &str,
        map: &std::collections::BTreeMap<u16, usize>,
        frame: &mut freergb_core::Frame,
    ) {
        if !self.config.devices.contains(id) {
            return;
        }
        let now = self.epoch.map_or(0, |e| {
            e.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
        });
        if self.config.enabled {
            self.pulses.apply(&self.config, map, frame, now);
        }
        self.preview.apply(&self.config, map, frame, now);
    }
    pub fn stop(&mut self) {
        self.capture = None;
        self.pulses.clear();
        self.preview.clear();
    }
    pub fn disable(&mut self) -> Value {
        let mut config = self.config.clone();
        config.enabled = false;
        let result = self.configure(config);
        self.config.enabled = false;
        self.stop();
        if let Err(error) = &result {
            self.error = Some(format!(
                "Stopped for this run; could not save disabled setting: {error}"
            ));
        }
        json!({"stopped":true,"persisted":self.path.is_some() && result.is_ok(),"error":result.err()})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_persist_without_capture_and_disable_survives_a_failed_save() {
        let root = std::env::temp_dir().join(format!(
            "freergb-keyboard-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("keyboard-reactive.json");
        let mut keyboard = KeyboardReactive::default().with_file(path.clone());
        let config = ReactiveConfig {
            enabled: true,
            devices: ["test-keyboard".into()].into(),
            ..Default::default()
        };
        keyboard.configure(config.clone()).unwrap();
        keyboard.tick(true);
        assert_eq!(keyboard.status()["state"], "blocked_no_hardware");
        assert_eq!(
            KeyboardReactive::default().with_file(path.clone()).config,
            config
        );
        let before = std::fs::read(&path).unwrap();
        let mut invalid = config.clone();
        invalid.fade_ms = 0;
        assert!(keyboard.configure(invalid).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        keyboard.path = Some(path.join("cannot-create-child.json"));
        let result = keyboard.disable();
        assert_eq!(result["stopped"], true);
        assert_eq!(result["persisted"], false);
        assert!(!keyboard.config.enabled);
        assert_eq!(keyboard.status()["state"], "disabled");
        assert!(keyboard.status()["error"]
            .as_str()
            .unwrap()
            .contains("could not save"));
        std::fs::remove_dir_all(root).unwrap();
    }
}

mod capture {
    use super::InputEvent;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, Receiver};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    #[cfg(any(windows, test))]
    #[derive(serde::Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum Record {
        Ready {
            version: u8,
        },
        Heartbeat {
            version: u8,
        },
        Press {
            version: u8,
            scan_code: u16,
            extended: bool,
            age_ms: u64,
            observed_unix_ms: u64,
            #[serde(default = "default_pressed")]
            pressed: bool,
            #[serde(default)]
            device_id: Option<String>,
        },
    }
    #[cfg(any(windows, test))]
    fn default_pressed() -> bool {
        true
    }
    #[cfg(any(windows, test))]
    #[derive(Default)]
    struct Protocol {
        ready: bool,
    }
    #[cfg(any(windows, test))]
    impl Protocol {
        fn read(&mut self, bytes: &[u8], now_unix_ms: u64) -> Result<Option<InputEvent>, String> {
            if bytes.len() > 512 || bytes.last() != Some(&b'\n') {
                return Err("Invalid keyboard helper message length".into());
            }
            let record = serde_json::from_slice::<Record>(bytes)
                .map_err(|_| "Keyboard helper protocol error")?;
            match record {
                Record::Ready { version: 1 } if !self.ready => {
                    self.ready = true;
                    Ok(None)
                }
                Record::Heartbeat { version: 1 } if self.ready => Ok(None),
                Record::Press {
                    version: 1,
                    scan_code,
                    extended,
                    age_ms,
                    observed_unix_ms,
                    pressed,
                    device_id,
                } if self.ready
                    && (1..=255).contains(&scan_code)
                    && device_id
                        .as_ref()
                        .is_none_or(|id| !id.is_empty() && id.len() <= 512) =>
                {
                    // Include time buffered in the pipe. Future timestamps after a
                    // clock adjustment are discarded instead of replaying input.
                    let Some(transit_age) = now_unix_ms.checked_sub(observed_unix_ms) else {
                        return Ok(None);
                    };
                    let age_ms = age_ms.max(transit_age);
                    Ok((age_ms <= 250)
                        .then(|| {
                            freergb_effects::keyboard::scan_to_usage(scan_code, extended).map(
                                |usage| InputEvent {
                                    usage,
                                    age_ms,
                                    pressed,
                                    device_id,
                                },
                            )
                        })
                        .flatten())
                }
                _ => Err("Unsupported keyboard helper record".into()),
            }
        }
    }
    pub struct Capture {
        stop: Arc<AtomicBool>,
        status: Arc<Mutex<(String, Option<String>)>>,
        receiver: Receiver<(InputEvent, Instant)>,
    }
    impl Capture {
        pub fn status(&self) -> (String, Option<String>) {
            self.status
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }
        pub fn drain(&self) -> Vec<InputEvent> {
            self.receiver
                .try_iter()
                .take(256)
                .map(|(mut event, received)| {
                    event.age_ms = event.age_ms.saturating_add(
                        received.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                    );
                    event
                })
                .filter(|event| event.age_ms <= 250)
                .collect()
        }
        pub fn start() -> Self {
            let (sender, receiver) = mpsc::sync_channel(256);
            let stop = Arc::new(AtomicBool::new(false));
            let status = Arc::new(Mutex::new(("starting".into(), None)));
            let result = Self {
                stop: stop.clone(),
                status: status.clone(),
                receiver,
            };
            // Detached cleanup is bounded in run(); dropping never blocks the render lock.
            let spawn = std::thread::Builder::new()
                .name("keyboard-reactive-input".into())
                .spawn(move || {
                    let outcome = run(&stop, &status, sender);
                    *status.lock().unwrap_or_else(|e| e.into_inner()) = match outcome {
                        Ok(()) => ("stopped".into(), None),
                        Err(error) => ("faulted".into(), Some(error)),
                    };
                });
            if spawn.is_err() {
                *result.status.lock().unwrap_or_else(|e| e.into_inner()) = (
                    "faulted".into(),
                    Some("Could not start keyboard input worker".into()),
                );
            }
            result
        }
    }
    impl Drop for Capture {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
        }
    }

    #[cfg(not(windows))]
    fn run(
        _: &AtomicBool,
        _: &Mutex<(String, Option<String>)>,
        _: mpsc::SyncSender<(InputEvent, Instant)>,
    ) -> Result<(), String> {
        Err("Keyboard capture currently supports Windows only".into())
    }
    #[cfg(windows)]
    fn run(
        stop: &AtomicBool,
        status: &Mutex<(String, Option<String>)>,
        sender: mpsc::SyncSender<(InputEvent, Instant)>,
    ) -> Result<(), String> {
        use std::time::Duration;
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
        if stop.load(Ordering::SeqCst) {
            return Ok(());
        }
        let executable = std::env::current_exe().map_err(|_| "Cannot locate keyboard helper")?;
        let helper = executable
            .parent()
            .ok_or("Cannot locate keyboard helper directory")?
            .join("freergb-keyboard-input.exe");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "Cannot start keyboard capture worker")?;
        runtime.block_on(async {
            let mut child = tokio::process::Command::new(helper).arg("--listen").creation_flags(0x08000000)
                .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).kill_on_drop(true)
                .spawn().map_err(|_|"Keyboard input helper is missing or could not start")?;
            let mut input = child.stdin.take().ok_or("Keyboard helper stdin unavailable")?;
            let mut output = BufReader::new(child.stdout.take().ok_or("Keyboard helper stdout unavailable")?);
            let outcome = async {
                let mut protocol = Protocol::default();
                loop {
                    let mut bytes = Vec::new();
                    let mut limited = (&mut output).take(513);
                    let request = tokio::time::timeout(Duration::from_secs(3), limited.read_until(b'\n', &mut bytes));
                    let read = tokio::select! {
                        _ = async { while !stop.load(Ordering::SeqCst) { tokio::time::sleep(Duration::from_millis(25)).await; } } => return Ok(()),
                        value = request => value.map_err(|_|"Keyboard helper heartbeat expired")?.map_err(|_|"Keyboard helper output failed")?,
                    };
                    if read == 0 || bytes.len() > 512 || bytes.last() != Some(&b'\n') { return Err("Keyboard helper closed or sent an invalid message".into()); }
                    let now_unix_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                        .map_err(|_| "Keyboard input clock unavailable")?.as_millis().min(u64::MAX as u128) as u64;
                    if let Some(event) = protocol.read(&bytes, now_unix_ms)? { let _ = sender.try_send((event,Instant::now())); }
                    *status.lock().unwrap_or_else(|e|e.into_inner()) = ("listening".into(),None);
                }
            }.await;
            let _ = input.shutdown().await; drop(input);
            if tokio::time::timeout(Duration::from_millis(500),child.wait()).await.is_err() { let _ = child.kill().await; }
            outcome
        })
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn protocol_requires_handshake_bounds_and_never_replays_stale_keys() {
            let press = b"{\"kind\":\"press\",\"version\":1,\"scan_code\":30,\"extended\":false,\"age_ms\":0,\"observed_unix_ms\":1000}\n";
            let mut p = Protocol::default();
            assert!(p.read(press, 1000).is_err());
            assert_eq!(
                p.read(b"{\"kind\":\"ready\",\"version\":1}\n", 1000)
                    .unwrap(),
                None
            );
            assert_eq!(
                p.read(press, 1000).unwrap(),
                Some(InputEvent {
                    usage: 4,
                    age_ms: 0,
                    pressed: true,
                    device_id: None
                })
            );
            assert_eq!(p.read(press, 1251).unwrap(), None);
            assert_eq!(p.read(press, 999).unwrap(), None);
            assert_eq!(
                p.read(press, 1100).unwrap(),
                Some(InputEvent {
                    usage: 4,
                    age_ms: 100,
                    pressed: true,
                    device_id: None
                })
            );
            assert_eq!(p.read(b"{\"kind\":\"press\",\"version\":1,\"scan_code\":30,\"extended\":false,\"age_ms\":251,\"observed_unix_ms\":1000}\n", 1000).unwrap(),None);
            for bad in [
                b"{\"kind\":\"ready\",\"version\":1}\n".as_slice(),
                b"{\"kind\":\"heartbeat\",\"version\":2}\n",
                b"{\"kind\":\"heartbeat\",\"version\":1,\"text\":\"unexpected\"}\n",
                b"{}",
            ] {
                assert!(p.read(bad, 1000).is_err());
            }
            assert!(p.read(&[b'x'; 513], 1000).is_err());
        }

        #[test]
        fn identified_release_is_optional_and_preserved_without_guessing_a_source() {
            let mut p = Protocol::default();
            p.read(b"{\"kind\":\"ready\",\"version\":1}\n", 1000)
                .unwrap();
            let event = p.read(b"{\"kind\":\"press\",\"version\":1,\"scan_code\":30,\"extended\":false,\"age_ms\":0,\"observed_unix_ms\":1000,\"pressed\":false,\"device_id\":\"fixture:keyboard\"}\n",1000).unwrap().unwrap();
            assert_eq!(event.device_id.as_deref(), Some("fixture:keyboard"));
            assert!(!event.pressed);
            assert_eq!(event.usage, 4);
        }
    }
}
