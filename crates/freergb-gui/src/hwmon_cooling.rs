//! RPC-only Linux hwmon fan control: per header firmware / fixed duty / curve
//! on any hwmon temperature. Safety limits, capture/restore and fault handling
//! live in the daemon (`cooling.hwmon.*`, docs/COOLING_LINUX_HWMON.md).
use super::{Command, Snapshot};
use eframe::egui;
use freergb_control::FanCurve;
use serde_json::{json, Value};

#[derive(Default)]
pub struct HwmonPanel {
    /// Per channel: (mode index 0 firmware/1 fixed/2 curve, percent, sensor id, pump).
    drafts: std::collections::BTreeMap<String, (usize, f32, String, bool)>,
}

/// `(sensor id, "chip: label (value)")` for every hwmon temperature.
pub fn temperatures(view: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for chip in view["chips"].as_array().into_iter().flatten() {
        let id = chip["id"].as_str().unwrap_or_default();
        let name = chip["name"].as_str().unwrap_or("hwmon");
        for t in chip["temps"].as_array().into_iter().flatten() {
            let index = t["index"].as_u64().unwrap_or(0);
            let value = t["celsius"]
                .as_f64()
                .map_or("–".to_owned(), |c| format!("{c:.0} °C"));
            out.push((
                format!("{id}/temp{index}"),
                format!("{name}: {} ({value})", t["label"].as_str().unwrap_or("?")),
            ));
        }
    }
    out
}

/// Request for one header, or `None` for "return to firmware".
pub fn request(
    channel: &str,
    mode: usize,
    percent: f32,
    sensor: &str,
    pump: bool,
    curve: &FanCurve,
) -> Option<Value> {
    match mode {
        1 => {
            Some(json!({"channel_id": channel, "mode": "manual", "percent": percent, "pump": pump}))
        }
        2 if !sensor.is_empty() => Some(
            json!({"channel_id": channel, "mode": "curve", "curve": curve, "sensor_id": sensor, "pump": pump}),
        ),
        _ => None,
    }
}

