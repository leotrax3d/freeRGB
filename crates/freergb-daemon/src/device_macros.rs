//! Device-identified input only. Anonymous/global keyboard events never trigger.
use crate::keyboard::InputEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    device_id: String,
    usage: u16,
    macro_name: String,
}

#[derive(Default)]
pub struct DeviceMacros {
    pub bindings: Vec<Binding>,
    pub enabled: bool,
    path: Option<PathBuf>,
    held: BTreeSet<(String, u16)>,
    last: Option<Instant>,
}

impl DeviceMacros {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let read = std::fs::File::open(&path).and_then(|file| {
            let mut bytes = Vec::new();
            file.take(32769).read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        if let Ok(bytes) = read {
            if bytes.len() <= 32768 {
                if let Ok(bindings) = serde_json::from_slice::<Vec<Binding>>(&bytes) {
                    if Self::validate_shape(&bindings).is_ok() {
                        self.bindings = bindings;
                    }
                }
            }
        }
        self.path = Some(path);
        self
    }
    fn validate_shape(bindings: &[Binding]) -> Result<(), String> {
        if bindings.len() > 64 {
            return Err("maximum 64 device bindings".into());
        }
        let mut keys = BTreeSet::new();
        for b in bindings {
            // F6–F10 only. Typing/modifier keys and cancellation F11 are reserved.
            if !(0x3f..=0x43).contains(&b.usage)
                || b.device_id.is_empty()
                || b.device_id.len() > 512
            {
                return Err(
                    "use a mapped F6–F10 key and a valid device id; other keys are reserved".into(),
                );
            }
            crate::profiles::validate_name(&b.macro_name).map_err(|e| e.to_string())?;
            if !keys.insert((&b.device_id, b.usage)) {
                return Err("duplicate device key".into());
            }
        }
        Ok(())
    }
    pub fn validate(bindings: &[Binding], devices: &Value, macros: &Value) -> Result<(), String> {
        Self::validate_shape(bindings)?;
        for b in bindings {
            let mapped =
                devices.as_array().into_iter().flatten().any(|d| {
                    d["id"] == b.device_id && d["keys"].get(b.usage.to_string()).is_some()
                });
            if !mapped || macros.get(&b.macro_name).is_none() {
                return Err("unknown mapped device key or saved macro".into());
            }
        }
        Ok(())
    }
    pub fn save(
        &mut self,
        bindings: Vec<Binding>,
        devices: &Value,
        macros: &Value,
    ) -> Result<(), String> {
        Self::validate(&bindings, devices, macros)?;
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(
                &tmp,
                serde_json::to_vec(&bindings).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::rename(tmp, path).map_err(|e| e.to_string())?;
        }
        self.bindings = bindings;
        self.disable();
        Ok(())
    }
    pub fn disable(&mut self) {
        self.enabled = false;
        self.held.clear();
        self.last = None;
    }
    pub fn dispatch(
        &mut self,
        event: &InputEvent,
        now: Instant,
        devices: &Value,
        macros: &Value,
    ) -> Option<String> {
        if !self.enabled || event.age_ms > 250 {
            return None;
        }
        let device = event.device_id.as_ref()?;
        let key = (device.clone(), event.usage);
        if !event.pressed {
            self.held.remove(&key);
            return None;
        }
        if self.held.len() >= 64 || !self.held.insert(key) {
            return None;
        }
        // Modified F-keys belong to the existing global shortcut manager.
        if self
            .held
            .iter()
            .any(|(_, usage)| (0xe0..=0xe7).contains(usage))
        {
            return None;
        }
        if self
            .last
            .is_some_and(|t| now.saturating_duration_since(t) < Duration::from_millis(500))
        {
            return None;
        }
        let binding = self
            .bindings
            .iter()
            .find(|b| b.device_id == *device && b.usage == event.usage)?;
        let connected = devices
            .as_array()
            .into_iter()
            .flatten()
            .any(|d| d["id"] == *device && d["connected"] == true);
        if !connected || Self::validate(std::slice::from_ref(binding), devices, macros).is_err() {
            return None;
        }
        self.last = Some(now);
        Some(binding.macro_name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn identity_mapping_reserved_duplicates_and_debounce() {
        let devices = json!([{"id":"keyboard","connected":true,"keys":{"63":42}}]);
        let macros = json!({"Demo":{}});
        let parse = |value| serde_json::from_value::<Vec<Binding>>(value).unwrap();
        let binding = json!({"device_id":"keyboard","usage":63,"macro_name":"Demo"});
        let mut manager = DeviceMacros::default();
        manager
            .save(parse(json!([binding])), &devices, &macros)
            .unwrap();
        assert!(!manager.enabled);
        manager.enabled = true;
        let now = Instant::now();
        let mut event = InputEvent {
            usage: 63,
            age_ms: 0,
            pressed: true,
            device_id: None,
        };
        assert!(manager.dispatch(&event, now, &devices, &macros).is_none());
        event.device_id = Some("other".into());
        assert!(manager.dispatch(&event, now, &devices, &macros).is_none());
        event.device_id = Some("keyboard".into());
        assert_eq!(
            manager.dispatch(&event, now, &devices, &macros),
            Some("Demo".into())
        );
        assert!(manager
            .dispatch(&event, now + Duration::from_secs(1), &devices, &macros)
            .is_none());
        event.pressed = false;
        manager.dispatch(&event, now, &devices, &macros);
        event.pressed = true;
        assert!(manager
            .dispatch(&event, now + Duration::from_millis(100), &devices, &macros)
            .is_none());
        for invalid in [
            json!([binding, binding]),
            json!([{"device_id":"keyboard","usage":64,"macro_name":"Demo"}]),
            json!([{"device_id":"keyboard","usage":68,"macro_name":"Demo"}]),
            json!([{"device_id":"keyboard","usage":63,"macro_name":"Missing"}]),
        ] {
            assert!(manager.save(parse(invalid), &devices, &macros).is_err());
        }
    }
}
