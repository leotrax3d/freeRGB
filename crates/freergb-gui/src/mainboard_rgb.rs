//! RPC-only motherboard RGB settings: global switch, LED count and name per
//! ARGB header. Validation and device handling live in the daemon
//! (`mainboard_rgb.get`/`mainboard_rgb.save`, docs/MAINBOARD_RGB.md).
use super::{Command, Snapshot};
use eframe::egui;
use serde_json::{json, Map, Value};

/// Driven controller families: (vid, pids, ARGB headers, label).
const FAMILIES: &[(u64, &[u64], usize, &str)] = &[
    (
        0x0b05,
        &[0x18f3, 0x1939, 0x19af, 0x1aa6, 0x1bed],
        4,
        "ASUS Aura",
    ),
    (0x048d, &[0x5702, 0x8297], 2, "Gigabyte RGB Fusion 2"),
    (
        0x048d,
        &[0x5711],
        6,
        "Gigabyte IT5711 (4 external + 2 onboard)",
    ),
    (0x048d, &[0x8950], 2, "Gigabyte RGB Fusion 2"),
    (0x0414, &[0xa100], 2, "Gigabyte RGB Fusion 2"),
    (0x26ce, &[0x01a2, 0x01a6], 3, "ASRock Polychrome"),
];

/// Controllers to show: detected motherboard devices plus saved keys, sorted.
pub fn controllers(
    devices: &[Value],
    saved: &Map<String, Value>,
) -> Vec<(String, &'static str, usize, bool)> {
    let mut out: Vec<(String, &'static str, usize, bool)> = Vec::new();
    for device in devices {
        let (Some(vid), Some(pid)) = (
            device["info"]["usb_vid"].as_u64(),
            device["info"]["usb_pid"].as_u64(),
        ) else {
            continue;
        };
        if device["info"]["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("module:msi-motherboard:"))
        {
            let key = format!("{vid:04x}:{pid:04x}");
            if !out.iter().any(|c| c.0 == key) {
                out.push((key, "MSI Mystic Light", 4, true));
            }
            continue;
        }
        if let Some((_, _, headers, label)) = FAMILIES
            .iter()
            .find(|(v, p, _, _)| *v == vid && p.contains(&pid))
        {
            let key = format!("{vid:04x}:{pid:04x}");
            if !out.iter().any(|c| c.0 == key) {
                out.push((key, label, *headers, true));
            }
        }
    }
    for key in saved.keys() {
        if out.iter().any(|c| &c.0 == key) {
            continue;
        }
        let ids: Vec<u64> = key
            .split(':')
            .filter_map(|x| u64::from_str_radix(x, 16).ok())
            .collect();
        let family = FAMILIES
            .iter()
            .find(|(v, p, _, _)| ids.len() == 2 && *v == ids[0] && p.contains(&ids[1]));
        out.push((
            key.clone(),
            family.map_or(
                if key.starts_with("1462:") || key.starts_with("0db0:") {
                    "MSI Mystic Light"
                } else {
                    "Controller"
                },
                |f| f.3,
            ),
            family.map_or(4, |f| f.2),
            false,
        ));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Sets one header in `settings`, padding the list; counts clamped, blank names dropped.
pub fn set_header(settings: &mut Value, key: &str, index: usize, leds: u16, name: &str, max: u16) {
    if !settings["controllers"].is_object() {
        settings["controllers"] = json!({});
    }
    let list = settings["controllers"]
        .as_object_mut()
        .expect("object")
        .entry(key.to_owned())
        .or_insert_with(|| json!([]));
    if !list.is_array() {
        *list = json!([]);
    }
    let list = list.as_array_mut().expect("array");
    while list.len() <= index {
        list.push(json!({"leds": 0}));
    }
    let name: String = name.trim().chars().take(40).collect();
    let mut header = json!({"leds": leds.min(max)});
    if !name.is_empty() {
        header["name"] = json!(name);
    }
    // Keep the colour order and fallback of the header being edited.
    for field in ["order", "fallback", "type", "region"] {
        if let Some(value) = list[index].get(field) {
            header[field] = value.clone();
        }
    }
    list[index] = header;
}

pub const ORDERS: [&str; 6] = ["rgb", "rbg", "grb", "gbr", "brg", "bgr"];

/// Sets colour order (`rgb` = default) and release fallback of an existing
/// header: `spectrum` (default), `off`, or `#rrggbb` for a static colour.
/// Defaults are removed so saved files stay minimal.
pub fn set_header_options(
    settings: &mut Value,
    key: &str,
    index: usize,
    order: &str,
    fallback: &str,
) {
    let Some(header) = settings["controllers"][key]
        .get_mut(index)
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    if order == "rgb" || !ORDERS.contains(&order) {
        header.remove("order");
    } else {
        header.insert("order".into(), json!(order));
    }
    match fallback {
        "off" => header.insert("fallback".into(), json!({"mode": "off"})),
        c if c.starts_with('#') && c.len() == 7 => {
            header.insert("fallback".into(), json!({"mode": "static", "color": c}))
        }
        _ => header.remove("fallback"),
    };
}

#[derive(Default)]
pub struct MainboardRgbPanel {
    draft: Option<Value>,
    dirty: bool,
    fans: u16,
}

impl MainboardRgbPanel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        tx: &std::sync::mpsc::Sender<Command>,
    ) {
        let state = &snapshot.control["mainboard_rgb"];
        if !state["settings"].is_object() {
            ui.label("Motherboard RGB settings require an updated daemon.");
            return;
        }
        if !self.dirty {
            self.draft = Some(state["settings"].clone());
        }
        let max = state["max_leds"].as_u64().unwrap_or(512) as u16;
        let presets: Vec<u16> = state["fan_presets"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64())
                    .map(|v| v as u16)
                    .collect()
            })
            .unwrap_or_default();
        self.fans = self.fans.max(1);
        let draft = self.draft.get_or_insert_with(|| state["settings"].clone());
        if let Some(controllers) = state["fixed_headers"].as_object() {
            for (key, fixed) in controllers {
                if let Some(headers) = fixed.as_array() {
                    for (i, h) in headers.iter().enumerate() {
                        let current = &draft["controllers"][key][i];
                        if current["leds"] != h["leds"] || current["type"] != h["type"] {
                            let name = current["name"]
                                .as_str()
                                .or_else(|| h["name"].as_str())
                                .unwrap_or("")
                                .to_owned();
                            set_header(
                                draft,
                                key,
                                i,
                                h["leds"].as_u64().unwrap_or(1) as u16,
                                &name,
                                max,
                            );
                            draft["controllers"][key][i]["type"] = h["type"].clone();
                            self.dirty = true;
                        }
                    }
                }
            }
        }
        let mut enabled = draft["enabled"].as_bool().unwrap_or(true);
        if ui
            .checkbox(&mut enabled, "Manage motherboard RGB (ARGB/RGB headers)")
            .changed()
        {
            draft["enabled"] = json!(enabled);
            self.dirty = true;
        }
        if let Some(vendors) = state["vendor_software"]
            .as_array()
            .filter(|v| !v.is_empty())
        {
            let names: Vec<&str> = vendors.iter().filter_map(Value::as_str).collect();
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("{} is running and controls the motherboard lighting; freeRGB leaves it alone until it is closed.", names.join(", ")),
            );
        }
        if let Some(error) = state["error"].as_str() {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if enabled {
            let saved = draft["controllers"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            let list = controllers(&snapshot.devices, &saved);
            if list.is_empty() {
                ui.label("No supported motherboard RGB controller was found (ASUS Aura USB, Gigabyte RGB Fusion 2 USB, ASRock Polychrome USB, MSI Mystic Light).");
            }
            for (key, label, headers, detected) in list {
                ui.group(|ui| {
                    ui.label(format!(
                        "{label} · {key}{}",
                        if detected { "" } else { " · not detected" }
                    ));
                    let fixed = state["fixed_headers"][&key].as_array();
                    for index in 0..fixed.map_or(headers, Vec::len) {
                        let header = &draft["controllers"][&key][index];
                        let mut kind = header["type"].as_str().unwrap_or("argb").to_owned();
                        let mut region = header["region"].clone();
                        if !region.is_object() {
                            region = json!({"x":0.0,"y":0.0,"w":1.0,"h":1.0});
                        }
                        if let Some(fixed) = fixed.and_then(|list| list.get(index)) {
                            kind = fixed["type"].as_str().unwrap_or("argb").to_owned();
                        }
                        let mut mapping_changed = false;
                        let mut leds = header["leds"].as_u64().unwrap_or(0) as u16;
                        if let Some(fixed) = fixed.and_then(|list| list.get(index)) {
                            leds = fixed["leds"].as_u64().unwrap_or(1) as u16;
                        }
                        let mut name = header["name"]
                            .as_str()
                            .or_else(|| {
                                fixed
                                    .and_then(|list| list.get(index))
                                    .and_then(|h| h["name"].as_str())
                            })
                            .unwrap_or("")
                            .to_owned();
                        let mut order = header["order"].as_str().unwrap_or("rgb").to_owned();
                        let mut fallback = match header["fallback"]["mode"].as_str() {
                            Some("off") => "off".to_owned(),
                            Some("static") => header["fallback"]["color"]
                                .as_str()
                                .unwrap_or("#ffffff")
                                .to_owned(),
                            _ => "spectrum".to_owned(),
                        };
                        let mut options_changed = false;
                        let mut changed = false;
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("Header {}", index + 1));
                            egui::ComboBox::from_id_salt(("type", &key, index))
                                .selected_text(&kind)
                                .show_ui(ui, |ui| {
                                    if fixed.is_some() {
                                        ui.disable();
                                    }
                                    for (value, label) in
                                        [("argb", "5 V ARGB"), ("rgb", "12 V RGB")]
                                    {
                                        mapping_changed |= ui
                                            .selectable_value(&mut kind, value.to_owned(), label)
                                            .changed();
                                    }
                                });
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut name)
                                        .hint_text(format!("ARGB header {}", index + 1))
                                        .desired_width(140.0)
                                        .char_limit(40),
                                )
                                .changed();
                            changed |= ui
                                .add_enabled(
                                    fixed.is_none(),
                                    egui::DragValue::new(&mut leds)
                                        .range(0..=max)
                                        .suffix(" LEDs"),
                                )
                                .changed();
                            ui.small("Fan preset:");
                            for per in &presets {
                                if ui
                                    .add_enabled(
                                        fixed.is_none(),
                                        egui::Button::new(format!(
                                            "{}{per}",
                                            if self.fans > 1 {
                                                format!("{}×", self.fans)
                                            } else {
                                                String::new()
                                            }
                                        )),
                                    )
                                    .clicked()
                                {
                                    leds = per.saturating_mul(self.fans);
                                    changed = true;
                                }
                            }
                            {
                                egui::ComboBox::from_id_salt(("order", &key, index))
                                    .selected_text(order.to_uppercase())
                                    .show_ui(ui, |ui| {
                                        for o in ORDERS {
                                            options_changed |= ui
                                                .selectable_value(
                                                    &mut order,
                                                    o.to_owned(),
                                                    o.to_uppercase(),
                                                )
                                                .changed();
                                        }
                                    });
                                if key.starts_with("0b05:") {
                                    let kind = match fallback.as_str() {
                                        "spectrum" | "off" => fallback.clone(),
                                        _ => "static".to_owned(),
                                    };
                                    egui::ComboBox::from_id_salt(("fallback", &key, index))
                                        .selected_text(match kind.as_str() {
                                            "off" => "Off on stop",
                                            "static" => "Static on stop",
                                            _ => "Spectrum on stop",
                                        })
                                        .show_ui(ui, |ui| {
                                            for (value, text) in [
                                                ("spectrum", "Firmware spectrum"),
                                                ("off", "Off"),
                                                ("static", "Static colour"),
                                            ] {
                                                if ui
                                                    .selectable_label(kind == value, text)
                                                    .clicked()
                                                    && kind != value
                                                {
                                                    fallback = if value == "static" {
                                                        "#ffffff".to_owned()
                                                    } else {
                                                        value.to_owned()
                                                    };
                                                    options_changed = true;
                                                }
                                            }
                                        });
                                    if kind == "static" {
                                        let mut rgb = [255u8; 3];
                                        for (i, v) in rgb.iter_mut().enumerate() {
                                            *v = u8::from_str_radix(
                                                fallback.get(1 + i * 2..3 + i * 2).unwrap_or("ff"),
                                                16,
                                            )
                                            .unwrap_or(255);
                                        }
                                        if ui.color_edit_button_srgb(&mut rgb).changed() {
                                            fallback = format!(
                                                "#{:02x}{:02x}{:02x}",
                                                rgb[0], rgb[1], rgb[2]
                                            );
                                            options_changed = true;
                                        }
                                    }
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Canvas (0-1)");
                            for field in ["x", "y", "w", "h"] {
                                let mut v = region[field].as_f64().unwrap_or(0.0);
                                ui.label(field.to_uppercase());
                                if ui
                                    .add(egui::DragValue::new(&mut v).speed(0.01).range(
                                        if field == "w" || field == "h" {
                                            0.01..=1.0
                                        } else {
                                            0.0..=1.0
                                        },
                                    ))
                                    .changed()
                                {
                                    region[field] = json!(v);
                                    mapping_changed = true;
                                }
                            }
                            let conflict = state["vendor_software"]
                                .as_array()
                                .is_some_and(|v| !v.is_empty());
                            if ui
                                .add_enabled(
                                    !self.dirty
                                        && !changed
                                        && !options_changed
                                        && !mapping_changed
                                        && detected
                                        && leds > 0
                                        && !conflict,
                                    egui::Button::new("Test RGB (6 s)"),
                                )
                                .clicked()
                            {
                                let _ = tx.send(Command {
                                    method: "mainboard_rgb.test",
                                    params: json!({"key": key, "index": index}),
                                });
                            }
                        });
                        if kind == "rgb" {
                            leds = leds.min(1);
                        }
                        if changed || options_changed || mapping_changed {
                            set_header(draft, &key, index, leds, &name, max);
                            if options_changed {
                                set_header_options(draft, &key, index, &order, &fallback);
                            }
                            draft["controllers"][&key][index]["type"] = json!(kind);
                            draft["controllers"][&key][index]["region"] = region;
                            self.dirty = true;
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Fans chained per header for presets");
                ui.add(egui::DragValue::new(&mut self.fans).range(1..=8));
            });
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.dirty, egui::Button::new("Save"))
                .clicked()
            {
                let _ = tx.send(Command {
                    method: "mainboard_rgb.save",
                    params: json!({"settings": draft}),
                });
                self.dirty = false;
            }
            if self.dirty && ui.button("Discard").clicked() {
                self.dirty = false;
            }
        });
        ui.small("Save before testing: red, green and blue for two seconds each, then effects resume. Canvas rectangles sample the shared effect. Type does not change voltage. MSI fixed zones retain reported LED counts.");
        ui.small("Headers with 0 LEDs keep their firmware effect. Lighting returns to the firmware effect when freeRGB stops.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_detected_and_saved_controllers_and_edits_headers() {
        let devices = vec![
            json!({"info": {"usb_vid": 0x0b05, "usb_pid": 0x19af}}),
            json!({"info": {"usb_vid": 0x1b1c, "usb_pid": 0x1bfd}}),
        ];
        let mut settings = json!({"enabled": true, "controllers": {"048d:5702": [{"leds": 16}]}});
        let list = controllers(&devices, settings["controllers"].as_object().unwrap());
        assert_eq!(
            list,
            vec![
                ("048d:5702".to_owned(), "Gigabyte RGB Fusion 2", 2, false),
                ("0b05:19af".to_owned(), "ASUS Aura", 4, true),
            ]
        );
        set_header(&mut settings, "0b05:19af", 2, 700, "  Front fans ", 512);
        assert_eq!(
            settings["controllers"]["0b05:19af"],
            json!([{"leds": 0}, {"leds": 0}, {"leds": 512, "name": "Front fans"}])
        );
        set_header(&mut settings, "0b05:19af", 2, 8, " ", 512);
        assert_eq!(settings["controllers"]["0b05:19af"][2], json!({"leds": 8}));
    }

    #[test]
    fn header_options_are_kept_across_edits_and_defaults_dropped() {
        let mut s = json!({"enabled": true, "controllers": {"0b05:19af": [{"leds": 8}]}});
        set_header_options(&mut s, "0b05:19af", 0, "grb", "#ff8800");
        assert_eq!(
            s["controllers"]["0b05:19af"][0],
            json!({"leds": 8, "order": "grb", "fallback": {"mode": "static", "color": "#ff8800"}})
        );
        set_header(&mut s, "0b05:19af", 0, 12, "Fans", 512);
        assert_eq!(s["controllers"]["0b05:19af"][0]["order"], "grb");
        set_header_options(&mut s, "0b05:19af", 0, "rgb", "spectrum");
        assert_eq!(
            s["controllers"]["0b05:19af"][0],
            json!({"leds": 12, "name": "Fans"})
        );
        set_header_options(&mut s, "0b05:19af", 0, "xyz", "off");
        assert_eq!(
            s["controllers"]["0b05:19af"][0]["fallback"],
            json!({"mode": "off"})
        );
        assert!(s["controllers"]["0b05:19af"][0].get("order").is_none());
    }
}
