use eframe::egui;
use freergb_control::{MacroDefinition, MacroStep};
use freergb_core::Rgb;
use freergb_effects::EffectSpec;

pub fn edit(ui: &mut egui::Ui, source: &mut String) {
    let Ok(mut definition) = serde_json::from_str::<MacroDefinition>(source) else {
        ui.label("Fix the definition in Advanced JSON to use the visual editor.");
        advanced(ui, source);
        return;
    };
    let mut changed = false;
    let mut action = None;
    ui.horizontal(|ui| {
        ui.label("Repeat");
        changed |= ui
            .add(egui::DragValue::new(&mut definition.repeat).range(1..=20))
            .changed();
    });
    let count = definition.steps.len();
    for (index, step) in definition.steps.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("Step {}", index + 1));
                    if ui.add_enabled(index > 0, egui::Button::new("Up")).clicked() {
                        action = Some((index, 0));
                    }
                    if ui
                        .add_enabled(index + 1 < count, egui::Button::new("Down"))
                        .clicked()
                    {
                        action = Some((index, 1));
                    }
                    if ui.small_button("Delete").clicked() {
                        action = Some((index, 2));
                    }
                });
                match step {
                    MacroStep::Delay { ms } => {
                        ui.label("Delay");
                        changed |= ui
                            .add(egui::Slider::new(ms, 0..=10_000).text("ms"))
                            .changed();
                    }
                    MacroStep::Color { color } => {
                        changed |= super::color_edit(ui, "Color", color);
                    }
                    MacroStep::Brightness { value } => {
                        changed |= ui
                            .add(egui::Slider::new(value, 0.0..=1.0).text("Brightness"))
                            .changed();
                    }
                    MacroStep::Text { text } => {
                        ui.label("Type text into foreground application");
                        changed |= ui.text_edit_multiline(text).changed();
                    }
                    MacroStep::KeyDown { key } | MacroStep::KeyUp { key } => {
                        ui.label("Recorded key press/release");
                        changed |= ui.text_edit_singleline(key).changed();
                    }
                    MacroStep::Key { key } => {
                        ui.label("Key");
                        egui::ComboBox::from_id_salt("key")
                            .selected_text(key.as_str())
                            .show_ui(ui, |ui| {
                                for name in [
                                    "enter",
                                    "escape",
                                    "tab",
                                    "space",
                                    "backspace",
                                    "up",
                                    "down",
                                    "left",
                                    "right",
                                    "media_play_pause",
                                    "volume_up",
                                    "volume_down",
                                    "volume_mute",
                                ] {
                                    changed |=
                                        ui.selectable_value(key, name.into(), name).changed();
                                }
                            });
                    }
                    MacroStep::Chord { keys } => {
                        ui.label("Shortcut modifiers");
                        ui.horizontal(|ui| {
                            for name in ["ctrl", "alt", "shift", "meta"] {
                                let mut enabled = keys.iter().any(|k| k == name);
                                if ui.checkbox(&mut enabled, name).changed() {
                                    keys.retain(|k| k != name);
                                    if enabled {
                                        keys.push(name.into());
                                    }
                                    changed = true;
                                }
                            }
                        });
                        let mut main = keys
                            .iter()
                            .find(|k| !["ctrl", "alt", "shift", "meta"].contains(&k.as_str()))
                            .cloned()
                            .unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label("Key (a-z, 0-9, enter, tab, ...)");
                            if ui.text_edit_singleline(&mut main).changed() {
                                keys.retain(|k| {
                                    ["ctrl", "alt", "shift", "meta"].contains(&k.as_str())
                                });
                                if !main.is_empty() {
                                    keys.push(main.to_lowercase());
                                }
                                changed = true;
                            }
                        });
                    }
                    MacroStep::Effect { effect } => {
                        let name = effect.name();
                        egui::ComboBox::from_id_salt("effect")
                            .selected_text(super::label(name))
                            .show_ui(ui, |ui| {
                                for candidate in EffectSpec::NAMES {
                                    if ui
                                        .selectable_label(
                                            candidate == name,
                                            super::label(candidate),
                                        )
                                        .clicked()
                                    {
                                        *effect = EffectSpec::default_for(candidate).unwrap();
                                        changed = true;
                                    }
                                }
                            });
                        changed |= super::effect_params(ui, effect);
                    }
                }
            });
        });
    }
    if let Some((index, operation)) = action {
        match operation {
            0 => definition.steps.swap(index, index - 1),
            1 => definition.steps.swap(index, index + 1),
            _ => {
                definition.steps.remove(index);
            }
        }
        changed = true;
    }
    ui.add_enabled_ui(definition.steps.len() < 128, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, step) in [
                ("+ Delay", MacroStep::Delay { ms: 1000 }),
                ("+ Color", MacroStep::Color { color: Rgb::RED }),
                ("+ Brightness", MacroStep::Brightness { value: 1.0 }),
                (
                    "+ Effect",
                    MacroStep::Effect {
                        effect: EffectSpec::default_for("rainbow_wave").unwrap(),
                    },
                ),
                (
                    "+ Text",
                    MacroStep::Text {
                        text: "Text".into(),
                    },
                ),
                (
                    "+ Key",
                    MacroStep::Key {
                        key: "enter".into(),
                    },
                ),
                (
                    "+ Shortcut",
                    MacroStep::Chord {
                        keys: vec!["ctrl".into(), "c".into()],
                    },
                ),
            ] {
                if ui.button(label).clicked() {
                    definition.steps.push(step);
                    changed = true;
                }
            }
        });
    });
    if changed {
        if let Ok(json) = serde_json::to_string_pretty(&definition) {
            *source = json;
        }
    }
    advanced(ui, source);
}

fn advanced(ui: &mut egui::Ui, source: &mut String) {
    egui::CollapsingHeader::new("Advanced JSON").show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(source)
                .code_editor()
                .desired_rows(10)
                .desired_width(f32::INFINITY),
        );
    });
}
