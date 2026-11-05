//! Boot-time / headless cooling (docs/COOLING_SERVICE.md). The Rust daemon is
//! the single curve evaluator: in service mode it starts without GUI or login
//! (`freergbd --cooling-service`), applies the boot profile (or the remembered
//! assignments) through the normal policy, and an independent watchdog thread
//! supervises it with `freergb_control::supervisor`. On a fault the watchdog
//! asks every channel to return (100 %, then firmware) and exits so the OS
//! service manager restarts the daemon; if even that is impossible, process
//! exit closes every native transport and the worker leases return the
//! channels on their own.
use freergb_control::supervisor::{Limits, Supervisor, Verdict};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    /// Started with the user session (autostart/tray); previous behaviour.
    #[default]
    Login,
    /// Started at boot by the OS service manager, no GUI or login needed.
    Service,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    pub version: u32,
    pub mode: RunMode,
    /// Profile applied at boot; `None` resumes the remembered assignments.
    #[serde(default)]
    pub boot_profile: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootAction {
    ResumeAssignments,
    ActivateProfile(String),
}

/// What the daemon does once the channel inventory is known.
pub fn boot_action(config: &ServiceConfig, profiles: &[String]) -> BootAction {
    match &config.boot_profile {
        Some(name) if config.mode == RunMode::Service && profiles.contains(name) => {
            BootAction::ActivateProfile(name.clone())
        }
        _ => BootAction::ResumeAssignments,
    }
}

/// Shared between the engine loop (writer) and the watchdog (reader).
#[derive(Default)]
pub struct Health {
    epoch: std::sync::OnceLock<Instant>,
    heartbeat_ms: AtomicU64,
    boot_done: AtomicBool,
    driver_since_ms: AtomicU64,
}

impl Health {
    fn now_ms(&self) -> u64 {
        self.epoch.get_or_init(Instant::now).elapsed().as_millis() as u64
    }
    pub fn beat(&self) {
        self.heartbeat_ms.store(self.now_ms(), Ordering::SeqCst);
    }
    pub fn boot_complete(&self) {
        self.boot_done.store(true, Ordering::SeqCst);
    }
    pub fn is_boot_complete(&self) -> bool {
        self.boot_done.load(Ordering::SeqCst)
    }
    /// A blocking native/driver request started (0 = none outstanding).
    pub fn driver_begin(&self) {
        self.driver_since_ms
            .store(self.now_ms().max(1), Ordering::SeqCst);
    }
    pub fn driver_end(&self) {
        self.driver_since_ms.store(0, Ordering::SeqCst);
    }
}

/// The watchdog side: turns the shared health values into supervisor events
/// on the watchdog's own clock (separate for tests).
pub struct Watch {
    supervisor: Supervisor,
    last_beat: u64,
}

impl Watch {
    pub fn new(limits: Limits, now_ms: u64) -> Result<Self, String> {
        Ok(Self {
            supervisor: Supervisor::new(limits, true, now_ms)?,
            last_beat: 0,
        })
    }

    pub fn observe(&mut self, health: &Health, now_ms: u64) -> Verdict {
        let beat = health.heartbeat_ms.load(Ordering::SeqCst);
        if beat != self.last_beat {
            self.last_beat = beat;
            self.supervisor.heartbeat(now_ms);
        }
        match health.driver_since_ms.load(Ordering::SeqCst) {
            0 => self.supervisor.driver_reply(now_ms),
            _ => self.supervisor.driver_request(now_ms),
        }
        if health.is_boot_complete() {
            self.supervisor.boot_complete(now_ms);
        }
        self.supervisor.check(now_ms)
    }
}

