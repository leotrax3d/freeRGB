//! Bounded daemon-owned channel annotations. No pairing guesses or fan writes.
use crate::mainboard::Channel;
use freergb_control::channels::{BindingState, ChannelBindings, ChannelCandidate};
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;

const MAX_BYTES: usize = 256 * 1024;
#[derive(Default)]
pub struct Bindings {
    config: ChannelBindings,
    path: Option<PathBuf>,
    error: Option<String>,
}
pub enum SaveError {
    Invalid(String),
    Io(String),
}
fn candidates(channels: &[Channel]) -> Vec<ChannelCandidate<'_>> {
    channels
        .iter()
        .map(|c| ChannelCandidate {
            provider: c.provider,
            device_id: &c.device_id,
            id: &c.id,
            kind: &c.kind,
            control_available: c.control_available,
            fresh: c.reading.age_ms <= 3000 && c.reading.value.is_some(),
        })
        .collect()
}
impl Bindings {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let read = std::fs::File::open(&path).and_then(|f| {
            let mut bytes = Vec::new();
            f.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        match read {
            Ok(bytes) if bytes.len() <= MAX_BYTES => {
                match serde_json::from_slice::<ChannelBindings>(&bytes) {
                    Ok(config) if config.validate().is_ok() => self.config = config,
                    _ => self.error = Some("Invalid cooling-channel-bindings.json ignored".into()),
                }
            }
            Ok(_) => self.error = Some("Oversized cooling-channel-bindings.json ignored".into()),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => self.error = Some(e.to_string()),
            _ => {}
        }
        self.path = Some(path);
        self
    }
    pub fn view(&self, channels: &[Channel]) -> Value {
        let candidates = candidates(channels);
        let assignments: Vec<_> = self
            .config
            .bindings
            .iter()
            .map(|b| {
                json!({
                    "control_id":b.control_id, "state":b.resolve(&candidates),
                    "hardware_verified":false, "writable":false,
                })
            })
            .collect();
        json!({"config":self.config, "assignments":assignments, "error":self.error,
            "persistence_enabled":self.path.is_some()})
    }
    pub fn save(&mut self, config: ChannelBindings, channels: &[Channel]) -> Result<(), SaveError> {
        config.validate().map_err(SaveError::Invalid)?;
        let candidates = candidates(channels);
        if config.bindings.iter().any(|b| {
            matches!(
                b.resolve(&candidates),
                BindingState::IdentityMismatch | BindingState::IncompatibleChannel
            )
        }) {
            return Err(SaveError::Invalid(
                "Channel kind, capability or device identity does not match the cached inventory"
                    .into(),
            ));
        }
        let bytes =
            serde_json::to_vec_pretty(&config).map_err(|e| SaveError::Invalid(e.to_string()))?;
        if bytes.len() > MAX_BYTES {
            return Err(SaveError::Invalid("Channel bindings exceed 256 KiB".into()));
        }
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| SaveError::Io(e.to_string()))?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes).map_err(|e| SaveError::Io(e.to_string()))?;
            std::fs::rename(&temporary, path).map_err(|e| SaveError::Io(e.to_string()))?;
        }
        self.config = config;
        self.error = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_control::channels::{ChannelBinding, ChannelRole, ElectricalMode};
    #[test]
    fn offline_annotations_persist_but_failed_save_preserves_previous_state() {
        let root = std::env::temp_dir().join(format!(
            "freergb-bindings-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("cooling-channel-bindings.json");
        let config = ChannelBindings {
            version: 1,
            bindings: vec![ChannelBinding {
                provider: "mainboard.lhm".into(),
                device_id: "board".into(),
                control_id: "control".into(),
                tachometer_id: None,
                header: "CPU_FAN".into(),
                role: ChannelRole::Fan,
                electrical_mode: ElectricalMode::Unknown,
            }],
        };
        let mut store = Bindings::default().with_file(path.clone());
        assert!(store.save(config.clone(), &[]).is_ok());
        let mut channel = Channel {
            provider: "mainboard.lhm",
            id: "control".into(),
            device_id: "another-board".into(),
            name: "Control".into(),
            kind: "control".into(),
            reading: freergb_device_api::SensorReading {
                id: "control".into(),
                name: "Control".into(),
                unit: "%".into(),
                value: Some(50.0),
                age_ms: 0,
            },
            control_available: true,
            writable: false,
            physical_mapping: "unconfirmed",
            electrical_mode: "unknown",
        };
        assert!(matches!(
            store.save(config.clone(), std::slice::from_ref(&channel)),
            Err(SaveError::Invalid(_))
        ));
        channel.device_id = "board".into();
        channel.kind = "fan".into();
        assert!(matches!(
            store.save(config.clone(), std::slice::from_ref(&channel)),
            Err(SaveError::Invalid(_))
        ));
        channel.kind = "control".into();
        channel.reading.age_ms = 3001;
        assert!(store
            .save(config.clone(), std::slice::from_ref(&channel))
            .is_ok());
        assert_eq!(store.view(&[channel])["assignments"][0]["state"], "stale");
        let reopened = Bindings::default().with_file(path.clone());
        assert_eq!(reopened.config, config);
        assert_eq!(reopened.view(&[])["assignments"][0]["state"], "unavailable");
        assert_eq!(reopened.view(&[])["assignments"][0]["writable"], false);
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(matches!(
            store.save(ChannelBindings::default(), &[]),
            Err(SaveError::Io(_))
        ));
        assert_eq!(store.config, config);
        let invalid = Bindings::default().with_file(path.clone());
        assert!(invalid.error.is_some());
        assert!(invalid.config.bindings.is_empty());
        std::fs::remove_dir(&path).unwrap();
        std::fs::write(&path, vec![b' '; MAX_BYTES + 1]).unwrap();
        let oversized = Bindings::default().with_file(path);
        assert!(oversized.error.as_deref().unwrap().contains("Oversized"));
        assert!(oversized.config.bindings.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
