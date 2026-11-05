//! Motherboard RGB settings (docs/MAINBOARD_RGB.md): the global "manage
//! motherboard RGB" switch and, per controller, the LED count and name of each
//! addressable header. Per-user config file `mainboard-rgb.json`, edited from
//! both GUIs over RPC. The daemon's `--enable-asus-aura`/`--asus-argb-leds`
//! flags remain as a test override.
use freergb_protocols::mainboard_headers::{ColorOrder, Fallback, HeaderReport, ReportCell};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

/// Addressable headers per controller (ASUS Aura has up to four).
pub const MAX_HEADERS: usize = 4;
pub const MAX_LEDS: u16 = 512;
pub const MAX_CONTROLLERS: usize = 8;
const MAX_NAME: usize = 40;
/// Common ARGB fan LED counts offered as presets.
pub const FAN_PRESETS: [u16; 4] = [8, 12, 16, 18];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    #[serde(default)]
    pub leds: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Colour byte order of the attached strip (ASUS Aura only).
    #[serde(default, skip_serializing_if = "is_default")]
    pub order: ColorOrder,
    /// What the header shows once freeRGB releases it (ASUS Aura only).
    #[serde(default, skip_serializing_if = "is_default")]
    pub fallback: Fallback,
}
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Global switch; on by default. Off leaves every motherboard controller alone.
    #[serde(default = "enabled")]
    pub enabled: bool,
    /// By controller key `vvvv:pppp` (lower-case hex USB IDs).
    #[serde(default)]
    pub controllers: BTreeMap<String, Vec<Header>>,
}
fn enabled() -> bool {
    true
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            controllers: BTreeMap::new(),
        }
    }
}

pub fn key(vid: u16, pid: u16) -> String {
    format!("{vid:04x}:{pid:04x}")
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.controllers.len() > MAX_CONTROLLERS {
            return Err("Too many motherboard RGB controllers".into());
        }
        for (key, headers) in &self.controllers {
            let valid_key = key.len() == 9
                && key.as_bytes()[4] == b':'
                && key
                    .bytes()
                    .enumerate()
                    .all(|(i, b)| i == 4 || matches!(b, b'0'..=b'9' | b'a'..=b'f'));
            if !valid_key {
                return Err(format!("Invalid controller key {key:?}"));
            }
            if headers.len() > MAX_HEADERS {
                return Err("At most four ARGB headers per controller".into());
            }
            for header in headers {
                if header.leds > MAX_LEDS {
                    return Err(format!("ARGB LED counts must be 0..={MAX_LEDS}"));
                }
                if let Some(name) = &header.name {
                    if name.trim().is_empty()
                        || name.chars().count() > MAX_NAME
                        || name.chars().any(char::is_control)
                    {
                        return Err("Header names must be 1..40 printable characters".into());
                    }
                }
            }
        }
        Ok(())
    }
    /// LED counts and names for `N` headers of one controller (missing = 0/default).
    pub fn headers<const N: usize>(&self, vid: u16, pid: u16) -> ([u16; N], [Option<String>; N]) {
        let mut leds = [0; N];
        let mut names: [Option<String>; N] = std::array::from_fn(|_| None);
        if let Some(headers) = self.controllers.get(&key(vid, pid)) {
            for (index, header) in headers.iter().take(N).enumerate() {
                leds[index] = header.leds;
                names[index] = header.name.clone();
            }
        }
        (leds, names)
    }
    /// Colour order and release fallback for `N` headers (missing = defaults).
    pub fn header_options<const N: usize>(
        &self,
        vid: u16,
        pid: u16,
    ) -> ([ColorOrder; N], [Fallback; N]) {
        let mut orders = [ColorOrder::default(); N];
        let mut fallbacks = [Fallback::default(); N];
        if let Some(headers) = self.controllers.get(&key(vid, pid)) {
            for (index, header) in headers.iter().take(N).enumerate() {
                orders[index] = header.order;
                fallbacks[index] = header.fallback;
            }
        }
        (orders, fallbacks)
    }
}

#[derive(Default)]
pub struct MainboardRgb {
    settings: Settings,
    path: Option<PathBuf>,
    error: Option<String>,
    /// Running vendor software last seen by the hardware scan.
    vendor_software: Vec<&'static str>,
    /// Header tables the adapters read on their last connect, by controller key.
    reports: BTreeMap<String, ReportCell>,
}

