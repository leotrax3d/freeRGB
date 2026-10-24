//! RPC-only channel assignment editor. Validation and resolution live outside egui.
use super::{Command, Snapshot};
use eframe::egui;
use freergb_control::channels::{ChannelBinding, ChannelBindings, ChannelRole, ElectricalMode};
use serde_json::json;

#[derive(Default)]
pub struct BindingsPanel {
    config: Option<ChannelBindings>,
    dirty: bool,
}
impl BindingsPanel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        tx: &std::sync::mpsc::Sender<Command>,
    ) {
        let state = &snapshot.control["channel_bindings"];
        let Ok(saved) = serde_json::from_value::<ChannelBindings>(state["config"].clone()) else {
            ui.label("Channel assignments require an updated daemon.");
            return;
        };
        if !self.dirty || self.config.as_ref() == Some(&saved) {
            self.config = Some(saved.clone());
            self.dirty = false;
        }
        let config = self.config.get_or_insert_with(|| saved.clone());
        ui.label("Name motherboard outputs and choose their RPM sensor explicitly.");
        ui.small("Assignments and reported PWM/DC modes are user annotations. They do not change hardware settings or establish that wiring has been verified. RGB/ARGB uses a separate connection.");
        if let Some(error) = state["error"].as_str() {
            ui.colored_label(egui::Color32::YELLOW, error);
        }
        let channels = snapshot.control["cooling_channels"].as_array();
        let mut remove = None;
        for (index, binding) in config.bindings.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                ui.group(|ui| {
                    ui.label(&binding.device_id);
                    ui.small(format!("Control: {}", binding.control_id));
                    ui.horizontal(|ui| {
                        ui.label("Header name");
                        self.dirty |= ui.text_edit_singleline(&mut binding.header).changed();
                        if ui.small_button("Remove assignment").clicked() {
                            remove = Some(index);
                        }
                    });
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("role")
                            .selected_text(format!("{:?}", binding.role))
                            .show_ui(ui, |ui| {
                                for role in [ChannelRole::Fan, ChannelRole::Pump] {
                                    self.dirty |= ui
                                        .selectable_value(
                                            &mut binding.role,
                                            role,
                                            format!("{role:?}"),
                                        )
                                        .changed();
                                }
                            });
                        ui.label("Reported electrical mode");
                        egui::ComboBox::from_id_salt("mode")
                            .selected_text(format!("{:?}", binding.electrical_mode))
                            .show_ui(ui, |ui| {
                                for mode in [
                                    ElectricalMode::Unknown,
                                    ElectricalMode::Pwm,
                                    ElectricalMode::Dc,
                                ] {
                                    self.dirty |= ui
                                        .selectable_value(
                                            &mut binding.electrical_mode,
                                            mode,
                                            format!("{mode:?}"),
                                        )
                                        .changed();
                                }
                            });
                    });
                    egui::ComboBox::from_id_salt("tachometer")
                        .selected_text(
                            binding
                                .tachometer_id
                                .as_deref()
                                .unwrap_or("No RPM sensor assigned"),
                        )
                        .show_ui(ui, |ui| {
                            self.dirty |= ui
                                .selectable_value(
                                    &mut binding.tachometer_id,
                                    None,
                                    "No RPM sensor assigned",
                                )
                                .changed();
                            for channel in channels.into_iter().flatten().filter(|c| {
                                c["kind"] == "fan"
                                    && c["provider"] == binding.provider
                                    && c["device_id"] == binding.device_id
                            }) {
                                if let Some(id) = channel["id"].as_str() {
                                    let label = format!(
                                        "{} · {} RPM · {}",
                                        channel["name"].as_str().unwrap_or("Sensor"),
                                        channel["reading"]["value"],
                                        id
                                    );
                                    self.dirty |= ui
                                        .selectable_value(
                                            &mut binding.tachometer_id,
                                            Some(id.into()),
                                            label,
                                        )
                                        .changed();
                                }
                            }
                        });
                    if let Some(assignment) = state["assignments"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|a| a["control_id"] == binding.control_id)
                    {
                        ui.small(format!(
                            "Saved status: {} · hardware unverified",
                            assignment["state"].as_str().unwrap_or("unavailable")
                        ));
                    }
                });
            });
        }
        if let Some(index) = remove {
            config.bindings.remove(index);
            self.dirty = true;
        }
        for channel in channels
            .into_iter()
            .flatten()
            .filter(|c| c["kind"] == "control" && c["control_available"] == true)
        {
            let Some(id) = channel["id"].as_str() else {
                continue;
            };
            if config.bindings.iter().any(|b| b.control_id == id) {
                continue;
            }
            if ui
                .add_enabled(
                    config.bindings.len() < 128,
                    egui::Button::new(format!(
                        "Assign {} · {}",
                        channel["name"].as_str().unwrap_or("Control"),
                        channel["device_id"].as_str().unwrap_or("")
                    )),
                )
                .clicked()
            {
                config.bindings.push(ChannelBinding {
                    provider: channel["provider"].as_str().unwrap_or_default().into(),
                    device_id: channel["device_id"].as_str().unwrap_or_default().into(),
                    control_id: id.into(),
                    tachometer_id: None,
                    header: String::new(),
                    role: ChannelRole::Fan,
                    electrical_mode: ElectricalMode::Unknown,
                });
                self.dirty = true;
            }
        }
        if channels.is_none_or(|c| c.is_empty()) {
            ui.label("No motherboard channels available; saved annotations remain editable.");
        }
        let validation = config.validate();
        if let Err(error) = &validation {
            ui.colored_label(egui::Color32::YELLOW, error);
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    snapshot.connected && self.dirty && validation.is_ok(),
                    egui::Button::new("Save channel assignments"),
                )
                .clicked()
            {
                let _ = tx.send(Command {
                    method: "cooling.bindings.save",
                    params: json!({"config":config}),
                });
            }
            if self.dirty && ui.button("Discard edits").clicked() {
                *config = saved;
                self.dirty = false;
            }
        });
    }
}
