//! Minimal Classic editor for composite curves (`cooling.curve_set.*`).
//! The Tauri GUI owns the full editor; this keeps add/remove/parameters and a
//! JSON fallback. Editing never writes hardware: a channel only follows a
//! composite output when a live session or background assignment selects it.
use super::{Command, Snapshot};
use eframe::egui;
use freergb_control::{
    curves::{CurveMixFunction, CurveNode, CurveSet},
    FanCurve,
};
use serde_json::json;
use std::{collections::BTreeMap, sync::mpsc::Sender};

pub struct CompositeCurves {
    text: String,
    loaded: bool,
    sensors: BTreeMap<String, String>,
    outputs: BTreeMap<String, (f64, bool)>,
    error: Option<String>,
}

impl Default for CompositeCurves {
    fn default() -> Self {
        Self {
            text: "{\n  \"nodes\": {}\n}".into(),
            loaded: false,
            sensors: BTreeMap::new(),
            outputs: BTreeMap::new(),
            error: None,
        }
    }
}

/// First free `curve_N` name.
pub fn next_name(set: &CurveSet) -> String {
    (1..=65)
        .map(|i| format!("curve_{i}"))
        .find(|n| !set.nodes.contains_key(n))
        .unwrap_or_else(|| "curve_new".into())
}

/// Simulated outputs from a `cooling.curve_set.values` reply.
pub fn outputs(value: &serde_json::Value) -> BTreeMap<String, (f64, bool)> {
    value["outputs"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| {
            Some((
                k.clone(),
                (
                    v["percent"].as_f64()?,
                    v["failsafe"].as_bool().unwrap_or(true),
                ),
            ))
        })
        .collect()
}

fn pct(v: &mut f32) -> egui::DragValue<'_> {
    egui::DragValue::new(v).range(0.0..=100.0).suffix(" %")
}
fn deg(v: &mut f32) -> egui::DragValue<'_> {
    egui::DragValue::new(v).range(0.0..=110.0).suffix(" °C")
}

impl CompositeCurves {
    fn reload(&mut self, snapshot: &Snapshot) {
        if let Ok(set) = serde_json::from_value::<CurveSet>(snapshot.control["curve_set"].clone()) {
            self.text = serde_json::to_string_pretty(&set).unwrap_or_default();
            self.loaded = true;
        }
    }