impl MainboardRgb {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let mut bytes = Vec::new();
        match std::fs::File::open(&path).and_then(|f| f.take(64 * 1024).read_to_end(&mut bytes)) {
            Ok(_) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(settings) if settings.validate().is_ok() => self.settings = settings,
                _ => self.error = Some("Invalid mainboard-rgb.json ignored".into()),
            },
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => self.error = Some(e.to_string()),
            Err(_) => {}
        }
        self.path = Some(path);
        self
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn set_vendor_software(&mut self, running: Vec<&'static str>) {
        self.vendor_software = running;
    }
    pub fn vendor_software(&self) -> &[&'static str] {
        &self.vendor_software
    }
    /// Slot an adapter fills with what its controller reports (shown as "detected").
    pub fn report_cell(&mut self, vid: u16, pid: u16) -> ReportCell {
        self.reports.entry(key(vid, pid)).or_default().clone()
    }
    fn detected(&self) -> BTreeMap<String, HeaderReport> {
        self.reports
            .iter()
            .filter_map(|(key, cell)| Some((key.clone(), cell.lock().ok()?.clone()?)))
            .collect()
    }
    pub fn view(&self) -> Value {
        json!({
            "detected": self.detected(),
            "settings": self.settings,
            "max_headers": MAX_HEADERS,
            "max_leds": MAX_LEDS,
            "fan_presets": FAN_PRESETS,
            "vendor_software": self.vendor_software,
            "error": self.error,
        })
    }
    pub fn save(&mut self, settings: Settings) -> Result<(), String> {
        settings.validate()?;
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        }
        self.settings = settings;
        self.error = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_enable_management_and_settings_round_trip() {
        let dir = std::env::temp_dir().join(format!("freergb-mbrgb-{}", std::process::id()));
        let path = dir.join("mainboard-rgb.json");
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = MainboardRgb::default().with_file(path.clone());
        assert!(store.settings().enabled);
        assert_eq!(store.settings().headers::<4>(0x0b05, 0x19af).0, [0; 4]);
        let mut settings = Settings::default();
        settings.controllers.insert(
            key(0x0b05, 0x19af),
            vec![
                Header {
                    leds: 16,
                    name: Some("Front fans".into()),
                    order: ColorOrder::Grb,
                    fallback: Fallback::Off,
                },
                Header::default(),
            ],
        );
        store.save(settings.clone()).unwrap();
        let loaded = MainboardRgb::default().with_file(path);
        assert_eq!(loaded.settings(), &settings);
        let (leds, names) = loaded.settings().headers::<2>(0x0b05, 0x19af);
        assert_eq!(leds, [16, 0]);
        assert_eq!(names[0].as_deref(), Some("Front fans"));
        let (orders, fallbacks) = loaded.settings().header_options::<2>(0x0b05, 0x19af);
        assert_eq!(orders, [ColorOrder::Grb, ColorOrder::Rgb]);
        assert_eq!(fallbacks, [Fallback::Off, Fallback::Spectrum]);
        let old: Header = serde_json::from_str(r#"{"leds":8}"#).unwrap();
        assert_eq!(
            (old.order, old.fallback),
            (ColorOrder::Rgb, Fallback::Spectrum)
        );
        let legacy: Settings = serde_json::from_str("{}").unwrap();
        assert!(legacy.enabled);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_settings_are_rejected() {
        let mut store = MainboardRgb::default();
        for (key, headers) in [
            ("0b05-19af", vec![Header::default()]),
            ("0B05:19AF", vec![Header::default()]),
            (
                "0b05:19af",
                vec![Header {
                    leds: 513,
                    name: None,
                    ..Default::default()
                }],
            ),
            ("0b05:19af", vec![Header::default(); 5]),
            (
                "0b05:19af",
                vec![Header {
                    leds: 1,
                    name: Some(" ".into()),
                    ..Default::default()
                }],
            ),
            (
                "0b05:19af",
                vec![Header {
                    leds: 1,
                    name: Some("x\n".into()),
                    ..Default::default()
                }],
            ),
        ] {
            let mut settings = Settings::default();
            settings.controllers.insert(key.into(), headers);
            assert!(store.save(settings).is_err(), "{key}");
        }
        assert!(store.settings().controllers.is_empty());
    }
}
