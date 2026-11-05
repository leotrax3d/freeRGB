//! Temperature source identity and user-configured text-file sensors.
//!
//! Classification is a display/grouping aid only: it never decides whether a
//! channel is a pump or a fan, and an unknown category is still selectable.
//! File sensors are parsed here without I/O; the daemon performs the bounded
//! read and passes the bytes and file age in.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Largest file a text-file sensor reads (bytes). Larger files are invalid.
pub const MAX_FILE_BYTES: u64 = 4096;
pub const MAX_FILE_SENSORS: usize = 32;
/// Identifier prefix for file sensors in the shared source map.
pub const FILE_SOURCE_PREFIX: &str = "file:";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCategory {
    Cpu,
    Gpu,
    Motherboard,
    Storage,
    Liquid,
    File,
    Other,
}

/// Best-effort category from provider, device and sensor names. Matching is
/// on lowercase ASCII words; nothing here implies hardware support.
pub fn classify(provider: &str, device: &str, sensor: &str) -> SourceCategory {
    let device = device.to_ascii_lowercase();
    let sensor = sensor.to_ascii_lowercase();
    let any = |words: &[&str]| words.iter().any(|w| device.contains(w));
    if provider == "file" {
        return SourceCategory::File;
    }
    if ["liquid", "coolant", "water"]
        .iter()
        .any(|w| sensor.contains(w) || device.contains(w))
    {
        return SourceCategory::Liquid;
    }
    // LHM identifiers look like "/amdcpu/0", "/gpu-nvidia/0", "/nvme/0", "/lpc/nct6701d";
    // hwmon chip ids start with the driver name ("k10temp", "amdgpu", "nvme", "drivetemp").
    if any(&[
        "gpu-", "/gpu", "amdgpu", "nouveau", "radeon", "i915", "xe:", "nvidia",
    ]) {
        return SourceCategory::Gpu;
    }
    if any(&["cpu", "k10temp", "coretemp", "zenpower"]) {
        return SourceCategory::Cpu;
    }
    if any(&["nvme", "hdd", "ssd", "drivetemp", "storage", "/ata"]) {
        return SourceCategory::Storage;
    }
    if any(&[
        "/lpc",
        "/motherboard",
        "nct",
        "it8",
        "f718",
        "w836",
        "asus",
        "acpitz",
        "/ec",
        "-ec",
    ]) {
        return SourceCategory::Motherboard;
    }
    SourceCategory::Other
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSensor {
    /// Absolute local path. UNC/network paths need `allow_network`.
    pub path: String,
    /// Value multiplied before bounds (e.g. 0.001 for millidegrees).
    #[serde(default = "one")]
    pub scale: f64,
    #[serde(default)]
    pub offset: f64,
    #[serde(default = "default_min")]
    pub min_c: f64,
    #[serde(default = "default_max")]
    pub max_c: f64,
    /// File modification older than this is stale (treated as missing).
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub allow_network: bool,
}
fn one() -> f64 {
    1.0
}
fn default_min() -> f64 {
    -20.0
}
fn default_max() -> f64 {
    150.0
}
fn default_timeout() -> u64 {
    10_000
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSensorSet {
    #[serde(default)]
    pub sensors: BTreeMap<String, FileSensor>,
}

fn is_network_path(path: &str) -> bool {
    path.starts_with("\\\\") || path.starts_with("//")
}

fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || is_network_path(path)
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && (bytes[2] == b'\\' || bytes[2] == b'/'))
}

impl FileSensor {
    pub fn validate(&self) -> Result<(), String> {
        if self.path.is_empty() || self.path.len() > 1024 || self.path.chars().any(char::is_control)
        {
            return Err("file sensor path must be 1..1024 printable characters".into());
        }
        if !is_absolute(&self.path) {
            return Err("file sensor path must be absolute".into());
        }
        if is_network_path(&self.path) && !self.allow_network {
            return Err("network paths need allow_network:true".into());
        }
        if !self.scale.is_finite() || self.scale == 0.0 || !self.offset.is_finite() {
            return Err("scale must be finite and nonzero, offset finite".into());
        }
        if !(self.min_c.is_finite()
            && self.max_c.is_finite()
            && -100.0 <= self.min_c
            && self.min_c < self.max_c
            && self.max_c <= 200.0)
        {
            return Err("bounds must satisfy -100 <= min_c < max_c <= 200".into());
        }
        if !(500..=3_600_000).contains(&self.timeout_ms) {
            return Err("timeout_ms must be 500..3600000".into());
        }
        Ok(())
    }