    pub fn observe(&mut self, snapshot: &Snapshot) {
        if !self.loaded {
            self.reload(snapshot);
        }
        self.sensors =
            serde_json::from_value(snapshot.control["sensor_sources"].clone()).unwrap_or_default();
        self.outputs = if snapshot.connected {
            outputs(&snapshot.control["curve_set_values"])
        } else {
            BTreeMap::new()
        };
        self.error = serde_json::from_str::<CurveSet>(&self.text)
            .map_err(|e| e.to_string())
            .and_then(|s| s.validate())
            .err();
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        tx: &Sender<Command>,
        snapshot: &Snapshot,
        curve: &FanCurve,
    ) {
        ui.collapsing("Composite curves", |ui| {
            ui.label("Graph, linear, adaptive, constant, trigger, mix and follow nodes. A failed or critical input runs the output at 100 %. Values below are simulated by the daemon; no fan is changed by editing.");
            ui.horizontal(|ui| {
                if ui.button("Reload saved curves").clicked() {
                    self.reload(snapshot);
                }
                if ui
                    .add_enabled(snapshot.connected && self.error.is_none(), egui::Button::new("Save composite curves"))
                    .clicked()
                {
                    if let Ok(set) = serde_json::from_str::<CurveSet>(&self.text) {
                        let _ = tx.send(Command { method: "cooling.curve_set.save", params: json!({ "set": set }) });
                    }
                }
            });
            let Ok(mut set) = serde_json::from_str::<CurveSet>(&self.text) else {
                ui.collapsing("Advanced JSON", |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.text).code_editor().desired_rows(10).desired_width(f32::INFINITY));
                });
                if let Some(error) = &self.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                return;
            };
            let mut changed = false;
            let sensor = self.sensors.keys().next().cloned();
            let names: Vec<String> = set.nodes.keys().cloned().collect();
            ui.add_enabled_ui(set.nodes.len() < 64, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(sensor) = &sensor {
                        if ui.button("Add graph (current curve)").clicked() {
                            set.nodes.insert(next_name(&set), CurveNode::Graph { sensor: sensor.clone(), curve: curve.clone() });
                            changed = true;
                        }
                        if ui.button("Add linear").clicked() {
                            set.nodes.insert(next_name(&set), CurveNode::Linear { sensor: sensor.clone(), min_c: 35.0, max_c: 75.0, min_percent: 40.0, max_percent: 100.0, critical_c: 85.0 });
                            changed = true;
                        }
                        if ui.button("Add adaptive").clicked() {
                            set.nodes.insert(next_name(&set), CurveNode::Equilibrium { sensor: sensor.clone(), target_c: 65.0, critical_c: 85.0, min_percent: 40.0, max_percent: 100.0, gain_percent_per_c: 1.0, max_change_percent_per_s: 2.0, trend_s: 10.0 });
                            changed = true;
                        }
                        if ui.button("Add trigger").clicked() {
                            set.nodes.insert(next_name(&set), CurveNode::Trigger { sensor: sensor.clone(), idle_c: 50.0, load_c: 65.0, idle_percent: 40.0, load_percent: 80.0, critical_c: 85.0, hold_ms: 5000 });
                            changed = true;
                        }
                    } else {
                        ui.label("No temperature source available for sensor nodes.");
                    }
                    if ui.button("Add constant").clicked() {
                        set.nodes.insert(next_name(&set), CurveNode::Constant { percent: 60.0 });
                        changed = true;
                    }
                    if let Some(first) = names.first() {
                        if ui.button("Add mix").clicked() {
                            set.nodes.insert(next_name(&set), CurveNode::Mix { sources: vec![first.clone()], function: CurveMixFunction::Maximum });
                            changed = true;
                        }
                        if ui.button("Add follow").clicked() {
                            set.nodes.insert(next_name(&set), CurveNode::Follow { source: first.clone(), scale: 1.0, offset_percent: 0.0 });
                            changed = true;
                        }
                    }
                });
            });
            let labels = &self.sensors;
            let mut remove = None;
            for (name, node) in &mut set.nodes {
                ui.push_id(name, |ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(name);
                            match self.outputs.get(name) {
                                Some((p, false)) => { ui.label(format!("{p:.0} %")); }
                                Some((p, true)) => { ui.colored_label(egui::Color32::LIGHT_RED, format!("{p:.0} % (fail-safe)")); }
                                None => { ui.weak("not evaluated"); }
                            }
                            if ui.small_button("Remove").clicked() {
                                remove = Some(name.clone());
                            }
                        });
                        let pick_sensor = |ui: &mut egui::Ui, s: &mut String| {
                            let mut edited = false;
                            egui::ComboBox::from_id_salt("sensor").selected_text(labels.get(s.as_str()).map_or(s.as_str(), String::as_str)).show_ui(ui, |ui| {
                                for (key, label) in labels { edited |= ui.selectable_value(s, key.clone(), label).changed(); }
                            });
                            edited
                        };
                        match node {
                            CurveNode::Graph { sensor, curve: c } => {
                                changed |= pick_sensor(ui, sensor);
                                ui.label(format!("{} points, critical {} °C (edit points in JSON or replace with the current curve)", c.points.len(), c.critical_c));
                                if ui.small_button("Use current curve").clicked() { *c = curve.clone(); changed = true; }
                            }
                            CurveNode::Linear { sensor, min_c, max_c, min_percent, max_percent, critical_c } => {
                                changed |= pick_sensor(ui, sensor);
                                ui.horizontal(|ui| { ui.label("From"); changed |= ui.add(deg(min_c)).changed(); changed |= ui.add(pct(min_percent)).changed(); ui.label("to"); changed |= ui.add(deg(max_c)).changed(); changed |= ui.add(pct(max_percent)).changed(); ui.label("critical"); changed |= ui.add(deg(critical_c)).changed(); });
                            }
                            CurveNode::Equilibrium { sensor, target_c, critical_c, min_percent, max_percent, gain_percent_per_c, max_change_percent_per_s, trend_s } => {
                                changed |= pick_sensor(ui, sensor);
                                ui.horizontal(|ui| { ui.label("Hold"); changed |= ui.add(deg(target_c)).changed(); ui.label("between"); changed |= ui.add(pct(min_percent)).changed(); changed |= ui.add(pct(max_percent)).changed(); ui.label("critical"); changed |= ui.add(deg(critical_c)).changed(); });
                                ui.horizontal(|ui| { ui.label("Gain"); changed |= ui.add(egui::DragValue::new(gain_percent_per_c).range(0.1..=10.0).speed(0.1).suffix(" %/°C")).changed(); ui.label("max change"); changed |= ui.add(egui::DragValue::new(max_change_percent_per_s).range(0.1..=20.0).speed(0.1).suffix(" %/s")).changed(); ui.label("trend"); changed |= ui.add(egui::DragValue::new(trend_s).range(0.0..=120.0).suffix(" s")).changed(); });
                            }
                            CurveNode::Constant { percent } => { changed |= ui.add(pct(percent)).changed(); }
                            CurveNode::Trigger { sensor, idle_c, load_c, idle_percent, load_percent, critical_c, hold_ms } => {
                                changed |= pick_sensor(ui, sensor);
                                ui.horizontal(|ui| { ui.label("Idle ≤"); changed |= ui.add(deg(idle_c)).changed(); changed |= ui.add(pct(idle_percent)).changed(); ui.label("load ≥"); changed |= ui.add(deg(load_c)).changed(); changed |= ui.add(pct(load_percent)).changed(); });
                                ui.horizontal(|ui| { ui.label("critical"); changed |= ui.add(deg(critical_c)).changed(); ui.label("hold"); changed |= ui.add(egui::DragValue::new(hold_ms).range(0..=600_000).suffix(" ms")).changed(); });
                            }
                            CurveNode::Mix { sources, function } => {
                                egui::ComboBox::from_id_salt("function").selected_text(format!("{function:?}")).show_ui(ui, |ui| {
                                    for f in [CurveMixFunction::Maximum, CurveMixFunction::Minimum, CurveMixFunction::Average, CurveMixFunction::Sum] { changed |= ui.selectable_value(function, f, format!("{f:?}")).changed(); }
                                });
                                for candidate in names.iter().filter(|n| *n != name) {
                                    let mut on = sources.contains(candidate);
                                    if ui.checkbox(&mut on, candidate).changed() { if on { sources.push(candidate.clone()); } else { sources.retain(|s| s != candidate); } changed = true; }
                                }
                            }
                            CurveNode::Follow { source, scale, offset_percent } => {
                                egui::ComboBox::from_id_salt("source").selected_text(source.as_str()).show_ui(ui, |ui| {
                                    for candidate in names.iter().filter(|n| *n != name) { changed |= ui.selectable_value(source, candidate.clone(), candidate).changed(); }
                                });
                                ui.horizontal(|ui| { ui.label("× "); changed |= ui.add(egui::DragValue::new(scale).range(0.0..=4.0).speed(0.05)).changed(); ui.label("+"); changed |= ui.add(egui::DragValue::new(offset_percent).range(-100.0..=100.0).suffix(" %")).changed(); });
                            }
                        }
                    });
                });
            }
            if let Some(name) = remove { set.nodes.remove(&name); changed = true; }
            if changed { self.text = serde_json::to_string_pretty(&set).unwrap_or_default(); }
            ui.collapsing("Advanced JSON", |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.text).code_editor().desired_rows(10).desired_width(f32::INFINITY));
            });
            if let Some(error) = &self.error { ui.colored_label(egui::Color32::LIGHT_RED, error); }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_outputs_parse_daemon_replies() {
        let mut set = CurveSet::default();
        assert_eq!(next_name(&set), "curve_1");
        set.nodes
            .insert("curve_1".into(), CurveNode::Constant { percent: 50.0 });
        assert_eq!(next_name(&set), "curve_2");
        let out = outputs(
            &json!({"outputs":{"a":{"percent":55.0,"failsafe":false},"b":{"percent":100.0}}}),
        );
        assert_eq!(out["a"], (55.0, false));
        assert_eq!(
            out["b"],
            (100.0, true),
            "missing failsafe flag reads as fail-safe"
        );
    }
}
