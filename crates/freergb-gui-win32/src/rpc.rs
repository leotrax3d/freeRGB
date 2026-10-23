//! Bridge to freergbd: a worker thread makes every JSON-RPC call so the
//! message loop never blocks, exactly like the egui Classic GUI. While idle
//! the daemon is polled once per second; each refresh wakes the UI with a
//! private window message.

use freergb_client::ClientError;
use serde_json::{json, Value};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Message posted to the GUI thread when new daemon state arrived.
pub const WAKE_MSG: u32 = 0x8000 + 1; // WM_APP + 1

/// A call queued by the UI.
pub struct Command {
    pub method: &'static str,
    pub params: Value,
}

/// Latest state fetched from the daemon.
#[derive(Default)]
pub struct Snapshot {
    pub exported_report: Option<std::path::PathBuf>,
    pub control: Value,
    pub connected: bool,
    pub error: Option<String>,
    pub devices: Vec<Value>,
    pub plugins: Vec<Value>,
    pub diagnostics: Option<Value>,
    pub effect: Option<Value>,
    /// `None` when the daemon runs without profile storage (`--no-state`).
    pub profiles: Option<Vec<String>>,
    /// Saved custom effects; `None` without storage (`--no-state`).
    pub custom_effects: Option<Vec<String>>,
    /// Increments with every successful fetch.
    pub generation: u64,
    pub profile_revision: u64,
}

pub fn is_setter(method: &str) -> bool {
    matches!(
        method,
        "brightness.set"
            | "color.set"
            | "effects.set"
            | "devices.set_region"
            | "devices.set_brightness"
    )
}

/// Slider writes can merge only after the most recent action/query barrier.
/// Save/run/cancel actions retain their original ordering and parameters.
fn push_command(pending: &mut Vec<Command>, command: Command) {
    if is_setter(command.method) {
        let start = pending
            .iter()
            .rposition(|p| !is_setter(p.method))
            .map_or(0, |index| index + 1);
        let mut index = start;
        while index < pending.len() {
            if pending[index].method == command.method
                && pending[index].params.get("id") == command.params.get("id")
            {
                pending.remove(index);
            } else {
                index += 1;
            }
        }
    }
    pending.push(command);
}

fn optional_query(reachable: &mut bool, method: &str) -> Option<Value> {
    if !*reachable {
        return None;
    }
    match freergb_client::call(method, Value::Null) {
        Ok(value) => Some(value),
        Err(ClientError::Rpc { .. }) => None,
        Err(_) => {
            *reachable = false;
            None
        }
    }
}

