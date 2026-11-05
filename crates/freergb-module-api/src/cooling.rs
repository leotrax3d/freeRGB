//! API 3 cooling plugin protocol: third-party sensors and fan controls.
//!
//! Same process model as the v1 HID protocol (JSON lines on stdin/stdout,
//! daemon-owned device handle, brokered transport calls with `"v":3`), plus a
//! signed [`Policy`] in the package manifest that declares every sensor and
//! channel, the channel roles and floors, and an allowlist of the HID reports
//! the plugin may move. The host enforces lease, floor, pump rules, freshness
//! and an independent fail-safe; see `docs/COOLING_PLUGINS.md`.
use crate::{ModuleError, ModuleInfo, Op, MAX_REPORT};
use serde::{Deserialize, Serialize};

pub mod sdk;

pub const API: u32 = 3;
pub const MAX_SENSORS: usize = 64;
pub const MAX_CHANNELS: usize = 32;
pub const MAX_RULES: usize = 64;
pub const MAX_FAILSAFE: usize = 8;
pub const MAX_PREFIX: usize = 64;
/// Lowest floor a plugin fan channel may declare (mirrors
/// `freergb_control::fan_support::SAFE_FAN_FLOOR`). Plugins get no
/// unrestricted tier.
pub const MIN_FAN_FLOOR: u8 = 40;
/// Lease the host grants per duty command; renewed by every control tick.
pub const LEASE_MS: u64 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Sensor,
    Control,
    Both,
}