    /// Parses the first number in `text` (whitespace-trimmed). `age_ms` is the
    /// time since the file was last modified. Returns `Err` for every invalid
    /// or stale case; callers treat that as a missing reading.
    pub fn parse(&self, text: &[u8], age_ms: u64) -> Result<f64, String> {
        if text.len() as u64 > MAX_FILE_BYTES {
            return Err("file larger than 4096 bytes".into());
        }
        if age_ms > self.timeout_ms {
            return Err("file not updated within timeout".into());
        }
        let text = std::str::from_utf8(text).map_err(|_| "file is not UTF-8")?;
        let token = text
            .split(|c: char| c.is_whitespace() || c == ';' || c == ',')
            .find(|t| !t.is_empty())
            .ok_or("file is empty")?;
        let raw: f64 = token.parse().map_err(|_| {
            format!(
                "not a number: {}",
                token.chars().take(32).collect::<String>()
            )
        })?;
        let value = raw * self.scale + self.offset;
        if !value.is_finite() || value < self.min_c || value > self.max_c {
            return Err("value outside configured bounds".into());
        }
        Ok(value)
    }
}

impl FileSensorSet {
    pub fn validate(&self) -> Result<(), String> {
        if self.sensors.len() > MAX_FILE_SENSORS {
            return Err(format!("at most {MAX_FILE_SENSORS} file sensors"));
        }
        for (name, sensor) in &self.sensors {
            if name.is_empty()
                || name.len() > 64
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err("file sensor names are 1..64 of [A-Za-z0-9_-]".into());
            }
            sensor.validate().map_err(|e| format!("{name}: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor(path: &str) -> FileSensor {
        serde_json::from_value(serde_json::json!({ "path": path })).unwrap()
    }

    #[test]
    fn classifies_common_provider_identities() {
        use SourceCategory::*;
        assert_eq!(
            classify(
                "mainboard.lhm",
                "mainboard.lhm:/amdcpu/0",
                "Core (Tctl/Tdie)"
            ),
            Cpu
        );
        assert_eq!(
            classify(
                "mainboard.lhm",
                "mainboard.lhm:/gpu-nvidia/0",
                "GPU Hot Spot"
            ),
            Gpu
        );
        assert_eq!(
            classify("mainboard.lhm", "mainboard.lhm:/nvme/0", "Temperature"),
            Storage
        );
        assert_eq!(
            classify("mainboard.lhm", "mainboard.lhm:/lpc/nct6701d/0", "CPU"),
            Motherboard
        );
        assert_eq!(
            classify("mainboard.hwmon", "mainboard.hwmon:k10temp@pci", "Tctl"),
            Cpu
        );
        assert_eq!(
            classify("mainboard.hwmon", "mainboard.hwmon:amdgpu@pci", "junction"),
            Gpu
        );
        assert_eq!(classify("device", "link-hub", "Liquid temperature"), Liquid);
        assert_eq!(classify("file", "x", "y"), File);
        assert_eq!(classify("device", "something", "t"), Other);
    }

    #[test]
    fn file_sensor_parses_bounds_scale_and_freshness() {
        let mut s = sensor("/run/temp");
        s.validate().unwrap();
        assert_eq!(s.parse(b" 42.5\n", 0).unwrap(), 42.5);
        assert_eq!(s.parse(b"41;extra", 0).unwrap(), 41.0);
        assert!(s.parse(b"", 0).is_err());
        assert!(s.parse(b"hot", 0).is_err());
        assert!(s.parse(b"NaN", 0).is_err());
        assert!(s.parse(b"500", 0).is_err());
        assert!(s.parse(b"42", 10_001).is_err());
        assert!(s.parse(&[b'1'; 4097], 0).is_err());
        assert!(s.parse(&[0xff, 0xfe], 0).is_err());
        s.scale = 0.001;
        assert_eq!(s.parse(b"45000", 0).unwrap(), 45.0);
    }

    #[test]
    fn file_sensor_rejects_relative_network_and_bad_limits() {
        assert!(sensor("temp.txt").validate().is_err());
        assert!(sensor("\\\\server\\share\\t.txt").validate().is_err());
        let mut net = sensor("\\\\server\\share\\t.txt");
        net.allow_network = true;
        net.validate().unwrap();
        sensor("C:\\temps\\gpu.txt").validate().unwrap();
        let mut s = sensor("/t");
        s.min_c = 90.0;
        s.max_c = 10.0;
        assert!(s.validate().is_err());
        let mut s = sensor("/t");
        s.timeout_ms = 10;
        assert!(s.validate().is_err());
        let mut set = FileSensorSet::default();
        set.sensors.insert("bad name".into(), sensor("/t"));
        assert!(set.validate().is_err());
    }
}
