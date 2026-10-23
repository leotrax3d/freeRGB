//! Cooling profiles: named, switchable sets of background channel curves
//! (FanControl-style "configurations"). Activating a profile re-validates every
//! channel under the current support policy and hands the result to the
//! protected service like `cooling.background.apply` (UAC). Persisted in the
//! per-user config `cooling-profiles.json`; never trusted by the service.
use crate::background::{BackgroundRequest, MAX_CHANNELS};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

const MAX_PROFILES: usize = 32;
const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilesConfig {
    pub version: u32,
    pub profiles: BTreeMap<String, BackgroundRequest>,
    /// Last profile handed to the service (display only).
    pub active: Option<String>,
}

pub fn valid_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && trimmed == name
        && name.len() <= 64
        && !name.chars().any(char::is_control)
}

#[derive(Default)]
pub struct CoolingProfiles {
    config: ProfilesConfig,
    path: Option<PathBuf>,
    error: Option<String>,
}

impl CoolingProfiles {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let mut bytes = Vec::new();
        let read =
            std::fs::File::open(&path).and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut bytes));
        match read {
            Ok(_) if bytes.len() as u64 <= MAX_BYTES => {
                match serde_json::from_slice::<ProfilesConfig>(&bytes) {
                    Ok(config) if config.version == 1 => self.config = config,
                    _ => self.error = Some("Invalid cooling-profiles.json ignored".into()),
                }
            }
            Ok(_) => self.error = Some("Oversized cooling-profiles.json ignored".into()),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => self.error = Some(e.to_string()),
            _ => {}
        }
        self.config.version = 1;
        self.path = Some(path);
        self
    }

    fn persist(&mut self) -> Result<(), String> {
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

    pub fn view(&self) -> Value {
        let profiles: Vec<_> = self
            .config
            .profiles
            .iter()
            .map(|(name, request)| json!({"name": name, "channels": request.channels.len()}))
            .collect();
        json!({"profiles": profiles, "active": self.config.active, "error": self.error})
    }

    pub fn get(&self, name: &str) -> Option<&BackgroundRequest> {
        self.config.profiles.get(name)
    }

    pub fn save(&mut self, name: &str, request: BackgroundRequest) -> Result<(), String> {
        if !valid_name(name) {
            return Err(
                "Profile names need 1..64 characters without leading/trailing spaces".into(),
            );
        }
        if request.channels.len() > MAX_CHANNELS {
            return Err(format!("At most {MAX_CHANNELS} channels per profile"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for channel in &request.channels {
            channel.curve.validate()?;
            if !seen.insert(&channel.control_id) {
                return Err("A channel appears twice in the profile".into());
            }
        }
        if !self.config.profiles.contains_key(name) && self.config.profiles.len() >= MAX_PROFILES {
            return Err(format!("At most {MAX_PROFILES} cooling profiles"));
        }
        self.config.profiles.insert(name.to_owned(), request);
        self.persist()
    }

    pub fn delete(&mut self, name: &str) -> Result<bool, String> {
        let removed = self.config.profiles.remove(name).is_some();
        if removed {
            if self.config.active.as_deref() == Some(name) {
                self.config.active = None;
            }
            self.persist()?;
        }
        Ok(removed)
    }

    pub fn mark_active(&mut self, name: &str) -> Result<(), String> {
        self.config.active = Some(name.to_owned());
        self.persist()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::BackgroundChannel;
    use freergb_control::FanCurve;

    fn channel(index: u8) -> BackgroundChannel {
        let mut curve = FanCurve {
            minimum_percent: 40.0,
            ..FanCurve::default()
        };
        for point in &mut curve.points {
            point.duty_percent = point.duty_percent.max(40.0);
        }
        BackgroundChannel {
            control_id: format!("/lpc/it8689e/0/control/{index}"),
            temperature_id: "/lpc/it8689e/0/temperature/0".into(),
            rpm_id: format!("/lpc/it8689e/0/fan/{index}"),
            curve,
            confirmed_fan: true,
        }
    }

    #[test]
    fn profiles_validate_persist_and_track_the_active_one() {
        let path =
            std::env::temp_dir().join(format!("freergb-profiles-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut store = CoolingProfiles::default().with_file(path.clone());
        let quiet = BackgroundRequest {
            channels: vec![channel(1), channel(2)],
        };
        store.save("Quiet", quiet).unwrap();
        store.save("Off", BackgroundRequest::default()).unwrap();
        for bad in ["", " Quiet", "Quiet ", "a\nb", &"x".repeat(65)] {
            assert!(
                store.save(bad, BackgroundRequest::default()).is_err(),
                "{bad:?}"
            );
        }
        assert!(store
            .save(
                "Twice",
                BackgroundRequest {
                    channels: vec![channel(1), channel(1)]
                }
            )
            .is_err());
        let mut invalid = channel(3);
        invalid.curve.points.clear();
        assert!(store
            .save(
                "Broken",
                BackgroundRequest {
                    channels: vec![invalid]
                }
            )
            .is_err());
        store.mark_active("Quiet").unwrap();
        let reloaded = CoolingProfiles::default().with_file(path.clone());
        assert_eq!(reloaded.get("Quiet").unwrap().channels.len(), 2);
        assert_eq!(reloaded.view()["active"], "Quiet");
        let mut reloaded = reloaded;
        assert!(reloaded.delete("Quiet").unwrap());
        assert!(reloaded.view()["active"].is_null());
        assert!(!reloaded.delete("Quiet").unwrap());
        std::fs::remove_file(path).unwrap();
    }
}
