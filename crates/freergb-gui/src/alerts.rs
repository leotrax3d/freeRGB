//! Local advisory monitoring. Never changes hardware or claims a cooling failsafe.
use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, io::Read};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    device: String,
    sensor: String,
    unit: String,
    minimum: f64,
    maximum: f64,
    hysteresis: f64,
}

fn validate(rules: &[Rule]) -> Result<(), String> {
    if rules.len() > 64 {
        return Err("At most 64 sensor rules".into());
    }
    let mut keys = std::collections::BTreeSet::new();
    for r in rules {
        if r.device.is_empty()
            || r.sensor.is_empty()
            || r.device.len() > 512
            || r.sensor.len() > 512
            || r.unit.len() > 32
            || !r.minimum.is_finite()
            || !r.maximum.is_finite()
            || !r.hysteresis.is_finite()
            || r.minimum >= r.maximum
            || r.hysteresis < 0.0
            || r.hysteresis > (r.maximum - r.minimum) / 2.0
            || !keys.insert((&r.device, &r.sensor))
        {
            return Err("Invalid or duplicate sensor rule; check limits and hysteresis".into());
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Condition {
    Healthy,
    Missing,
    Low,
    High,
}
impl Condition {
    fn label(self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Missing => "Missing / stale / disconnected",
            Self::Low => "Below minimum",
            Self::High => "Above maximum",
        }
    }
}

#[derive(Default)]
struct State {
    active: Option<Condition>,
    pending: Option<(Condition, f64)>,
    acknowledged: bool,
}
impl State {
    fn observe(&mut self, rule: &Rule, value: Option<f64>, now: f64) {
        let condition = match value.filter(|v| v.is_finite()) {
            None => Condition::Missing,
            Some(v)
                if v < rule.minimum
                    || (self.active == Some(Condition::Low)
                        && v < rule.minimum + rule.hysteresis) =>
            {
                Condition::Low
            }
            Some(v)
                if v > rule.maximum
                    || (self.active == Some(Condition::High)
                        && v > rule.maximum - rule.hysteresis) =>
            {
                Condition::High
            }
            Some(_) => Condition::Healthy,
        };
        if self.active == Some(condition) {
            self.pending = None;
            return;
        }
        match self.pending {
            Some((candidate, since)) if candidate == condition && now - since >= 3.0 => {
                self.active = Some(condition);
                self.pending = None;
                self.acknowledged = false;
            }
            Some((candidate, _)) if candidate == condition => {}
            _ => self.pending = Some((condition, now)),
        }
    }
}

pub struct Alerts {
    rules: Vec<Rule>,
    states: BTreeMap<(String, String), State>,
    error: Option<String>,
}
impl Default for Alerts {
    fn default() -> Self {
        let loaded = (|| -> Result<Vec<Rule>, String> {
            let Some(path) = super::pages::config_root().map(|p| p.join("sensor-alerts.json"))
            else {
                return Ok(Vec::new());
            };
            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
                Err(e) => return Err(e.to_string()),
            };
            let mut text = String::new();
            file.take(128 * 1024 + 1)
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            if text.len() > 128 * 1024 {
                return Err("Sensor rule file too large".into());
            }
            let rules: Vec<Rule> = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            validate(&rules)?;
            Ok(rules)
        })();
        match loaded {
            Ok(rules) => Self {
                rules,
                states: BTreeMap::new(),
                error: None,
            },
            Err(error) => Self {
                rules: Vec::new(),
                states: BTreeMap::new(),
                error: Some(error),
            },
        }
    }
}
impl Alerts {
    pub fn observe(&mut self, devices: &[Value], connected: bool, now: f64) {
        for rule in &self.rules {
            if validate(std::slice::from_ref(rule)).is_err() {
                self.states
                    .remove(&(rule.device.clone(), rule.sensor.clone()));
                continue;
            }
            let value = devices
                .iter()
                .find(|d| d["info"]["id"].as_str() == Some(&rule.device))
                .filter(|d| connected && d["status"] == "Connected")
                .and_then(|d| d["sensors"].as_array())
                .and_then(|sensors| {
                    sensors
                        .iter()
                        .find(|s| s["id"].as_str() == Some(&rule.sensor))
                })
                .filter(|s| {
                    s["unit"].as_str() == Some(&rule.unit)
                        && s["age_ms"].as_u64().is_some_and(|age| age <= 3000)
                })
                .and_then(|s| s["value"].as_f64());
            self.states
                .entry((rule.device.clone(), rule.sensor.clone()))
                .or_default()
                .observe(rule, value, now);
        }
    }
    pub fn unacknowledged(&self) -> usize {
        self.states
            .values()
            .filter(|s| s.active.is_some_and(|c| c != Condition::Healthy) && !s.acknowledged)
            .count()
    }
    pub fn add_sensor_ui(&mut self, ui: &mut egui::Ui, device: &Value, sensor: &Value) {
        let (Some(id), Some(sensor_id), Some(unit)) = (
            device["info"]["id"].as_str(),
            sensor["id"].as_str(),
            sensor["unit"].as_str(),
        ) else {
            return;
        };
        if self.rules.len() < 64
            && !self
                .rules
                .iter()
                .any(|r| r.device == id && r.sensor == sensor_id)
            && ui.small_button("Configure sensor warning").clicked()
        {
            self.rules.push(Rule {
                device: id.into(),
                sensor: sensor_id.into(),
                unit: unit.into(),
                minimum: 0.0,
                maximum: 10000.0,
                hysteresis: 0.0,
            });
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Sensor warnings", |ui| {
            ui.label("Local warnings while this GUI is open. No hardware protection or fan control. Changes must be saved; identify the sensor before choosing limits.");
            ui.label("Conditions and recovery must persist for 3 seconds. Missing, stale, disconnected or changed-unit readings raise a warning.");
            let mut remove = None;
            for (index, rule) in self.rules.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    ui.label(format!("{} / {} ({})", rule.device, rule.sensor, rule.unit));
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Min"); changed |= ui.add(egui::DragValue::new(&mut rule.minimum)).changed();
                        ui.label("Max"); changed |= ui.add(egui::DragValue::new(&mut rule.maximum)).changed();
                        ui.label("Hysteresis"); changed |= ui.add(egui::DragValue::new(&mut rule.hysteresis)).changed();
                        if ui.small_button("Remove").clicked() { remove = Some(index); }
                    });
                    let key = (rule.device.clone(), rule.sensor.clone());
                    if changed { self.states.remove(&key); }
                    if let Some(state) = self.states.get_mut(&key) {
                        if let Some(condition) = state.active {
                            ui.label(format!("{}{}", condition.label(), if state.acknowledged { " (acknowledged)" } else { "" }));
                            if condition != Condition::Healthy && !state.acknowledged && ui.small_button("Acknowledge").clicked() { state.acknowledged = true; }
                        } else { ui.label("Waiting for stable readings"); }
                    }
                });
            }
            if let Some(index) = remove { let r = self.rules.remove(index); self.states.remove(&(r.device, r.sensor)); }
            if ui.button("Save sensor warning rules").clicked() {
                self.error = (|| -> Result<(), String> {
                    validate(&self.rules)?;
                    let path = super::pages::config_root().ok_or("Configuration directory unavailable")?.join("sensor-alerts.json");
                    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                    let temporary = path.with_extension("json.tmp");
                    std::fs::write(&temporary, serde_json::to_vec_pretty(&self.rules).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                    std::fs::rename(temporary, path).map_err(|e| e.to_string())
                })().err();
            }
            if let Some(error) = &self.error { ui.colored_label(egui::Color32::LIGHT_RED, error); }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn brief_excursions_do_not_accumulate_and_high_recovery_has_hysteresis() {
        let r = rule();
        let mut s = State::default();
        s.observe(&r, Some(1200.0), 0.0);
        s.observe(&r, Some(1200.0), 3.0);
        s.observe(&r, Some(2100.0), 4.0);
        s.observe(&r, Some(1200.0), 6.0);
        s.observe(&r, Some(2100.0), 7.0);
        s.observe(&r, Some(2100.0), 9.0);
        assert_eq!(s.active, Some(Condition::Healthy));
        s.observe(&r, Some(2100.0), 10.0);
        assert_eq!(s.active, Some(Condition::High));
        s.acknowledged = true;
        s.observe(&r, Some(1950.0), 11.0);
        s.observe(&r, Some(1950.0), 20.0);
        assert_eq!(s.active, Some(Condition::High));
        s.observe(&r, Some(1900.0), 21.0);
        s.observe(&r, Some(1900.0), 24.0);
        assert_eq!(s.active, Some(Condition::Healthy));
        s.observe(&r, Some(2100.0), 25.0);
        s.observe(&r, Some(2100.0), 28.0);
        assert_eq!(s.active, Some(Condition::High));
        assert!(!s.acknowledged);
    }
    fn rule() -> Rule {
        Rule {
            device: "d".into(),
            sensor: "s".into(),
            unit: "rpm".into(),
            minimum: 500.0,
            maximum: 2000.0,
            hysteresis: 100.0,
        }
    }
    #[test]
    fn debounce_hysteresis_and_acknowledgement_reset() {
        let r = rule();
        let mut s = State::default();
        s.observe(&r, Some(400.0), 0.0);
        s.observe(&r, Some(400.0), 2.9);
        assert_eq!(s.active, None);
        s.observe(&r, Some(400.0), 3.0);
        assert_eq!(s.active, Some(Condition::Low));
        s.acknowledged = true;
        s.observe(&r, Some(550.0), 4.0);
        assert_eq!(s.active, Some(Condition::Low));
        assert!(s.acknowledged);
        s.observe(&r, Some(600.0), 5.0);
        s.observe(&r, Some(600.0), 8.0);
        assert_eq!(s.active, Some(Condition::Healthy));
        s.observe(&r, None, 9.0);
        s.observe(&r, None, 12.0);
        assert_eq!(s.active, Some(Condition::Missing));
        assert!(!s.acknowledged);
    }
    #[test]
    fn invalid_rules_are_rejected() {
        let r = rule();
        assert!(validate(std::slice::from_ref(&r)).is_ok());
        assert!(validate(&[r.clone(), r]).is_err());
        for (min, max, h) in [
            (f64::NAN, 2000.0, 0.0),
            (500.0, 400.0, 0.0),
            (500.0, 2000.0, 800.0),
            (500.0, 2000.0, -1.0),
        ] {
            let mut r = rule();
            r.minimum = min;
            r.maximum = max;
            r.hysteresis = h;
            assert!(validate(&[r]).is_err());
        }
    }
    #[test]
    fn stale_disconnected_and_changed_units_cannot_look_healthy() {
        let mut a = Alerts {
            rules: vec![rule()],
            states: BTreeMap::new(),
            error: None,
        };
        let d = serde_json::json!({"info":{"id":"d"},"status":"Connected","sensors":[{"id":"s","unit":"rpm","value":1200,"age_ms":0}]});
        a.observe(std::slice::from_ref(&d), true, 0.0);
        a.observe(std::slice::from_ref(&d), true, 3.0);
        assert_eq!(a.unacknowledged(), 0);
        for (time, connected, age, unit) in [
            (4.0, true, 3001, "rpm"),
            (7.0, true, 0, "C"),
            (8.0, false, 0, "rpm"),
        ] {
            let mut d = d.clone();
            d["sensors"][0]["age_ms"] = age.into();
            d["sensors"][0]["unit"] = unit.into();
            a.observe(&[d], connected, time);
        }
        assert_eq!(a.unacknowledged(), 1);
        a.observe(&[], false, 100.0);
        assert_eq!(a.states.len(), 1);
    }
}
