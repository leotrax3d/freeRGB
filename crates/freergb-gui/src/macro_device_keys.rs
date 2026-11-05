use super::{Command, Snapshot};
use eframe::egui;
use serde_json::json;
use std::sync::mpsc::Sender;

#[derive(Default)]
pub struct DeviceKeys {
    device: String,
    usage: u16,
}
impl DeviceKeys {
    pub fn ui(&mut self, ui: &mut egui::Ui, tx: &Sender<Command>, snapshot: &Snapshot, name: &str) {
        let state = &snapshot.control["macro_device_keys"];
        let send = |method, params| {
            let _ = tx.send(Command { method, params });
        };
        ui.collapsing("Device macro keys", |ui| {
            ui.label("Mapped F6–F10 only. Typing/modifier keys and cancellation F11 are reserved.");
            ui.label("Physical source identity is pending owner verification; anonymous events never trigger.");
            egui::ComboBox::from_id_salt("macro-key-device").selected_text(if self.device.is_empty() {"Choose device"} else {&self.device}).show_ui(ui,|ui| {
                for device in state["devices"].as_array().into_iter().flatten() {
                    if let Some(id) = device["id"].as_str() {ui.selectable_value(&mut self.device,id.to_owned(),device["name"].as_str().unwrap_or(id));}
                }
            });
            if self.usage == 0 {self.usage = 63;}
            egui::ComboBox::from_id_salt("macro-device-key").selected_text(format!("F{}",self.usage-57)).show_ui(ui,|ui| {
                for usage in 63..=67 {ui.selectable_value(&mut self.usage,usage,format!("F{}",usage-57));}
            });
            let bindings = state["bindings"].as_array().cloned().unwrap_or_default();
            if ui.add_enabled(!self.device.is_empty(),egui::Button::new(format!("Assign current macro: {name}"))).clicked() {
                let mut next = bindings.clone(); next.push(json!({"device_id":self.device,"usage":self.usage,"macro_name":name}));
                send("macros.device_keys.save",json!({"bindings":next}));
            }
            for (index,binding) in bindings.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{} · usage {} → {}",binding["device_id"].as_str().unwrap_or("?"),binding["usage"],binding["macro_name"].as_str().unwrap_or("?")));
                    if ui.small_button("Remove").clicked() {
                        let mut next = bindings.clone(); next.remove(index); send("macros.device_keys.save",json!({"bindings":next}));
                    }
                });
            }
            let mut enabled = state["enabled"] == true;
            if ui.checkbox(&mut enabled,"Enable device macro keys for this daemon session").changed() {send("macros.device_keys.enable",json!({"enabled":enabled}));}
        });
    }
}
