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

/// Largest settings table: IT5711 has four external and two onboard channels.
pub const MAX_HEADERS: usize = 6;
pub const MAX_LEDS: u16 = 512;
pub const MAX_CONTROLLERS: usize = 8;
const MAX_NAME: usize = 40;
/// Common ARGB fan LED counts offered as presets.
pub const FAN_PRESETS: [u16; 4] = [8, 12, 16, 18];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    #[serde(default)]
    pub leds: u16,
    #[serde(default, rename = "type", skip_serializing_if = "is_default")]
    pub kind: HeaderType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<freergb_core::Region>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Colour byte order of the attached strip.
    #[serde(default, skip_serializing_if = "is_default")]
    pub order: ColorOrder,
    /// What the header shows once freeRGB releases it (ASUS Aura only).
    #[serde(default, skip_serializing_if = "is_default")]
    pub fallback: Fallback,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeaderType {
    Rgb,
    #[default]
    Argb,
}

pub fn header_limit(key: &str) -> usize {
    if key == "048d:5711" {
        6
    } else {
        4
    }
}

/// USB controller key for built-in and brokered module device IDs.
pub fn device_key(id: &str) -> Option<String> {
    let parts: Vec<_> = id.split(':').collect();
    let at = match parts.first().copied()? {
        "hid" => 1,
        "module" => 2,
        _ => return None,
    };
    let vid = *parts.get(at)?;
    let pid = *parts.get(at + 1)?;
    let supported = match parts[0] {
        "module" => matches!(
            parts.get(1).copied(),
            Some("msi-motherboard" | "asus-aura-usb" | "asus-aura-core" | "gigabyte-rgbfusion2")
        ),
        _ => matches!(
            (vid, pid),
            ("0b05", "18f3" | "1939" | "19af" | "1aa6" | "1bed")
                | ("048d", "5702" | "8297" | "5711" | "8950")
                | ("0414", "a100")
                | ("26ce", "01a2" | "01a6")
        ),
    };
    supported.then(|| format!("{vid}:{pid}"))
}

fn zone_label(name: &str) -> &str {
    name.rsplit_once(" LED ")
        .filter(|(_, suffix)| {
            suffix
                .chars()
                .all(|c| c.is_ascii_digit() || c == '[' || c == ']')
        })
        .map_or_else(
            || name.strip_suffix(" LED").unwrap_or(name),
            |(zone, _)| zone,
        )
}

/// Resolve adapter-named contiguous zones without assuming that onboard LEDs
/// or absent headers occupy a configured slot. ASRock's layout interleaves them.
fn header_ranges(
    key: &str,
    layout: &freergb_core::LedLayout,
    headers: &[Header],
) -> Vec<Option<(usize, usize)>> {
    let mut groups: Vec<(String, usize, usize)> = Vec::new();
    for (i, led) in layout.leds.iter().enumerate() {
        let name = zone_label(&led.name);
        if groups.last().is_some_and(|g| g.0 == name) {
            groups.last_mut().unwrap().2 += 1;
        } else {
            groups.push((name.to_owned(), i, 1));
        }
    }
    groups.retain(|g| {
        !(key.starts_with("1462:") || key.starts_with("0db0:"))
            || !g.0.to_lowercase().contains("onboard")
    });
    if key.starts_with("26ce:") || key.starts_with("048d:") || key == "0414:a100" {
        headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let default = format!("ARGB header {}", i + 1);
                groups
                    .iter()
                    .find(|g| g.0 == h.name.as_deref().unwrap_or(&default))
                    .map(|g| (g.1, g.2))
                    .or_else(|| {
                        // New Fusion variants can expose firmware channel names.
                        // Their driven zones are contiguous in configured slot order.
                        if !key.starts_with("26ce:") && h.leds > 0 {
                            let at = headers[..i].iter().filter(|h| h.leds > 0).count();
                            groups.get(at).map(|g| (g.1, g.2))
                        } else {
                            None
                        }
                    })
            })
            .collect()
    } else {
        groups.into_iter().map(|g| Some((g.1, g.2))).collect()
    }
}

