//! Thin RPC-only live cooling editor. Selection is user input; authorization,
//! validation, sampler, curves, calibration and maintenance belong to the daemon.
use super::{Command, Snapshot};
use eframe::egui;
use freergb_control::FanCurve;
use serde_json::json;
use std::sync::mpsc;

/// Display name of a support tier (DECISIONS.md #14).
pub fn tier_label(tier: &str) -> &'static str {
    match tier {
        "verified" => "Verified",
        "lhm_supported" => "LHM-supported",
        "unrestricted" => "Unrestricted",
        "read_only" => "Read-only",
        _ => "Unknown",
    }
}

/// Capability reported by the protected inventory for a channel entry.
pub fn channel_tier(channel: &serde_json::Value) -> &'static str {
    if channel["kind"] != "control" {
        return "";
    }
    if channel["control_available"] != true {
        return "read_only";
    }
    match channel["support"]["tier"].as_str() {
        Some("verified") => "verified",
        Some("lhm_supported") => "lhm_supported",
        _ => "unknown",
    }
}

/// Why the daemon refuses this control, if it does.
pub fn refusal(channel: &serde_json::Value) -> Option<&'static str> {
    if channel["support"]["software_mode"] == true {
        Some("Another program already controls this header (software mode); freeRGB will not take it over.")
    } else if channel["support"]["held_full"] == true {
        Some("Held at 100% after a worker crash. Restart Windows to return it to firmware control.")
    } else {
        None
    }
}

