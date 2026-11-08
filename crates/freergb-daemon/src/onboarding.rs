//! First-run state and the safety acknowledgement (docs/SETUP_WIZARD.md).
//! Without an acknowledgement of the current safety text version the daemon
//! refuses every fan write; RGB lighting is unaffected. Per-user config file
//! `onboarding.json`.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;

/// Bump when the safety/liability text changes materially: users confirm again.
pub const SAFETY_TEXT_VERSION: u32 = 3;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnboardingState {
    pub version: u32,
    /// Safety text version the user acknowledged, with the time.
    pub safety_acknowledged: Option<u32>,
    pub safety_acknowledged_unix: Option<u64>,
    pub completed: bool,
}

#[derive(Default)]
pub struct Onboarding {
    state: OnboardingState,
    path: Option<PathBuf>,
}

impl Onboarding {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let mut bytes = Vec::new();
        if std::fs::File::open(&path)
            .and_then(|f| f.take(64 * 1024).read_to_end(&mut bytes))
            .is_ok()
        {
            if let Ok(state) = serde_json::from_slice::<OnboardingState>(&bytes) {
                self.state = state;
            }
        }
        self.state.version = 1;
        self.path = Some(path);
        self
    }
    fn persist(&self) -> Result<(), String> {
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec_pretty(&self.state).map_err(|e| e.to_string())?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn safety_acknowledged(&self) -> bool {
        self.state.safety_acknowledged == Some(SAFETY_TEXT_VERSION)
    }
    /// Gate for every fan write path.
    pub fn require_safety(&self) -> Result<(), String> {
        if self.safety_acknowledged() {
            Ok(())
        } else {
            Err(
                "Fan control is locked until the safety notice is acknowledged in the setup wizard"
                    .into(),
            )
        }
    }
    pub fn acknowledge(&mut self, version: u32, accepted: bool) -> Result<(), String> {
        if !accepted || version != SAFETY_TEXT_VERSION {
            return Err("The current safety notice must be read and accepted".into());
        }
        self.state.safety_acknowledged = Some(version);
        self.state.safety_acknowledged_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());
        self.persist()
    }
    /// Withdraw the acknowledgement (fan control locks again).
    pub fn revoke_safety(&mut self) -> Result<(), String> {
        self.state.safety_acknowledged = None;
        self.state.safety_acknowledged_unix = None;
        self.persist()
    }
    pub fn complete(&mut self) -> Result<(), String> {
        self.state.completed = true;
        self.persist()
    }
    /// Re-open the wizard from the settings; the acknowledgement stays.
    pub fn reset(&mut self) -> Result<(), String> {
        self.state.completed = false;
        self.persist()
    }
    pub fn view(&self) -> Value {
        json!({
            "completed": self.state.completed,
            "safety_text_version": SAFETY_TEXT_VERSION,
            "safety_acknowledged": self.safety_acknowledged(),
            "safety_acknowledged_unix": self.state.safety_acknowledged_unix,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_outdated_confirmation_does_not_authorize_startup() {
        let path = std::env::temp_dir().join(format!(
            "freergb-onboarding-outdated-{}.json",
            std::process::id()
        ));
        let state = OnboardingState {
            version: 1,
            safety_acknowledged: Some(SAFETY_TEXT_VERSION + 1),
            safety_acknowledged_unix: Some(1),
            completed: true,
        };
        std::fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
        assert!(!Onboarding::default()
            .with_file(path.clone())
            .safety_acknowledged());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn safety_gate_requires_the_current_version_and_persists() {
        let path =
            std::env::temp_dir().join(format!("freergb-onboarding-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut onboarding = Onboarding::default().with_file(path.clone());
        assert!(onboarding.require_safety().is_err());
        assert!(onboarding.acknowledge(SAFETY_TEXT_VERSION, false).is_err());
        assert!(onboarding
            .acknowledge(SAFETY_TEXT_VERSION + 1, true)
            .is_err());
        onboarding.acknowledge(SAFETY_TEXT_VERSION, true).unwrap();
        onboarding.complete().unwrap();
        let reloaded = Onboarding::default().with_file(path.clone());
        assert!(reloaded.require_safety().is_ok());
        assert_eq!(reloaded.view()["completed"], true);
        let mut reloaded = reloaded;
        reloaded.reset().unwrap();
        assert_eq!(reloaded.view()["completed"], false);
        assert!(reloaded.safety_acknowledged());
        reloaded.revoke_safety().unwrap();
        assert!(reloaded.require_safety().is_err());
        let restarted = Onboarding::default().with_file(path.clone());
        assert!(!restarted.safety_acknowledged());
        std::fs::remove_file(path).unwrap();
    }
}
