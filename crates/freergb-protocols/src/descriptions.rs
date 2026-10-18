//! Declarative device descriptions (first stage of the plugin system).
//!
//! A description is a TOML file with the device identity, which protocol
//! adapter drives it, the adapter parameters and the LED layout with each
//! LED's wire index. Adding a device that speaks an already implemented
//! protocol then needs no Rust code. Format: `docs/PLUGIN_SYSTEM.md`.
//!
//! Built-in descriptions ship inside the binary. Descriptions loaded from a
//! user directory are always treated as *experimental*, whatever they claim,
//! and the daemon only uses them when explicitly enabled.

use crate::registry::SupportLevel;
use freergb_core::{Led, LedLayout};
use freergb_device_api::DeviceKind;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Adapters a description may refer to.
pub const KNOWN_ADAPTERS: &[&str] = &["corsair-bragi"];

const BUILTIN: &[(&str, &str)] = &[(
    "corsair/k70-core.toml",
    include_str!("../devices/corsair/k70-core.toml"),
)];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDescription {
    device: RawDevice,
    #[serde(default)]
    bragi: Option<BragiParams>,
    layout: RawLayout,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDevice {
    name: String,
    vendor: String,
    kind: String,
    adapter: String,
    support: String,
    vendor_id: u16,
    product_ids: Vec<u16>,
    usage_page: u16,
    usage: u16,
}

/// Parameters of the Corsair Bragi adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BragiParams {
    /// Continuation packets need the 0x08 target bit and are acknowledged.
    pub stream_ack: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayout {
    width: u16,
    height: u16,
    leds: Vec<RawLed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLed {
    name: String,
    x: u16,
    y: u16,
    wire: u16,
}

/// A validated device description.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceDescription {
    pub name: String,
    pub vendor: String,
    pub kind: DeviceKind,
    pub adapter: &'static str,
    pub support: SupportLevel,
    pub vendor_id: u16,
    pub product_ids: Vec<u16>,
    pub usage_page: u16,
    pub usage: u16,
    pub bragi: Option<BragiParams>,
    pub layout: LedLayout,
    /// Wire index per layout LED, in layout order.
    pub wire_map: Vec<u16>,
    /// Where it came from (`builtin:…` or a file path).
    pub source: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DescriptionError {
    #[error("{source_name}: TOML: {msg}")]
    Toml { source_name: String, msg: String },
    #[error("{source_name}: {msg}")]
    Invalid { source_name: String, msg: String },
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
}

fn kind_from(s: &str) -> Option<DeviceKind> {
    Some(match s {
        "keyboard" => DeviceKind::Keyboard,
        "mouse" => DeviceKind::Mouse,
        "headset" => DeviceKind::Headset,
        "lighting_controller" => DeviceKind::LightingController,
        "fan_hub" => DeviceKind::FanHub,
        "cooler" => DeviceKind::Cooler,
        "memory" => DeviceKind::Memory,
        "other" => DeviceKind::Other,
        _ => return None,
    })
}

impl DeviceDescription {
    /// Parses and validates one description. `source_name` is used in errors.
    pub fn parse(text: &str, source_name: &str) -> Result<Self, DescriptionError> {
        let raw: RawDescription = toml::from_str(text).map_err(|e| DescriptionError::Toml {
            source_name: source_name.to_owned(),
            msg: e.to_string(),
        })?;
        let invalid = |msg: String| DescriptionError::Invalid {
            source_name: source_name.to_owned(),
            msg,
        };

        let adapter = KNOWN_ADAPTERS
            .iter()
            .copied()
            .find(|a| *a == raw.device.adapter)
            .ok_or_else(|| {
                invalid(format!(
                    "unknown adapter {:?}, known: {KNOWN_ADAPTERS:?}",
                    raw.device.adapter
                ))
            })?;
        let kind = kind_from(&raw.device.kind)
            .ok_or_else(|| invalid(format!("unknown kind {:?}", raw.device.kind)))?;
        let support = match raw.device.support.as_str() {
            "experimental" => SupportLevel::Experimental,
            "validated" => SupportLevel::Validated,
            other => {
                return Err(invalid(format!(
                    "support must be \"experimental\" or \"validated\", got {other:?}"
                )))
            }
        };
        if raw.device.product_ids.is_empty() {
            return Err(invalid("product_ids is empty".into()));
        }
        if adapter == "corsair-bragi" && raw.bragi.is_none() {
            return Err(invalid(
                "adapter corsair-bragi needs a [bragi] section".into(),
            ));
        }

        let mut wires = HashSet::new();
        for l in &raw.layout.leds {
            if !wires.insert(l.wire) {
                return Err(invalid(format!(
                    "wire index {} used twice (LED {:?})",
                    l.wire, l.name
                )));
            }
        }
        let leds = raw
            .layout
            .leds
            .iter()
            .map(|l| Led {
                name: l.name.clone(),
                pos: (l.x, l.y),
            })
            .collect();
        let layout = LedLayout::new(raw.layout.width, raw.layout.height, leds)
            .map_err(|e| invalid(e.to_string()))?;
        if layout.is_empty() {
            return Err(invalid("layout has no LEDs".into()));
        }

        Ok(Self {
            name: raw.device.name,
            vendor: raw.device.vendor,
            kind,
            adapter,
            support,
            vendor_id: raw.device.vendor_id,
            product_ids: raw.device.product_ids,
            usage_page: raw.device.usage_page,
            usage: raw.device.usage,
            bragi: raw.bragi,
            wire_map: raw.layout.leds.iter().map(|l| l.wire).collect(),
            layout,
            source: source_name.to_owned(),
        })
    }

    pub fn matches(&self, vid: u16, pid: u16, usage_page: u16, usage: u16) -> bool {
        self.vendor_id == vid
            && self.product_ids.contains(&pid)
            && self.usage_page == usage_page
            && self.usage == usage
    }
}

/// Built-in descriptions (parsed once; a broken built-in file is a bug and
/// fails the unit tests).
pub fn builtin() -> &'static [DeviceDescription] {
    static CACHE: OnceLock<Vec<DeviceDescription>> = OnceLock::new();
    CACHE.get_or_init(|| {
        BUILTIN
            .iter()
            .map(|(name, text)| {
                DeviceDescription::parse(text, &format!("builtin:{name}"))
                    .expect("built-in description is valid")
            })
            .collect()
    })
}

/// Loads every `*.toml` in `dir`. Valid files are returned with their
/// support level forced to `Experimental`; invalid ones are reported.
pub fn load_user_dir(dir: &Path) -> (Vec<DeviceDescription>, Vec<DescriptionError>) {
    let mut ok = Vec::new();
    let mut errors = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (ok, errors),
        Err(e) => {
            errors.push(DescriptionError::Io(dir.display().to_string(), e));
            return (ok, errors);
        }
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    for p in paths {
        let name = p.display().to_string();
        match std::fs::read_to_string(&p) {
            Ok(text) => match DeviceDescription::parse(&text, &name) {
                Ok(mut d) => {
                    d.support = SupportLevel::Experimental;
                    ok.push(d);
                }
                Err(e) => errors.push(e),
            },
            Err(e) => errors.push(DescriptionError::Io(name, e)),
        }
    }
    (ok, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BragiModelCheck;
    impl BragiModelCheck {
        fn wire_led_count(d: &DeviceDescription) -> usize {
            crate::corsair_bragi::BragiModel::from_description(d)
                .unwrap()
                .wire_led_count
        }
    }

    #[test]
    fn builtin_k70_matches_the_validated_table() {
        let all = builtin();
        let k70 = all
            .iter()
            .find(|d| d.product_ids.contains(&0x1BFD))
            .unwrap();
        assert_eq!(k70.support, SupportLevel::Validated);
        assert_eq!(k70.layout.len(), 107);
        // Spot checks against the USB HID Usage Tables (keyboard page 0x07);
        // modifiers and Fn use Corsair indices 105..=111 / 122.
        let wire = |name: &str| k70.wire_map[k70.layout.index_of(name).unwrap()];
        assert_eq!(wire("A"), 0x04);
        assert_eq!(wire("Esc"), 0x29);
        assert_eq!(wire("Space"), 0x2C);
        assert_eq!(wire("F12"), 0x45);
        assert_eq!(wire("Num 0"), 0x62);
        assert_eq!(wire("ISO <"), 0x64);
        assert_eq!(wire("\\"), 49);
        assert_eq!(wire("Left Ctrl"), 105);
        assert_eq!(wire("Fn"), 122);
        let cells: HashSet<_> = k70.layout.leds.iter().map(|l| l.pos).collect();
        assert_eq!(cells.len(), 107, "grid cells unique");
        assert_eq!(BragiModelCheck::wire_led_count(k70), 123);
        assert!(k70.matches(0x1B1C, 0x1BFD, 0xFF42, 0x0001));
        assert!(!k70.matches(0x1B1C, 0x1BFD, 0x0001, 0x0006));
    }

    const MINI: &str = r#"
[device]
name = "Test Pad"
vendor = "Corsair"
kind = "mouse"
adapter = "corsair-bragi"
support = "validated"
vendor_id = 0x1B1C
product_ids = [0x1234]
usage_page = 0xFF42
usage = 1

[bragi]
stream_ack = false

[layout]
width = 2
height = 1
leds = [
  { name = "A", x = 0, y = 0, wire = 0 },
  { name = "B", x = 1, y = 0, wire = 1 },
]
"#;

    #[test]
    fn parses_minimal_and_rejects_mistakes() {
        let d = DeviceDescription::parse(MINI, "mini").unwrap();
        assert_eq!(d.kind, DeviceKind::Mouse);
        assert_eq!(d.wire_map, vec![0, 1]);

        let cases = [
            (MINI.replace("corsair-bragi", "made-up"), "unknown adapter"),
            (MINI.replace("\"mouse\"", "\"toaster\""), "unknown kind"),
            (
                MINI.replace("\"validated\"", "\"tested\""),
                "support must be",
            ),
            (MINI.replace("wire = 1", "wire = 0"), "used twice"),
            (MINI.replace("x = 1, y = 0", "x = 5, y = 0"), "outside"),
            (MINI.replace("[bragi]\nstream_ack = false\n", ""), "[bragi]"),
            (
                MINI.replace("usage = 1", "usage = 1\nextra = 2"),
                "unknown field",
            ),
        ];
        for (text, needle) in cases {
            let e = DeviceDescription::parse(&text, "case")
                .unwrap_err()
                .to_string();
            assert!(e.contains(needle), "expected {needle:?} in {e:?}");
        }
    }

    #[test]
    fn user_descriptions_are_always_experimental() {
        let dir = std::env::temp_dir().join(format!("freergb-desc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load_user_dir(&dir).0.is_empty(), "missing dir is fine");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.toml"), MINI).unwrap();
        std::fs::write(dir.join("b.toml"), "not toml [").unwrap();
        std::fs::write(dir.join("ignored.txt"), "x").unwrap();
        let (ok, errors) = load_user_dir(&dir);
        assert_eq!(ok.len(), 1);
        assert_eq!(
            ok[0].support,
            SupportLevel::Experimental,
            "user files never count as validated"
        );
        assert_eq!(errors.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
