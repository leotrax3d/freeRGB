//! User annotations for motherboard channels. Never grants hardware authority.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElectricalMode {
    #[default]
    Unknown,
    Pwm,
    Dc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelRole {
    Fan,
    Pump,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelBinding {
    pub provider: String,
    pub device_id: String,
    pub control_id: String,
    pub tachometer_id: Option<String>,
    pub header: String,
    pub role: ChannelRole,
    #[serde(default)]
    pub electrical_mode: ElectricalMode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelBindings {
    pub version: u32,
    pub bindings: Vec<ChannelBinding>,
}
impl Default for ChannelBindings {
    fn default() -> Self {
        Self {
            version: 1,
            bindings: Vec::new(),
        }
    }
}
fn text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}
impl ChannelBindings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.bindings.len() > 128 {
            return Err("Expected version 1 with at most 128 channel bindings".into());
        }
        let mut controls = BTreeSet::new();
        let mut tachometers = BTreeSet::new();
        let mut headers = BTreeSet::new();
        for b in &self.bindings {
            if b.provider != "mainboard.lhm"
                || !text(&b.device_id, 1024)
                || !text(&b.control_id, 4096)
                || !text(&b.header, 64)
                || b.header != b.header.trim()
            {
                return Err(
                    "Invalid provider, channel identity or header (1..64 UTF-8 bytes, trimmed)"
                        .into(),
                );
            }
            if !controls.insert((&b.provider, &b.control_id))
                || !headers.insert((&b.provider, &b.device_id, b.header.to_lowercase()))
            {
                return Err("Duplicate control or header assignment".into());
            }
            if let Some(tach) = &b.tachometer_id {
                if !text(tach, 4096)
                    || tach == &b.control_id
                    || !tachometers.insert((&b.provider, tach))
                {
                    return Err("Invalid or multiply assigned tachometer".into());
                }
            }
        }
        Ok(())
    }
}

/// Cached inventory only; constructing or resolving it performs no device I/O.
pub struct ChannelCandidate<'a> {
    pub provider: &'a str,
    pub device_id: &'a str,
    pub id: &'a str,
    pub kind: &'a str,
    pub control_available: bool,
    pub fresh: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingState {
    Unavailable,
    IdentityMismatch,
    IncompatibleChannel,
    Stale,
    UserAssigned,
}
impl ChannelBinding {
    pub fn resolve(&self, candidates: &[ChannelCandidate<'_>]) -> BindingState {
        let find = |id: &str| {
            candidates
                .iter()
                .find(|c| c.provider == self.provider && c.id == id)
        };
        let control = find(&self.control_id);
        if let Some(control) = control {
            if control.device_id != self.device_id {
                return BindingState::IdentityMismatch;
            }
            if control.kind != "control" || !control.control_available {
                return BindingState::IncompatibleChannel;
            }
        }
        let tach = match &self.tachometer_id {
            Some(id) => match find(id) {
                Some(c) => {
                    if c.device_id != self.device_id {
                        return BindingState::IdentityMismatch;
                    }
                    if c.kind != "fan" {
                        return BindingState::IncompatibleChannel;
                    }
                    Some(c)
                }
                None => None,
            },
            None => None,
        };
        let Some(control) = control else {
            return BindingState::Unavailable;
        };
        if self.tachometer_id.is_some() && tach.is_none() {
            return BindingState::Unavailable;
        }
        if !control.fresh || tach.is_some_and(|c| !c.fresh) {
            return BindingState::Stale;
        }
        BindingState::UserAssigned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding() -> ChannelBinding {
        ChannelBinding {
            provider: "mainboard.lhm".into(),
            device_id: "board".into(),
            control_id: "pwm-7".into(),
            tachometer_id: Some("rpm-2".into()),
            header: "CPU_FAN".into(),
            role: ChannelRole::Fan,
            electrical_mode: ElectricalMode::Unknown,
        }
    }
    #[test]
    fn annotations_cannot_alias_outputs_tachometers_or_headers() {
        let b = binding();
        let mut config = ChannelBindings {
            version: 1,
            bindings: vec![b.clone()],
        };
        assert!(config.validate().is_ok());
        config.bindings.push(b);
        assert!(config.validate().is_err());
        config.bindings[1].control_id = "other".into();
        config.bindings[1].header = "OTHER".into();
        assert!(config.validate().is_err()); // duplicate RPM
        config.bindings[1].tachometer_id = None;
        assert!(config.validate().is_ok());
        config.bindings[1].header = "cpu_fan".into();
        assert!(config.validate().is_err());
        config.bindings.truncate(1);
        config.bindings[0].header = "CPU\nFAN".into();
        assert!(config.validate().is_err());
        config.bindings[0] = binding();
        config.version = 2;
        assert!(config.validate().is_err());
        config.version = 1;
        config.bindings = (0..129)
            .map(|i| {
                let mut b = binding();
                b.control_id = format!("control-{i}");
                b.tachometer_id = None;
                b.header = format!("HEADER_{i}");
                b
            })
            .collect();
        assert!(config.validate().is_err());
        config.bindings.pop();
        assert!(config.validate().is_ok());
    }
    #[test]
    fn pairing_is_by_explicit_identity_and_never_implies_validation() {
        let b = binding();
        let mut candidates = vec![
            ChannelCandidate {
                provider: "mainboard.lhm",
                device_id: "board",
                id: "rpm-2",
                kind: "fan",
                control_available: false,
                fresh: true,
            },
            ChannelCandidate {
                provider: "mainboard.lhm",
                device_id: "board",
                id: "pwm-7",
                kind: "control",
                control_available: true,
                fresh: true,
            },
        ];
        assert_eq!(b.resolve(&candidates), BindingState::UserAssigned);
        candidates[0].device_id = "another-board";
        assert_eq!(b.resolve(&candidates), BindingState::IdentityMismatch);
        assert_eq!(b.resolve(&candidates[..1]), BindingState::IdentityMismatch);
        candidates[0].device_id = "board";
        candidates[0].fresh = false;
        assert_eq!(b.resolve(&candidates), BindingState::Stale);
        candidates[0].kind = "control";
        assert_eq!(b.resolve(&candidates), BindingState::IncompatibleChannel);
        assert_eq!(b.resolve(&[]), BindingState::Unavailable);
    }
}