impl HwmonPanel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        tx: &std::sync::mpsc::Sender<Command>,
        curve: &FanCurve,
    ) {
        let view = &snapshot.control["hwmon"];
        if view["started"] != true {
            ui.label("Linux hwmon provider is not running (Linux only; started unless --no-hwmon/--no-hardware).");
            return;
        }
        if let Some(board) = view["board"].as_object() {
            ui.label(format!(
                "Board: {} {} (BIOS {})",
                board
                    .get("manufacturer")
                    .and_then(Value::as_str)
                    .unwrap_or("?"),
                board.get("product").and_then(Value::as_str).unwrap_or("?"),
                board
                    .get("bios_version")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
            ));
        }
        let temps = temperatures(view);
        let assigned: std::collections::BTreeMap<String, &Value> = view["assignments"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| Some((a["channel_id"].as_str()?.to_owned(), a)))
            .collect();
        let status: std::collections::BTreeMap<String, &Value> = view["status"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| Some((s["channel_id"].as_str()?.to_owned(), s)))
            .collect();
        let mut headers = 0;
        for chip in view["chips"].as_array().into_iter().flatten() {
            let pwms = chip["pwms"].as_array().cloned().unwrap_or_default();
            if pwms.is_empty() {
                continue;
            }
            ui.separator();
            ui.strong(chip["name"].as_str().unwrap_or("hwmon"));
            for pwm in pwms {
                headers += 1;
                let Some(id) = pwm["channel_id"].as_str().map(str::to_owned) else {
                    continue;
                };
                let fan = pwm["paired_fan"].as_u64().and_then(|i| {
                    chip["fans"]
                        .as_array()?
                        .iter()
                        .find(|f| f["index"].as_u64() == Some(i))?["rpm"]
                        .as_f64()
                });
                let current = assigned.get(&id);
                let draft = self
                    .drafts
                    .entry(id.clone())
                    .or_insert_with(|| match current {
                        Some(a) if a["mode"] == "curve" => (
                            2,
                            50.0,
                            a["sensor_id"].as_str().unwrap_or_default().to_owned(),
                            a["pump"] == true,
                        ),
                        Some(a) => (
                            1,
                            a["percent"].as_f64().unwrap_or(50.0) as f32,
                            String::new(),
                            a["pump"] == true,
                        ),
                        None => (
                            0,
                            60.0,
                            temps.first().map(|t| t.0.clone()).unwrap_or_default(),
                            false,
                        ),
                    });
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "{}: {}%{}",
                        pwm["label"].as_str().unwrap_or("PWM"),
                        pwm["percent"]
                            .as_f64()
                            .map_or("–".into(), |p| format!("{p:.0}")),
                        fan.map_or(String::new(), |r| format!(", {r:.0} RPM"))
                    ));
                    if pwm["writable"] != true {
                        ui.small("read-only (permissions; see packaging/linux/60-freergb.rules)");
                        return;
                    }
                    egui::ComboBox::from_id_salt(("hwmon-mode", &id))
                        .selected_text(["Firmware", "Fixed", "Curve"][draft.0])
                        .show_ui(ui, |ui| {
                            for (i, name) in ["Firmware", "Fixed", "Curve"].iter().enumerate() {
                                ui.selectable_value(&mut draft.0, i, *name);
                            }
                        });
                    if draft.0 == 1 {
                        ui.add(egui::Slider::new(&mut draft.1, 0.0..=100.0).suffix("%"));
                    }
                    if draft.0 == 2 {
                        let selected = temps
                            .iter()
                            .find(|t| t.0 == draft.2)
                            .map_or("Select sensor", |t| t.1.as_str());
                        egui::ComboBox::from_id_salt(("hwmon-temp", &id))
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                for (tid, label) in &temps {
                                    ui.selectable_value(&mut draft.2, tid.clone(), label);
                                }
                            });
                    }
                    ui.checkbox(&mut draft.3, "Pump").on_hover_text(
                        "Pumps stay at 100% unless unrestricted control is enabled.",
                    );
                    if ui
                        .button("Apply")
                        .on_hover_text("Curve mode uses the curve from the editor above.")
                        .clicked()
                    {
                        let command = match request(&id, draft.0, draft.1, &draft.2, draft.3, curve)
                        {
                            Some(params) => Command {
                                method: "cooling.hwmon.set",
                                params,
                            },
                            None => Command {
                                method: "cooling.hwmon.release",
                                params: json!({"channel_id": id}),
                            },
                        };
                        let _ = tx.send(command);
                    }
                    if let Some(s) = status.get(&id) {
                        let text = format!(
                            "{}{}",
                            s["state"].as_str().unwrap_or(""),
                            s["message"]
                                .as_str()
                                .map_or(String::new(), |m| format!(": {m}"))
                        );
                        if s["state"] == "faulted" {
                            ui.colored_label(egui::Color32::LIGHT_RED, text);
                        } else {
                            ui.small(text);
                        }
                    }
                });
            }
        }
        if headers == 0 {
            ui.label("No fan headers found. Load the motherboard's SuperIO driver (e.g. `sudo modprobe nct6775` or `it87`); see docs/COOLING_LINUX_HWMON.md.");
        } else if ui.button("Return all headers to firmware").clicked() {
            let _ = tx.send(Command {
                method: "cooling.hwmon.release",
                params: json!({}),
            });
            self.drafts.clear();
        }
        ui.small(format!(
            "Fans keep a {}% minimum unless unrestricted control is on; missing temperatures or the critical temperature drive 100%. Headers return to firmware when freeRGB stops.",
            view["floor_percent"].as_f64().unwrap_or(40.0)
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_requests_and_lists_temperatures() {
        let view = json!({"chips":[{"id":"nct6798@x","name":"nct6798","temps":[{"index":1,"label":"CPUTIN","celsius":41.0}]}]});
        assert_eq!(
            temperatures(&view),
            vec![(
                "nct6798@x/temp1".to_owned(),
                "nct6798: CPUTIN (41 °C)".to_owned()
            )]
        );
        let curve = FanCurve::default();
        assert!(request("c/pwm1", 0, 50.0, "", false, &curve).is_none());
        assert_eq!(
            request("c/pwm1", 1, 55.0, "", true, &curve).unwrap()["percent"],
            55.0
        );
        assert!(request("c/pwm1", 2, 0.0, "", false, &curve).is_none());
        assert_eq!(
            request("c/pwm1", 2, 0.0, "c/temp1", false, &curve).unwrap()["mode"],
            "curve"
        );
    }
}