/// Starts the independent watchdog. `on_fault` must return every channel; the
/// thread then exits the process with code 3 (service manager restarts it).
pub fn spawn_watchdog(
    health: Arc<Health>,
    limits: Limits,
    on_fault: impl FnOnce(&str) + Send + 'static,
) -> Result<(), String> {
    let mut watch = Watch::new(limits, health.now_ms())?;
    std::thread::Builder::new()
        .name("cooling-watchdog".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_millis(250));
            if let Verdict::Fault(component) = watch.observe(&health, health.now_ms()) {
                let reason = format!("{component:?}");
                tracing::error!(reason, "cooling supervisor fault: returning all channels");
                on_fault(&reason);
                std::process::exit(3);
            }
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[derive(Default)]
pub struct CoolingService {
    pub config: ServiceConfig,
    path: Option<PathBuf>,
    /// Process runs as the boot-time service (`--cooling-service`).
    pub headless: bool,
    pub health: Arc<Health>,
    boot: Option<Value>,
    error: Option<String>,
}

impl CoolingService {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let mut bytes = Vec::new();
        match std::fs::File::open(&path).and_then(|f| f.take(64 * 1024).read_to_end(&mut bytes)) {
            Ok(_) => match serde_json::from_slice::<ServiceConfig>(&bytes) {
                Ok(config) if config.version == 1 => self.config = config,
                _ => self.error = Some("Invalid cooling-service.json ignored".into()),
            },
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => self.error = Some(e.to_string()),
            _ => {}
        }
        self.config.version = 1;
        self.path = Some(path);
        self
    }

    pub fn set(&mut self, config: ServiceConfig) -> Result<(), String> {
        if config
            .boot_profile
            .as_deref()
            .is_some_and(|n| !crate::cooling_profiles::valid_name(n))
        {
            return Err("Invalid boot profile name".into());
        }
        self.config = ServiceConfig {
            version: 1,
            ..config
        };
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec_pretty(&self.config).map_err(|e| e.to_string())?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        }
        self.error = None;
        Ok(())
    }

    pub fn boot_pending(&self) -> bool {
        self.boot.is_none()
    }

    pub fn finish_boot(&mut self, outcome: Value) {
        self.boot = Some(outcome);
        self.health.boot_complete();
    }

    pub fn view(&self) -> Value {
        json!({
            "mode": self.config.mode,
            "boot_profile": self.config.boot_profile,
            "headless": self.headless,
            "boot": self.boot.clone().unwrap_or(json!({"state": "pending"})),
            "error": self.error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_control::supervisor::Component;

    #[test]
    fn boot_action_needs_an_existing_profile() {
        let mut config = ServiceConfig::default();
        let names = vec!["Quiet".to_string()];
        config.boot_profile = Some("Quiet".into());
        assert_eq!(boot_action(&config, &names), BootAction::ResumeAssignments);
        config.mode = RunMode::Service;
        config.boot_profile = None;
        assert_eq!(boot_action(&config, &names), BootAction::ResumeAssignments);
        config.boot_profile = Some("Gone".into());
        assert_eq!(boot_action(&config, &names), BootAction::ResumeAssignments);
        config.boot_profile = Some("Quiet".into());
        assert_eq!(
            boot_action(&config, &names),
            BootAction::ActivateProfile("Quiet".into())
        );
    }

    #[test]
    fn config_persists_and_rejects_bad_names() {
        let path =
            std::env::temp_dir().join(format!("freergb-service-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut service = CoolingService::default().with_file(path.clone());
        assert_eq!(service.config.mode, RunMode::Login);
        assert!(service
            .set(ServiceConfig {
                version: 1,
                mode: RunMode::Service,
                boot_profile: Some(" bad".into()),
            })
            .is_err());
        service
            .set(ServiceConfig {
                version: 7,
                mode: RunMode::Service,
                boot_profile: Some("Quiet".into()),
            })
            .unwrap();
        let reloaded = CoolingService::default().with_file(path.clone());
        assert_eq!(reloaded.config.mode, RunMode::Service);
        assert_eq!(reloaded.config.boot_profile.as_deref(), Some("Quiet"));
        assert_eq!(reloaded.view()["boot"]["state"], "pending");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn watchdog_observation_detects_hang_boot_and_driver() {
        let limits = Limits::default();
        let beat = |h: &Health, t: u64| h.heartbeat_ms.store(t, Ordering::SeqCst);
        // Engine loop hang: heartbeat value stops changing.
        let health = Health::default();
        health.boot_complete();
        let mut watch = Watch::new(limits, 0).unwrap();
        beat(&health, 1);
        assert_eq!(watch.observe(&health, 1_000), Verdict::Healthy);
        assert_eq!(watch.observe(&health, 3_900), Verdict::Healthy);
        assert_eq!(
            watch.observe(&health, 4_100),
            Verdict::Fault(Component::Service)
        );
        // Boot never decided although the loop keeps beating.
        let booting = Health::default();
        let mut watch = Watch::new(limits, 0).unwrap();
        let mut verdict = Verdict::Booting;
        for t in (1_000..=91_000).step_by(1_000) {
            beat(&booting, t);
            verdict = watch.observe(&booting, t);
        }
        assert_eq!(verdict, Verdict::Fault(Component::Boot));
        // Native/driver request stuck while the loop still beats.
        let stuck = Health::default();
        stuck.boot_complete();
        let mut watch = Watch::new(limits, 0).unwrap();
        stuck.driver_since_ms.store(500, Ordering::SeqCst);
        beat(&stuck, 600);
        assert_eq!(watch.observe(&stuck, 600), Verdict::Healthy);
        beat(&stuck, 1_500);
        assert_eq!(watch.observe(&stuck, 1_500), Verdict::Healthy);
        beat(&stuck, 1_700);
        assert_eq!(
            watch.observe(&stuck, 1_700),
            Verdict::Fault(Component::Driver)
        );
    }
}