/// Saves diagnostics only to the application's per-user directory.
fn save_report(report: &Value) -> Result<std::path::PathBuf, String> {
    let root = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .ok_or("user configuration directory unavailable")?
        .join("freeRGB")
        .join("diagnostics");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = root.join(format!("report-{time}-{}.json", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, report).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(path)
}

/// Runs until the command channel closes. `wake` is the GUI window handle
/// (stored as an integer) that receives [`WAKE_MSG`] after every refresh.
pub fn worker(rx: Receiver<Command>, state: Arc<Mutex<Snapshot>>, wake: isize) {
    let mut profile_revision = 0;
    loop {
        let mut last_error = None;
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(cmd) => {
                // Coalesce bursts (e.g. slider drags): keep only the newest
                // command per method and target device.
                let mut pending = vec![cmd];
                while let Ok(c) = rx.try_recv() {
                    push_command(&mut pending, c);
                }
                for c in pending {
                    if c.method == "macros.run"
                        && !state.lock().unwrap_or_else(|e| e.into_inner()).connected
                    {
                        last_error = Some("Macro discarded: daemon is disconnected".into());
                        continue;
                    }
                    let result = if c.method == "gui.apply_custom_effect" {
                        freergb_client::call("custom_effects.get", c.params)
                            .and_then(|spec| freergb_client::call("effects.set", spec))
                    } else if c.method == "gui.export_diagnostics" {
                        match freergb_client::call("diagnostics.export", c.params) {
                            Ok(report) => match save_report(&report) {
                                Ok(path) => {
                                    state
                                        .lock()
                                        .unwrap_or_else(|e| e.into_inner())
                                        .exported_report = Some(path);
                                    Ok(Value::Bool(true))
                                }
                                Err(error) => {
                                    last_error = Some(error);
                                    continue;
                                }
                            },
                            Err(error) => Err(error),
                        }
                    } else {
                        freergb_client::call(c.method, c.params)
                    };
                    if matches!(c.method, "profiles.load")
                        && result
                            .as_ref()
                            .is_ok_and(|value| value == &Value::Bool(true))
                    {
                        profile_revision += 1;
                    }
                    if let Err(e) = result {
                        let transport_failed =
                            !matches!(e, freergb_client::ClientError::Rpc { .. });
                        last_error = Some(e.to_string());
                        if transport_failed {
                            break;
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        let mut fetched = (|| -> Result<_, freergb_client::ClientError> {
            Ok((
                freergb_client::call("devices.list", Value::Null)?,
                freergb_client::call("diagnostics.get", Value::Null)?,
                freergb_client::call("effects.get", Value::Null)?,
            ))
        })();
        let mut reachable = fetched.is_ok();
        let mut get = |method| optional_query(&mut reachable, method);
        let profiles =
            get("profiles.list").and_then(|v| serde_json::from_value::<Vec<String>>(v).ok());
        let plugins = get("plugins.list")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        let custom_effects =
            get("custom_effects.list").and_then(|v| serde_json::from_value::<Vec<String>>(v).ok());
        let control = json!({
            "keyboard_reactive": get("keyboard.reactive.get").unwrap_or(Value::Null),
            "curves": get("cooling.curves.list").unwrap_or(Value::Null),
            "macros": get("macros.list").unwrap_or(Value::Null),
            "macro_status": get("macros.status").unwrap_or(Value::Null),
            "sensor_sources": get("cooling.sensors.sources").unwrap_or(Value::Null),
            "sensor_values": get("cooling.sensors.values").unwrap_or(Value::Null),
            "sensor_graph": get("cooling.sensors.get").unwrap_or(Value::Null),
            "cooling_providers": get("cooling.providers.list").unwrap_or(Value::Null),
            "cooling_channels": get("cooling.channels.list").unwrap_or(Value::Null),
            "cooling_session": get("cooling.sessions.status").unwrap_or(Value::Null),
            "cooling_background": get("cooling.background.get").unwrap_or(Value::Null),
            "cooling_support": get("cooling.support.get").unwrap_or(Value::Null),
            "cooling_profiles": get("cooling.profiles.list").unwrap_or(Value::Null),
            "channel_bindings": get("cooling.bindings.get").unwrap_or(Value::Null),
            "monitoring_sensors": get("monitoring.sensors.list").unwrap_or(Value::Null),
        });
        if !reachable && fetched.is_ok() {
            fetched = Err(ClientError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "connection lost while refreshing daemon state",
            )));
        }
        {
            let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
            match fetched {
                Ok((devices, diag, effect)) => {
                    s.connected = true;
                    s.control = control;
                    s.devices = devices.as_array().cloned().unwrap_or_default();
                    s.plugins = plugins;
                    s.diagnostics = Some(diag);
                    s.effect = Some(effect);
                    s.profiles = profiles;
                    s.custom_effects = custom_effects;
                    s.generation += 1;
                    s.profile_revision = profile_revision;
                    s.error = last_error;
                }
                Err(e) => {
                    let generation = s.generation;
                    *s = Snapshot {
                        error: Some(e.to_string()),
                        generation,
                        ..Snapshot::default()
                    };
                }
            }
        }
        post_wake(wake);
    }
}

#[cfg(windows)]
fn post_wake(wake: isize) {
    unsafe {
        if wake != 0 {
            crate::ffi::PostMessageW(wake as _, WAKE_MSG, 0, 0);
        }
    }
}

#[cfg(not(windows))]
fn post_wake(_wake: isize) {}

/// Spawns the worker and returns the command sender plus the shared snapshot.
pub fn start(wake: isize) -> (Sender<Command>, Arc<Mutex<Snapshot>>) {
    let state = Arc::new(Mutex::new(Snapshot::default()));
    let (tx, rx) = mpsc::channel();
    let shared = state.clone();
    std::thread::Builder::new()
        .name("api-worker".into())
        .spawn(move || worker(rx, shared, wake))
        .expect("spawn worker thread");
    (tx, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_actions_are_barriers_and_setters_coalesce() {
        let mut pending = Vec::new();
        push_command(
            &mut pending,
            Command {
                method: "brightness.set",
                params: json!({"value": 0.2}),
            },
        );
        push_command(
            &mut pending,
            Command {
                method: "brightness.set",
                params: json!({"value": 0.8}),
            },
        );
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].params["value"], 0.8);
        push_command(
            &mut pending,
            Command {
                method: "profiles.save",
                params: json!({"name": "Dim"}),
            },
        );
        push_command(
            &mut pending,
            Command {
                method: "brightness.set",
                params: json!({"value": 0.4}),
            },
        );
        assert_eq!(pending.len(), 3);
        // Different devices stay separate.
        push_command(
            &mut pending,
            Command {
                method: "devices.set_brightness",
                params: json!({"id": "a", "value": 0.4}),
            },
        );
        push_command(
            &mut pending,
            Command {
                method: "devices.set_brightness",
                params: json!({"id": "b", "value": 0.6}),
            },
        );
        let setters = pending.iter().filter(|c| is_setter(c.method)).count();
        assert_eq!(setters, 3);
    }
}