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
/// Inventory channel id for a plain sensor id (same provider), else unchanged.
fn canonical(id: &str, channels: &[Channel]) -> String {
    channels
        .iter()
        .find(|c| c.sensor_id == id && c.id != id)
        .map_or_else(|| id.to_owned(), |c| c.id.clone())
}

/// A stored id matches a channel by its channel id or (legacy) its sensor id.
fn same_channel(stored: &str, channel_id: &str) -> bool {
    stored == channel_id
        || serde_json::from_str::<(String, String)>(channel_id)
            .is_ok_and(|(_, sensor)| sensor == stored)
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
    /// Saved binding for a channel identity (`Channel::id`), if any.
    pub fn binding(&self, channel_id: &str) -> Option<freergb_control::channels::ChannelBinding> {
        self.config
            .bindings
            .iter()
            .find(|b| same_channel(&b.control_id, channel_id))
            .cloned()
    }
    /// User-assigned role for a channel identity (`Channel::id`), if any.
    pub fn role(&self, channel_id: &str) -> Option<freergb_control::channels::ChannelRole> {
        self.config
            .bindings
            .iter()
            .find(|b| same_channel(&b.control_id, channel_id))
            .map(|b| b.role)
    }
    pub fn save(
        &mut self,
        mut config: ChannelBindings,
        channels: &[Channel],
    ) -> Result<(), SaveError> {
        // One identity per channel: plain sensor ids ("/lpc/.../control/2") are
        // rewritten to the inventory's channel id, so the same header can never
        // be bound twice in two spellings (owner acceptance 2026-11-05, item 10).
        let mut legacy = std::collections::BTreeSet::new();
        for binding in &mut config.bindings {
            let id = canonical(&binding.control_id, channels);
            if id != binding.control_id {
                legacy.insert(id.clone());
            }
            binding.control_id = id;
            if let Some(tach) = binding.tachometer_id.as_mut() {
                *tach = canonical(tach, channels);
            }
        }
        // Repair: a legacy spelling and its canonical twin collapse into the
        // later entry. Two canonical entries for one channel stay an error.
        let mut seen = std::collections::BTreeSet::new();
        let mut kept: Vec<_> = config
            .bindings
            .into_iter()
            .rev()
            .filter(|b| !legacy.contains(&b.control_id) || seen.insert(b.control_id.clone()))
            .collect();
        kept.reverse();
        config.bindings = kept;
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

    fn nct_channel(kind: &str, sensor: &str) -> Channel {
        Channel {
            provider: "mainboard.lhm",
            id: serde_json::to_string(&("/lpc/nct6701d/0", sensor)).unwrap(),
            device_id: "mainboard.lhm:/lpc/nct6701d/0".into(),
            name: sensor.into(),
            kind: kind.into(),
            reading: freergb_device_api::SensorReading {
                id: sensor.into(),
                name: sensor.into(),
                unit: "%".into(),
                value: Some(50.0),
                age_ms: 0,
            },
            control_available: kind == "control",
            writable: false,
            physical_mapping: "unconfirmed",
            electrical_mode: "unknown",
            sensor_id: sensor.into(),
            hardware_name: "NCT6701D".into(),
            support: None,
        }
    }

    fn cha_fan2(control: &str, tach: &str, header: &str) -> ChannelBinding {
        ChannelBinding {
            provider: "mainboard.lhm".into(),
            device_id: "mainboard.lhm:/lpc/nct6701d/0".into(),
            control_id: control.into(),
            tachometer_id: Some(tach.into()),
            header: header.into(),
            role: ChannelRole::Fan,
            electrical_mode: ElectricalMode::Unknown,
        }
    }

    #[test]
    fn one_binding_per_channel_whatever_the_id_spelling() {
        let channels = [
            nct_channel("control", "/lpc/nct6701d/0/control/2"),
            nct_channel("fan", "/lpc/nct6701d/0/fan/2"),
        ];
        let canonical_control = channels[0].id.clone();
        let canonical_tach = channels[1].id.clone();
        let mut store = Bindings::default();
        // Owner state 2026-11-05: a plain-id CHA_FAN2 binding plus the guided
        // setup's channel-id twin ("Fan #3"). Saving repairs it to one entry.
        let both = ChannelBindings {
            version: 1,
            bindings: vec![
                cha_fan2(
                    "/lpc/nct6701d/0/control/2",
                    "/lpc/nct6701d/0/fan/2",
                    "CHA_FAN2",
                ),
                cha_fan2(&canonical_control, &canonical_tach, "Fan #3"),
            ],
        };
        assert!(store.save(both, &channels).is_ok());
        assert_eq!(store.config.bindings.len(), 1);
        assert_eq!(store.config.bindings[0].control_id, canonical_control);
        assert_eq!(
            store.config.bindings[0].tachometer_id.as_deref(),
            Some(canonical_tach.as_str())
        );
        // A plain id alone is stored canonically and still found by channel id.
        let plain = ChannelBindings {
            version: 1,
            bindings: vec![cha_fan2(
                "/lpc/nct6701d/0/control/2",
                "/lpc/nct6701d/0/fan/2",
                "CHA_FAN2",
            )],
        };
        assert!(store.save(plain, &channels).is_ok());
        assert_eq!(store.config.bindings[0].control_id, canonical_control);
        assert_eq!(store.role(&canonical_control), Some(ChannelRole::Fan));
        // Two canonical entries for one channel stay rejected.
        let twice = ChannelBindings {
            version: 1,
            bindings: vec![
                cha_fan2(&canonical_control, &canonical_tach, "CHA_FAN2"),
                cha_fan2(&canonical_control, &canonical_tach, "Fan #3"),
            ],
        };
        assert!(matches!(
            store.save(twice, &channels),
            Err(SaveError::Invalid(_))
        ));
    }

    #[test]
    fn legacy_plain_ids_on_disk_still_resolve_by_channel_id() {
        let mut store = Bindings::default();
        store.config.bindings = vec![cha_fan2(
            "/lpc/nct6701d/0/control/1",
            "/lpc/nct6701d/0/fan/1",
            "AIO_PUMP",
        )];
        store.config.bindings[0].role = ChannelRole::Pump;
        let id = serde_json::to_string(&("/lpc/nct6701d/0", "/lpc/nct6701d/0/control/1")).unwrap();
        assert_eq!(store.role(&id), Some(ChannelRole::Pump));
    }
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
            sensor_id: "control".into(),
            hardware_name: "Board".into(),
            support: None,
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
