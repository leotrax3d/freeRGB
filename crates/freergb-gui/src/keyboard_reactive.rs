//! Thin editor for daemon-owned keyboard reactions. No hooks or LED mapping here.
use super::{Command, Snapshot};
use eframe::egui;
use freergb_effects::keyboard::ReactiveConfig;
use serde_json::json;

#[derive(Default)]
pub struct KeyboardPanel {
    config: Option<ReactiveConfig>,
    dirty: bool,
}
impl KeyboardPanel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        tx: &std::sync::mpsc::Sender<Command>,
    ) {
        let state = &snapshot.control["keyboard_reactive"];
        let Ok(saved) = serde_json::from_value::<ReactiveConfig>(state["config"].clone()) else {
            ui.label("Keyboard reactions require an updated daemon.");
            return;
        };
        if !self.dirty || self.config.as_ref() == Some(&saved) {
            self.config = Some(saved.clone());
            self.dirty = false;
        }
        let config = self.config.get_or_insert_with(|| saved.clone());
        ui.label("Pressed keys glow briefly over the current effect, then fade back.");
        self.dirty |= ui
            .checkbox(&mut config.enabled, "Enable keyboard reactions")
            .changed();
        ui.small("Listens to physical key positions from all local keyboards; only selected RGB keyboards react. No typed text or input history is saved.");
        let mut color = config.color.to_array();
        if ui.color_edit_button_srgb(&mut color).changed() {
            config.color = freergb_core::Rgb::new(color[0], color[1], color[2]);
            self.dirty = true;
        }
        self.dirty |= ui
            .add(egui::Slider::new(&mut config.fade_ms, 50..=5000).text("Fade (ms)"))
            .changed();
        self.dirty |= ui
            .checkbox(&mut config.iso, "ISO keyboard (large Enter key)")
            .changed();
        if let Some(devices) = state["devices"].as_array() {
            if devices.is_empty() {
                ui.label("No supported keyboard connected.");
            }
            for device in devices {
                let id = device["id"].as_str().unwrap_or_default();
                let mapped = device["keys"].as_object().map_or(0, |keys| keys.len());
                let mut selected = config.devices.contains(id);
                if ui
                    .add_enabled(
                        mapped > 0,
                        egui::Checkbox::new(
                            &mut selected,
                            format!(
                                "{} ({mapped} mapped keys)",
                                device["name"].as_str().unwrap_or("Keyboard")
                            ),
                        ),
                    )
                    .changed()
                {
                    if selected {
                        config.devices.insert(id.into());
                    } else {
                        config.devices.remove(id);
                    }
                    self.dirty = true;
                }
            }
        }
        ui.label(format!(
            "Capture: {}",
            state["status"]["state"].as_str().unwrap_or("unavailable")
        ));
        if let Some(error) = state["status"]["error"].as_str() {
            ui.colored_label(egui::Color32::YELLOW, error);
        }
        let validation = config.validate();
        if let Err(error) = &validation {
            ui.colored_label(egui::Color32::YELLOW, error);
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(snapshot.connected, egui::Button::new("Stop reactions now"))
                .clicked()
            {
                let _ = tx.send(Command {
                    method: "keyboard.reactive.disable",
                    params: json!({}),
                });
                self.dirty = false;
            }
            if ui
                .add_enabled(
                    snapshot.connected && validation.is_ok(),
                    egui::Button::new("Apply keyboard settings"),
                )
                .clicked()
            {
                let _ = tx.send(Command {
                    method: "keyboard.reactive.configure",
                    params: json!({"config":config}),
                });
            }
            if ui
                .add_enabled(
                    snapshot.connected && !self.dirty && !config.devices.is_empty(),
                    egui::Button::new("Preview A key"),
                )
                .clicked()
            {
                let _ = tx.send(Command {
                    method: "keyboard.reactive.preview",
                    params: json!({"usage":4}),
                });
            }
            if self.dirty && ui.button("Discard edits").clicked() {
                *config = saved;
                self.dirty = false;
            }
        });
    }
}