impl Capability {
    pub fn sensors(self) -> bool {
        matches!(self, Self::Sensor | Self::Both)
    }
    pub fn controls(self) -> bool {
        matches!(self, Self::Control | Self::Both)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Celsius,
    Rpm,
    Percent,
}

impl Unit {
    fn bounds(self) -> (f64, f64) {
        match self {
            Self::Celsius => (-50.0, 200.0),
            Self::Rpm => (0.0, 100_000.0),
            Self::Percent => (0.0, 100.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensorDecl {
    pub id: String,
    pub name: String,
    pub unit: Unit,
    /// Plausible range; a value outside it is malformed plugin output.
    pub min: f64,
    pub max: f64,
}

/// Pumps and unknown channels are never reduced: their floor must be 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Fan,
    Pump,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelDecl {
    pub id: String,
    pub name: String,
    pub role: Role,
    pub floor_percent: u8,
    /// Declared RPM sensor of this channel; its freshness gates control.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tachometer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HidMatch {
    pub vid: u16,
    pub pid: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_page: Option<u16>,
}

/// One allowed transport call: same op, data starting with `prefix`, at most
/// `max_len` bytes (written bytes, or the read/feature buffer length).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HidRule {
    pub op: Op,
    #[serde(default)]
    pub prefix: Vec<u8>,
    pub max_len: usize,
}

/// A static report the *host* writes on the daemon-owned handle after the
/// plugin crashed, hung or violated policy (e.g. "back to hardware mode").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailsafeReport {
    pub op: Op,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case", deny_unknown_fields)]
pub enum Io {
    /// No brokered I/O. Channels of such a plugin are shown as virtual.
    None,
    Hid {
        devices: Vec<HidMatch>,
        rules: Vec<HidRule>,
        #[serde(default)]
        failsafe: Vec<FailsafeReport>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub capability: Capability,
    #[serde(default)]
    pub sensors: Vec<SensorDecl>,
    #[serde(default)]
    pub channels: Vec<ChannelDecl>,
    pub io: Io,
    /// How often the host samples (250..=5000 ms).
    pub sample_interval_ms: u64,
    /// Readings older than this are reported as missing (interval..=10000 ms).
    pub max_age_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyError {
    Invalid,
    Denied,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

fn writes(op: Op) -> bool {
    matches!(op, Op::Write | Op::SendFeature)
}

impl Policy {
    pub fn validate(&self) -> Result<(), PolicyError> {
        let bad = Err(PolicyError::Invalid);
        let shape = match self.capability {
            Capability::Sensor => !self.sensors.is_empty() && self.channels.is_empty(),
            Capability::Control => !self.channels.is_empty(),
            Capability::Both => !self.sensors.is_empty() && !self.channels.is_empty(),
        };
        if !shape
            || self.sensors.len() > MAX_SENSORS
            || self.channels.len() > MAX_CHANNELS
            || !(250..=5000).contains(&self.sample_interval_ms)
            || !(self.sample_interval_ms..=10_000).contains(&self.max_age_ms)
        {
            return bad;
        }
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.sensors {
            let (lo, hi) = s.unit.bounds();
            if !valid_id(&s.id)
                || !ids.insert(s.id.as_str())
                || s.name.is_empty()
                || s.name.len() > 128
                || !(s.min.is_finite() && s.max.is_finite())
                || s.min >= s.max
                || s.min < lo
                || s.max > hi
            {
                return bad;
            }
        }
        for c in &self.channels {
            let floor_ok = match c.role {
                Role::Fan => (MIN_FAN_FLOOR..=100).contains(&c.floor_percent),
                Role::Pump | Role::Unknown => c.floor_percent == 100,
            };
            let tach_ok = c.tachometer.as_ref().is_none_or(|t| {
                self.sensors
                    .iter()
                    .any(|s| &s.id == t && s.unit == Unit::Rpm)
            });
            if !valid_id(&c.id)
                || !ids.insert(c.id.as_str())
                || c.name.is_empty()
                || c.name.len() > 128
                || !floor_ok
                || !tach_ok
            {
                return bad;
            }
        }
        if let Io::Hid {
            devices,
            rules,
            failsafe,
        } = &self.io
        {
            if devices.is_empty()
                || devices.len() > 16
                || rules.is_empty()
                || rules.len() > MAX_RULES
                || failsafe.len() > MAX_FAILSAFE
                || rules.iter().any(|r| {
                    r.max_len == 0
                        || r.max_len > MAX_REPORT
                        || r.prefix.len() > MAX_PREFIX
                        || r.prefix.len() > r.max_len
                        || (r.op == Op::Read && !r.prefix.is_empty())
                })
            {
                return bad;
            }
            // Anything that can change device state needs an independent,
            // host-executed way back.
            if rules.iter().any(|r| writes(r.op)) && failsafe.is_empty() {
                return bad;
            }
            for report in failsafe {
                if !writes(report.op)
                    || self
                        .authorize(report.op, &report.data, report.data.len())
                        .is_err()
                {
                    return bad;
                }
            }
        }
        Ok(())
    }

    /// `len` is the written length for writes and the buffer length for reads.
    pub fn authorize(&self, op: Op, data: &[u8], len: usize) -> Result<(), PolicyError> {
        let Io::Hid { rules, .. } = &self.io else {
            return Err(PolicyError::Denied);
        };
        let size = if writes(op) { data.len() } else { len };
        if size == 0 || size > MAX_REPORT {
            return Err(PolicyError::Denied);
        }
        if rules
            .iter()
            .any(|r| r.op == op && data.starts_with(&r.prefix) && size <= r.max_len)
        {
            Ok(())
        } else {
            Err(PolicyError::Denied)
        }
    }

    pub fn channel(&self, id: &str) -> Option<&ChannelDecl> {
        self.channels.iter().find(|c| c.id == id)
    }

    pub fn sensor(&self, id: &str) -> Option<&SensorDecl> {
        self.sensors.iter().find(|s| s.id == id)
    }

    pub fn failsafe(&self) -> &[FailsafeReport] {
        match &self.io {
            Io::Hid { failsafe, .. } => failsafe,
            Io::None => &[],
        }
    }

    pub fn is_virtual(&self) -> bool {
        self.io == Io::None
    }
}

/// Host -> plugin request: `{"v":3,"id":N,"method":…,"params":…}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub v: u32,
    pub id: u64,
    #[serde(flatten)]
    pub method: Method,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Method {
    /// Answer: [`ModuleInfo`] with `api: 3`. No I/O.
    Hello,
    /// Answer: [`Description`], must equal the signed policy. No I/O.
    Describe,
    /// Answer: [`Sample`] with exactly the declared sensors. I/O allowed.
    Sample,
    /// Apply `duty_percent` until `lease_ms` after receipt unless renewed.
    SetDuty {
        channel: String,
        duty_percent: u8,
        sequence: u64,
        lease_ms: u64,
    },
    /// Hand the channel back to firmware/automatic control.
    Release { channel: String },
    /// Release everything and exit.
    Shutdown,
}

impl Method {
    pub fn allows_io(&self) -> bool {
        !matches!(self, Self::Hello | Self::Describe)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Description {
    pub sensors: Vec<SensorDecl>,
    pub channels: Vec<ChannelDecl>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensorValue {
    pub id: String,
    /// `None` when the device reports the sensor as absent.
    pub value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub sensors: Vec<SensorValue>,
}

/// Plugin answer; exactly one of `result`/`error` (as in protocol v1).
pub type Response = crate::Response;

pub fn hello_ok(info: &ModuleInfo) -> bool {
    info.api == API && valid_id(&info.id)
}

pub fn error(message: impl Into<String>) -> ModuleError {
    ModuleError::new(crate::ErrorCode::Protocol, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fan(id: &str, floor: u8) -> ChannelDecl {
        ChannelDecl {
            id: id.into(),
            name: id.into(),
            role: Role::Fan,
            floor_percent: floor,
            tachometer: Some("rpm".into()),
        }
    }

    fn policy() -> Policy {
        Policy {
            capability: Capability::Both,
            sensors: vec![SensorDecl {
                id: "rpm".into(),
                name: "Fan".into(),
                unit: Unit::Rpm,
                min: 0.0,
                max: 5000.0,
            }],
            channels: vec![fan("fan1", 40)],
            io: Io::Hid {
                devices: vec![HidMatch {
                    vid: 1,
                    pid: 2,
                    interface: None,
                    usage_page: None,
                }],
                rules: vec![
                    HidRule {
                        op: Op::Write,
                        prefix: vec![0, 1],
                        max_len: 8,
                    },
                    HidRule {
                        op: Op::Read,
                        prefix: vec![],
                        max_len: 64,
                    },
                ],
                failsafe: vec![FailsafeReport {
                    op: Op::Write,
                    data: vec![0, 1, 0xff],
                }],
            },
            sample_interval_ms: 1000,
            max_age_ms: 3000,
        }
    }

    #[test]
    fn valid_policy_and_allowlist() {
        let p = policy();
        p.validate().unwrap();
        assert!(p.authorize(Op::Write, &[0, 1, 5], 3).is_ok());
        assert!(p.authorize(Op::Write, &[0, 2, 5], 3).is_err());
        assert!(p
            .authorize(Op::Write, &[0, 1, 0, 0, 0, 0, 0, 0, 0], 9)
            .is_err());
        assert!(p.authorize(Op::Read, &[], 64).is_ok());
        assert!(p.authorize(Op::Read, &[], 65).is_err());
        assert!(p.authorize(Op::SendFeature, &[0, 1], 2).is_err());
    }

    #[test]
    fn pumps_unknown_and_low_floors_are_refused() {
        for (role, floor) in [(Role::Pump, 60), (Role::Unknown, 99), (Role::Fan, 39)] {
            let mut p = policy();
            p.channels[0].role = role;
            p.channels[0].floor_percent = floor;
            assert_eq!(p.validate(), Err(PolicyError::Invalid), "{role:?}");
        }
        let mut p = policy();
        p.channels[0].role = Role::Pump;
        p.channels[0].floor_percent = 100;
        p.validate().unwrap();
    }

    #[test]
    fn writes_need_an_authorized_failsafe() {
        let mut p = policy();
        if let Io::Hid { failsafe, .. } = &mut p.io {
            failsafe.clear();
        }
        assert!(p.validate().is_err());
        let mut p = policy();
        if let Io::Hid { failsafe, .. } = &mut p.io {
            failsafe[0].data = vec![9, 9];
        }
        assert!(p.validate().is_err());
    }

    #[test]
    fn shape_ids_and_tachometer_are_checked() {
        let mut p = policy();
        p.capability = Capability::Sensor;
        assert!(p.validate().is_err(), "sensor plugin with channels");
        let mut p = policy();
        p.channels.push(fan("rpm", 50));
        assert!(p.validate().is_err(), "duplicate id across kinds");
        let mut p = policy();
        p.channels[0].tachometer = Some("missing".into());
        assert!(p.validate().is_err());
        let mut p = policy();
        p.max_age_ms = 500;
        assert!(p.validate().is_err());
    }

    #[test]
    fn wire_format() {
        let r = Request {
            v: API,
            id: 4,
            method: Method::SetDuty {
                channel: "fan1".into(),
                duty_percent: 55,
                sequence: 9,
                lease_ms: LEASE_MS,
            },
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains(r#""method":"set_duty""#), "{json}");
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back.method, r.method);
        let hello: Request = serde_json::from_str(r#"{"v":3,"id":1,"method":"hello"}"#).unwrap();
        assert_eq!(hello.method, Method::Hello);
        assert!(serde_json::from_str::<Io>(r#"{"transport":"smbus"}"#).is_err());
    }
}
