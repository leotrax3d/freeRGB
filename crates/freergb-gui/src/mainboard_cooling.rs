//! Thin RPC-only live cooling editor. Selection is user input; authorization,
//! validation, sampler, curves, calibration and maintenance belong to the daemon.
use super::{Command, Snapshot};
use eframe::egui;
use freergb_control::FanCurve;
use serde_json::json;
use std::sync::mpsc;

#[derive(Default)]
pub struct MainboardCooling {
    control: String,
    rpm: String,
    temperature: String,
    confirmed: bool,
}
impl MainboardCooling {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        tx: &mpsc::Sender<Command>,
        curve: &FanCurve,
    ) {
        let status = &snapshot.control["cooling_session"];
        ui.label("Experimental motherboard fan control. Selecting a channel does not prove its physical header or PWM/DC mode.");
        if status["available"] != true {
            ui.label("Install/enable the optional mainboard provider, then restart through the launcher. Live control requires the current protected service.");
            return;
        }
        let previous = (
            self.control.clone(),
            self.rpm.clone(),
            self.temperature.clone(),
        );
        for (kind, label, selected) in [
            ("control", "Fan output", &mut self.control),
            ("fan", "Matching fan RPM", &mut self.rpm),
        ] {
            egui::ComboBox::from_id_salt(format!("live-{kind}"))
                .selected_text(if selected.is_empty() {
                    label
                } else {
                    selected.as_str()
                })
                .show_ui(ui, |ui| {
                    for channel in snapshot.control["cooling_channels"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|c| c["kind"] == kind)
                    {
                        if let Some(id) = channel["reading"]["id"].as_str() {
                            ui.selectable_value(
                                selected,
                                id.to_owned(),
                                format!("{} · {}", channel["name"].as_str().unwrap_or(kind), id),
                            );
                        }
                    }
                });
        }
        egui::ComboBox::from_id_salt("live-temperature")
            .selected_text(if self.temperature.is_empty() {
                "Temperature source"
            } else {
                &self.temperature
            })
            .show_ui(ui, |ui| {
                for entry in snapshot.control["monitoring_sensors"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|s| s["provider"] == "mainboard.lhm" && s["sensor"]["unit"] == "°C")
                {
                    if let Some(id) = entry["sensor"]["id"].as_str() {
                        ui.selectable_value(
                            &mut self.temperature,
                            id.to_owned(),
                            format!(
                                "{} · {}",
                                entry["sensor"]["name"].as_str().unwrap_or("Temperature"),
                                id
                            ),
                        );
                    }
                }
            });
        if previous
            != (
                self.control.clone(),
                self.rpm.clone(),
                self.temperature.clone(),
            )
        {
            self.confirmed = false;
        }
        ui.checkbox(
            &mut self.confirmed,
            "I confirm this is a normal fan, not a pump, and the selected RPM belongs to it.",
        );
        ui.small("Starting requests Windows UAC for this selected fan. Native curves require minimum ≥40% and critical temperature ≤100°C.");
        let busy = ["authorizing", "executing", "stopping"]
            .contains(&status["state"].as_str().unwrap_or(""));
        let ready = snapshot.connected
            && self.confirmed
            && !self.control.is_empty()
            && !self.rpm.is_empty()
            && !self.temperature.is_empty()
            && status["state"] != "blocked"
            && !busy;
        ui.horizontal(|ui|{
            if ui.add_enabled(ready,egui::Button::new("Start current curve (UAC)")).clicked(){
                let _=tx.send(Command {method:"cooling.sessions.start",params:json!({"mode":"curve","control_id":self.control,"temperature_id":self.temperature,"rpm_id":self.rpm,"confirmed_fan":self.confirmed,"curve":curve})});
            }
            if ui.add_enabled(ready,egui::Button::new("Calibrate fan (UAC)")).clicked(){
                let _=tx.send(Command {method:"cooling.sessions.start",params:json!({"mode":"calibration","control_id":self.control,"temperature_id":self.temperature,"rpm_id":self.rpm,"confirmed_fan":self.confirmed,"critical_c":curve.critical_c})});
            }
            if ui.add_enabled(snapshot.connected && busy,egui::Button::new("Stop and return to firmware")).clicked(){
                let _=tx.send(Command {method:"cooling.sessions.stop",params:json!({"job_id":status["job_id"]})});
            }
        });
        ui.label(format!(
            "Job {} · {}",
            status["job_id"],
            status["state"].as_str().unwrap_or("unknown")
        ));
        if let Some(error) = status["error"].as_str() {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if !status["report"].is_null() {
            ui.collapsing("Last execution / calibration result", |ui| {
                ui.monospace(serde_json::to_string_pretty(&status["report"]).unwrap_or_default());
            });
        }
        ui.small("Closing the GUI leaves the daemon running. Use Stop to return this fan to firmware control.");
    }
}