pub fn fixed_headers(layout: &freergb_core::LedLayout) -> Vec<Value> {
    let mut groups: Vec<(String, usize)> = Vec::new();
    for led in &layout.leds {
        let name = zone_label(&led.name);
        if name.to_lowercase().contains("onboard") {
            continue;
        }
        if groups.last().is_some_and(|g| g.0 == name) {
            groups.last_mut().unwrap().1 += 1;
        } else {
            groups.push((name.to_owned(), 1));
        }
    }
    groups.into_iter().take(4).map(|(name,count)| json!({"name":name,"leds":count,"type":if name.starts_with("JRGB") { "rgb" } else { "argb" }})).collect()
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
            if headers.len() > header_limit(key) {
                return Err(format!("At most {} headers for {key}", header_limit(key)));
            }
            for header in headers {
                if header.region.is_some_and(|r| !r.is_valid()) {
                    return Err("Invalid header canvas region".into());
                }
                if header.kind == HeaderType::Rgb && header.leds > 1 {
                    return Err("RGB headers have one colour slot".into());
                }
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
                leds[index] = if header.kind == HeaderType::Rgb {
                    header.leds.min(1)
                } else {
                    header.leds
                };
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
    test: Option<(String, usize, std::time::Instant)>,
}

impl MainboardRgb {
    pub fn start_test(&mut self, key: String, index: usize) -> Result<(), String> {
        if !self.settings.enabled || !self.vendor_software.is_empty() {
            return Err("Close vendor lighting software and enable Headers first".into());
        }
        if index >= header_limit(&key)
            || self
                .settings
                .controllers
                .get(&key)
                .and_then(|h| h.get(index))
                .is_none_or(|h| h.leds == 0)
        {
            return Err("Save an enabled header before testing".into());
        }
        self.test = Some((key, index, std::time::Instant::now()));
        Ok(())
    }

    /// Apply header placement and colour order before device correction/protocol encoding.
    /// Built-in reports are authoritative; older adapters expose named contiguous zones.
    pub fn apply(
        &self,
        key: &str,
        layout: &freergb_core::LedLayout,
        canvas: &freergb_core::Canvas,
        frame: &mut freergb_core::Frame,
    ) {
        use freergb_core::Rgb;
        let Some(headers) = self.settings.controllers.get(key) else {
            return;
        };
        let report = self.reports.get(key).and_then(|c| c.lock().ok()?.clone());
        let groups = header_ranges(key, layout, headers);
        for (i, header) in headers.iter().enumerate() {
            if header.leds == 0 {
                continue;
            }
            let range = if let Some(r) = &report {
                r.headers.get(i).and_then(|h| h.start.map(|s| (s, h.count)))
            } else {
                groups.get(i).copied().flatten()
            };
            let Some((start, count)) = range else {
                continue;
            };
            if start.saturating_add(count) > frame.len() || count == 0 {
                continue;
            }
            for (n, color) in frame.colors_mut()[start..start + count]
                .iter_mut()
                .enumerate()
            {
                if let Some(r) = header.region {
                    let x = if header.kind == HeaderType::Rgb {
                        0.5
                    } else {
                        (n as f32 + 0.5) / count as f32
                    };
                    *color = canvas.sample(r.x + x * r.w, r.y + 0.5 * r.h);
                }
                if self
                    .test
                    .as_ref()
                    .is_some_and(|(k, h, at)| k == key && *h == i && at.elapsed().as_secs() < 6)
                {
                    let phase = self.test.as_ref().unwrap().2.elapsed().as_secs() as usize / 2;
                    *color = [Rgb::RED, Rgb::GREEN, Rgb::BLUE][phase.min(2)];
                }
                // ASUS already applies this option in its adapter.
                if !key.starts_with("0b05:") {
                    let [r, g, b] = header.order.bytes(*color);
                    *color = Rgb::new(r, g, b);
                }
            }
        }
    }

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
        self.test = None;
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
                    ..Default::default()
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
    fn fixed_msi_zones_and_sparse_fusion_slots_keep_their_identity() {
        use freergb_core::LedLayout;
        let mut layout = LedLayout::strip(4);
        for (led, name) in layout.leds.iter_mut().zip([
            "JRGB1 LED",
            "JRAINBOW1 LED",
            "Onboard LEDs LED 1",
            "Onboard LEDs LED 2",
        ]) {
            led.name = name.into();
        }
        assert_eq!(
            fixed_headers(&layout),
            vec![
                json!({"name":"JRGB1","leds":1,"type":"rgb"}),
                json!({"name":"JRAINBOW1","leds":1,"type":"argb"})
            ]
        );
        let mut layout = LedLayout::strip(2);
        layout.leds[0].name = "D_LED2 LED 1".into();
        layout.leds[1].name = "Onboard channel 2 LED 1".into();
        let mut headers = vec![Header::default(); 6];
        headers[1].leds = 1;
        headers[5].leds = 1;
        let ranges = header_ranges("048d:5711", &layout, &headers);
        assert_eq!(ranges[1], Some((0, 1)));
        assert_eq!(ranges[5], Some((1, 1)));
        assert_eq!(zone_label("Front LED fans LED 2"), "Front LED fans");
    }

    #[test]
    fn six_slots_are_reserved_for_it5711_and_mapping_is_validated() {
        let mut s = Settings::default();
        s.controllers
            .insert("048d:5711".into(), vec![Header::default(); 6]);
        assert!(s.validate().is_ok());
        s.controllers
            .get_mut("048d:5711")
            .unwrap()
            .push(Header::default());
        assert!(s.validate().is_err());
        s.controllers.clear();
        s.controllers
            .insert("0b05:19af".into(), vec![Header::default(); 5]);
        assert!(s.validate().is_err());
        let header = Header {
            leds: 2,
            kind: HeaderType::Rgb,
            ..Default::default()
        };
        s.controllers.insert("0b05:19af".into(), vec![header]);
        assert!(s.validate().is_err());
        let h = &mut s.controllers.get_mut("0b05:19af").unwrap()[0];
        h.leds = 1;
        h.region = Some(freergb_core::Region {
            w: 0.0,
            ..Default::default()
        });
        assert!(s.validate().is_err());
    }

    #[test]
    fn header_mapping_order_and_timed_pattern_use_only_the_selected_zone() {
        use freergb_core::{Canvas, Frame, LedLayout, Region, Rgb};
        let mut board = MainboardRgb::default();
        let mut layout = LedLayout::strip(4);
        for (led, name) in
            layout
                .leds
                .iter_mut()
                .zip(["12V RGB header 1", "Front LED 1", "Front LED 2", "PCH"])
        {
            led.name = name.into();
        }
        let mut settings = Settings::default();
        settings.controllers.insert(
            "26ce:01a2".into(),
            vec![Header {
                leds: 2,
                name: Some("Front".into()),
                order: ColorOrder::Grb,
                region: Some(Region {
                    x: 0.5,
                    w: 0.5,
                    ..Default::default()
                }),
                ..Default::default()
            }],
        );
        board.save(settings).unwrap();
        let mut canvas = Canvas::new(4, 1);
        canvas.paint(|x, _| if x < 0.5 { Rgb::BLUE } else { Rgb::RED });
        let mut frame = Frame::solid(4, Rgb::BLUE);
        board.apply("26ce:01a2", &layout, &canvas, &mut frame);
        assert_eq!(
            frame.colors(),
            &[Rgb::BLUE, Rgb::GREEN, Rgb::GREEN, Rgb::BLUE]
        );
        board.start_test("26ce:01a2".into(), 0).unwrap();
        board.test.as_mut().unwrap().2 =
            std::time::Instant::now() - std::time::Duration::from_secs(2);
        board.apply("26ce:01a2", &layout, &canvas, &mut frame);
        assert_eq!(frame.colors()[1], Rgb::RED); // green test phase after GRB
        board.test.as_mut().unwrap().2 =
            std::time::Instant::now() - std::time::Duration::from_secs(7);
        board.apply("26ce:01a2", &layout, &canvas, &mut frame);
        assert_eq!(frame.colors()[1], Rgb::GREEN); // canvas restored
        board.set_vendor_software(vec!["MSI Center"]);
        assert!(board.start_test("26ce:01a2".into(), 0).is_err());
        assert!(device_key("hid:0b05:19af:serial").is_some());
        assert!(device_key("module:msi-motherboard:1462:7b12:hash").is_some());
        assert!(device_key("hid:0b05:193b:mouse").is_none());
        assert!(device_key("module:msi-keyboard:1462:1234:hash").is_none());
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