#[derive(Default)]
pub struct MainboardCooling {
    control: String,
    rpm: String,
    temperature: String,
    confirmed: bool,
    confirmation: String,
    manual_duty: u8,
    profile: String,
    profile_name: String,
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
        ui.label("Motherboard fan control. Selecting a channel does not prove its physical header or PWM/DC mode.");
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
                            let tier = channel_tier(channel);
                            let label = if tier.is_empty() {
                                format!("{} · {}", channel["name"].as_str().unwrap_or(kind), id)
                            } else {
                                format!(
                                    "{} · {} · {}",
                                    channel["name"].as_str().unwrap_or(kind),
                                    tier_label(tier),
                                    id
                                )
                            };
                            ui.selectable_value(selected, id.to_owned(), label);
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
        let selected = snapshot.control["cooling_channels"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["kind"] == "control" && c["reading"]["id"] == self.control.as_str());
        let blocked = selected.and_then(refusal);
        if let Some(channel) = selected {
            let tier = channel_tier(channel);
            ui.label(format!("Support: {}", tier_label(tier)));
            let evidence = &snapshot.control["cooling_support"]["channels"][self.control.as_str()];
            ui.label(format!(
                "Channel test: {}",
                evidence["test"].as_str().unwrap_or("none")
            ));
            if tier == "lhm_supported" {
                ui.small("LHM-supported: reduced-speed curves need a passed channel test; calibration (100 → 40%) is available.");
            }
        }
        if let Some(reason) = blocked {
            ui.colored_label(egui::Color32::from_rgb(251, 191, 36), reason);
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
            && blocked.is_none()
            && !busy;
        ui.horizontal(|ui|{
            if ui.add_enabled(ready,egui::Button::new("Start current curve (UAC)")).clicked(){
                let _=tx.send(Command {method:"cooling.sessions.start",params:json!({"mode":"curve","control_id":self.control,"temperature_id":self.temperature,"rpm_id":self.rpm,"confirmed_fan":self.confirmed,"curve":curve})});
            }
            if ui.add_enabled(ready,egui::Button::new("Calibrate fan (UAC)")).clicked(){
                let _=tx.send(Command {method:"cooling.sessions.start",params:json!({"mode":"calibration","control_id":self.control,"temperature_id":self.temperature,"rpm_id":self.rpm,"confirmed_fan":self.confirmed,"critical_c":curve.critical_c})});
            }
            if ui.add_enabled(ready,egui::Button::new("Run channel test (UAC)")).on_hover_text("100 → 70 → 50% (never below 40%), then firmware control").clicked(){
                let _=tx.send(Command {method:"cooling.sessions.start",params:json!({"mode":"channel_test","control_id":self.control,"temperature_id":self.temperature,"rpm_id":self.rpm,"confirmed_fan":self.confirmed,"critical_c":curve.critical_c})});
            }
            ui.add(egui::Slider::new(&mut self.manual_duty, 0..=100).text("Fixed %"));
            if ui.add_enabled(ready,egui::Button::new("Run fixed duty (UAC)")).on_hover_text("Below 40% only on unlocked channels; critical temperature still forces 100%").clicked(){
                let _=tx.send(Command {method:"cooling.sessions.start",params:json!({"mode":"manual","control_id":self.control,"temperature_id":self.temperature,"rpm_id":self.rpm,"confirmed_fan":self.confirmed,"duty_percent":self.manual_duty})});
            }
            if ui.add_enabled(snapshot.connected && busy,egui::Button::new("Stop and return to firmware")).clicked(){
                let _=tx.send(Command {method:"cooling.sessions.stop",params:json!({"job_id":status["job_id"]})});
            }
        });
        let support = &snapshot.control["cooling_support"];
        if !support.is_null() {
            ui.separator();
            ui.colored_label(
                egui::Color32::from_rgb(251, 191, 36),
                "Unrestricted control (untested hardware): no channel test, down to 0% (fan stop), RPM deviations only warn. Always kept: firmware return on stop/crash/telemetry loss/watchdog, 100% at critical temperature, no takeover of foreign software mode.",
            );
            let mut enabled = support["unrestricted_enabled"] == true;
            if ui
                .checkbox(&mut enabled, "Allow unrestricted control (off by default)")
                .changed()
            {
                let _ = tx.send(Command {
                    method: "cooling.support.settings",
                    params: json!({"unrestricted_enabled": enabled}),
                });
            }
            if enabled && !self.control.is_empty() {
                let state = support["channels"][self.control.as_str()]["unlock"]
                    .as_str()
                    .unwrap_or("none");
                ui.label(format!("Selected channel unlock: {state}. Type the channel name to unlock, or 'PUMP <name>' to confirm a pump."));
                ui.text_edit_singleline(&mut self.confirmation);
                ui.horizontal(|ui| {
                    if ui.add_enabled(snapshot.connected && !self.confirmation.trim().is_empty(), egui::Button::new("Unlock channel")).clicked() {
                        let _ = tx.send(Command { method: "cooling.channels.unlock", params: json!({"control_id": self.control, "confirmation": self.confirmation}) });
                    }
                    if ui.add_enabled(snapshot.connected && state == "unlocked", egui::Button::new("Confirm pump control")).clicked() {
                        let _ = tx.send(Command { method: "cooling.channels.confirm_pump", params: json!({"control_id": self.control, "confirmation": self.confirmation}) });
                    }
                    if ui.add_enabled(snapshot.connected && state != "none", egui::Button::new("Revoke")).clicked() {
                        let _ = tx.send(Command { method: "cooling.channels.revoke", params: json!({"control_id": self.control}) });
                    }
                });
            }
        }
        let background = &snapshot.control["cooling_background"];
        if background["available"] == true {
            ui.separator();
            ui.label("Background control: saved curves run in the protected service from Windows startup, without this window.");
            let saved: Vec<serde_json::Value> = background["saved"]["channels"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            for channel in background["service"]["channels"]
                .as_array()
                .into_iter()
                .flatten()
            {
                ui.label(format!(
                    "{} · {} · {}% · {}{}",
                    channel["control_id"].as_str().unwrap_or("?"),
                    channel["state"].as_str().unwrap_or("?"),
                    channel["duty_percent"],
                    tier_label(channel["tier"].as_str().unwrap_or("")),
                    channel["fault"]
                        .as_str()
                        .map(|f| format!(" · {f}"))
                        .unwrap_or_default()
                ));
            }
            let applying = background["apply"]["state"] == "applying";
            ui.horizontal(|ui| {
                let duplicate = saved.iter().any(|c| c["control_id"] == self.control.as_str());
                if ui
                    .add_enabled(ready && !applying && !duplicate && saved.len() < 8, egui::Button::new("Run current curve in background (UAC)"))
                    .clicked()
                {
                    let mut next = saved.clone();
                    next.push(json!({"control_id":self.control,"temperature_id":self.temperature,"rpm_id":self.rpm,"curve":curve,"confirmed_fan":true}));
                    let _ = tx.send(Command { method: "cooling.background.apply", params: json!({"channels": next}) });
                }
                if ui
                    .add_enabled(snapshot.connected && !applying && !saved.is_empty(), egui::Button::new("Disable background control (UAC)"))
                    .clicked()
                {
                    let _ = tx.send(Command { method: "cooling.background.apply", params: json!({"channels": []}) });
                }
            });
            let profiles = &snapshot.control["cooling_profiles"];
            ui.label(format!(
                "Cooling profiles (active: {})",
                profiles["active"].as_str().unwrap_or("none")
            ));
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("cooling-profile")
                    .selected_text(if self.profile.is_empty() {
                        "Profile"
                    } else {
                        self.profile.as_str()
                    })
                    .show_ui(ui, |ui| {
                        for entry in profiles["profiles"].as_array().into_iter().flatten() {
                            if let Some(name) = entry["name"].as_str() {
                                ui.selectable_value(
                                    &mut self.profile,
                                    name.to_owned(),
                                    format!("{name} · {} channels", entry["channels"]),
                                );
                            }
                        }
                    });
                if ui
                    .add_enabled(
                        snapshot.connected && !applying && !self.profile.is_empty(),
                        egui::Button::new("Activate (UAC)"),
                    )
                    .clicked()
                {
                    let _ = tx.send(Command {
                        method: "cooling.profiles.activate",
                        params: json!({"name": self.profile}),
                    });
                }
                if ui
                    .add_enabled(
                        snapshot.connected && !self.profile.is_empty(),
                        egui::Button::new("Delete"),
                    )
                    .clicked()
                {
                    let _ = tx.send(Command {
                        method: "cooling.profiles.delete",
                        params: json!({"name": self.profile}),
                    });
                    self.profile.clear();
                }
            });
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.profile_name);
                let name = self.profile_name.trim().to_owned();
                if ui
                    .add_enabled(
                        snapshot.connected && !name.is_empty(),
                        egui::Button::new("Save current background as profile"),
                    )
                    .clicked()
                {
                    let _ = tx.send(Command {
                        method: "cooling.profiles.save",
                        params: json!({"name": name}),
                    });
                    self.profile = name;
                    self.profile_name.clear();
                }
            });
            if let Some(error) = profiles["error"].as_str() {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            if let Some(error) = background["apply"]["error"].as_str() {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        }
        ui.label(format!(
            "Job {} · {}{}",
            status["job_id"],
            status["state"].as_str().unwrap_or("unknown"),
            status["tier"]
                .as_str()
                .map(|t| format!(" · {}", tier_label(t)))
                .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tiers_and_refusals_follow_the_protected_inventory() {
        let control = |support: serde_json::Value| json!({"kind":"control","control_available":true,"support":support});
        assert_eq!(
            channel_tier(&control(
                json!({"tier":"lhm_supported","software_mode":false,"held_full":false})
            )),
            "lhm_supported"
        );
        assert_eq!(channel_tier(&control(serde_json::Value::Null)), "unknown");
        assert_eq!(
            channel_tier(&json!({"kind":"control","control_available":false})),
            "read_only"
        );
        assert_eq!(channel_tier(&json!({"kind":"fan"})), "");
        assert!(refusal(&control(json!({"tier":"verified","software_mode":true}))).is_some());
        assert!(refusal(&control(json!({"tier":"verified","held_full":true}))).is_some());
        assert!(refusal(&control(json!({"tier":"verified"}))).is_none());
        assert_eq!(tier_label("unrestricted"), "Unrestricted");
    }
}
