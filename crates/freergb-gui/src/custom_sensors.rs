use super::{Command, Snapshot};
use eframe::egui;
use freergb_control::{
    sensors::{MixFunction, Reading, SensorGraph, SensorNode},
    FanCurve,
};
use serde_json::json;
use std::{collections::BTreeMap, sync::mpsc::Sender};

pub struct CustomSensors {
    text: String,
    loaded: bool,
    labels: BTreeMap<String, String>,
    readings: BTreeMap<String, Reading>,
    error: Option<String>,
}
impl Default for CustomSensors {
    fn default() -> Self {
        Self {
            text: "{\n  \"nodes\": {}\n}".into(),
            loaded: false,
            labels: BTreeMap::new(),
            readings: BTreeMap::new(),
            error: None,
        }
    }
}
impl CustomSensors {
    fn reload(&mut self, snapshot: &Snapshot) {
        if let Ok(graph) =
            serde_json::from_value::<SensorGraph>(snapshot.control["sensor_graph"].clone())
        {
            self.text = serde_json::to_string_pretty(&graph).unwrap_or_default();
            self.loaded = true;
        }
    }
    pub fn observe(&mut self, snapshot: &Snapshot) {
        if !self.loaded {
            self.reload(snapshot);
        }
        self.labels =
            serde_json::from_value(snapshot.control["sensor_sources"].clone()).unwrap_or_default();
        self.readings = if snapshot.connected {
            serde_json::from_value(snapshot.control["sensor_values"]["values"].clone())
                .unwrap_or_default()
        } else {
            BTreeMap::new()
        };
        self.error = serde_json::from_str::<SensorGraph>(&self.text)
            .map_err(|e| e.to_string())
            .and_then(|g| g.validate())
            .err();
    }
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        tx: &Sender<Command>,
        snapshot: &Snapshot,
        _curve: &FanCurve,
    ) {
        ui.collapsing("Custom temperature sources",|ui| {
            ui.label("Edit daemon-evaluated Celsius sources. Displayed values use the saved graph. Missing/stale inputs stay unavailable. No fan or pump commands are sent.");
            ui.horizontal(|ui| {
                if ui.button("Reload saved sources").clicked(){self.reload(snapshot);}
                if ui.add_enabled(snapshot.connected&&self.error.is_none(),egui::Button::new("Save source graph")).clicked(){
                    if let Ok(graph)=serde_json::from_str::<SensorGraph>(&self.text){let _=tx.send(Command{method:"cooling.sensors.save",params:json!({"graph":graph})});}
                }
            });
            if let Ok(mut graph)=serde_json::from_str::<SensorGraph>(&self.text) {
                let mut changed=false;
                let names:Vec<_>=graph.nodes.keys().cloned().collect();
                let next_name=|graph:&SensorGraph| (1..=129).map(|i|format!("sensor_{i}")).find(|n|!graph.nodes.contains_key(n)).unwrap();
                ui.add_enabled_ui(graph.nodes.len()<128,|ui| {
                    ui.collapsing("Add a cached input",|ui| {for (key,label) in &self.labels {if ui.button(label).clicked(){graph.nodes.insert(next_name(&graph),SensorNode::Input{source:key.clone()});changed=true;}} if self.labels.is_empty(){ui.label("No fresh cached temperature source available.");}});
                    if let Some(first)=names.first(){ui.horizontal(|ui| {
                        if ui.button("Add offset").clicked(){graph.nodes.insert(next_name(&graph),SensorNode::Offset{source:first.clone(),amount:0.0,proportional:false});changed=true;}
                        if ui.button("Add mix").clicked(){graph.nodes.insert(next_name(&graph),SensorNode::Mix{sources:vec![first.clone()],function:MixFunction::Maximum});changed=true;}
                        if ui.button("Add time average").clicked(){graph.nodes.insert(next_name(&graph),SensorNode::TimeAverage{source:first.clone(),window_ms:10_000});changed=true;}
                    });}
                });
                let mut remove=None;
                for (name,node) in &mut graph.nodes {
                    ui.push_id(name,|ui|{ui.group(|ui| {
                        ui.horizontal(|ui|{ui.strong(name);if ui.small_button("Remove").clicked(){remove=Some(name.clone());}});
                        let pick=|ui:&mut egui::Ui,source:&mut String| {let mut edited=false;egui::ComboBox::from_id_salt("source").selected_text(source.as_str()).show_ui(ui,|ui|{for candidate in names.iter().filter(|n|*n!=name){edited|=ui.selectable_value(source,candidate.clone(),candidate).changed();}});edited};
                        match node {
                            SensorNode::Input{source}=>{ui.label(self.labels.get(source).unwrap_or(source));}
                            SensorNode::Offset{source,amount,proportional}=>{changed|=pick(ui,source);changed|=ui.add(egui::DragValue::new(amount).range(-200.0..=200.0)).changed();changed|=ui.checkbox(proportional,"Proportional %").changed();}
                            SensorNode::TimeAverage{source,window_ms}=>{changed|=pick(ui,source);changed|=ui.add(egui::DragValue::new(window_ms).range(1000..=600_000).suffix(" ms")).changed();}
                            SensorNode::Mix{sources,function}=>{
                                egui::ComboBox::from_id_salt("function").selected_text(format!("{function:?}")).show_ui(ui,|ui|{for choice in [MixFunction::Maximum,MixFunction::Minimum,MixFunction::Average,MixFunction::Sum,MixFunction::Subtract]{changed|=ui.selectable_value(function,choice,format!("{choice:?}")).changed();}});
                                for candidate in names.iter().filter(|n|*n!=name){let mut selected=sources.contains(candidate);if ui.checkbox(&mut selected,candidate).changed(){if selected{sources.push(candidate.clone());}else{sources.retain(|s|s!=candidate);}changed=true;}}
                                ui.small("Subtract uses the displayed source order; edit JSON to reorder.");ui.label(sources.join(" → "));
                            }
                        }
                        if let Some(reading)=self.readings.get(name){match reading.value {Some(value)=>{ui.label(format!("{value:.1} °C"));},None=>{ui.colored_label(egui::Color32::LIGHT_RED,reading.error.as_deref().unwrap_or("Unavailable"));}}}
                    });});
                }
                if let Some(name)=remove{graph.nodes.remove(&name);changed=true;}
                if changed {self.text=serde_json::to_string_pretty(&graph).unwrap_or_default();}

            }
            ui.collapsing("Advanced source JSON",|ui|{ui.add(egui::TextEdit::multiline(&mut self.text).code_editor().desired_rows(10).desired_width(f32::INFINITY));});
            if let Some(error)=&self.error{ui.colored_label(egui::Color32::LIGHT_RED,error);}
        });
    }
}
