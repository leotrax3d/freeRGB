//! Explicit focus-scoped capture; no native/global hook is installed here.
use super::{Command, Snapshot};
use eframe::egui;
use serde_json::json;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Recording {
    session: Option<String>,
    heartbeat: Option<Instant>,
    text_mode: bool,
    text: String,
    modifiers: egui::Modifiers,
    started: Option<Instant>,
}

impl Recording {
    fn id() -> egui::Id {
        egui::Id::new("macro-recording-pad")
    }
    fn send(&self, tx: &Sender<Command>, event: serde_json::Value) {
        if let Some(session) = &self.session {
            let _ = tx.send(Command {
                method: "macros.record.event",
                params: json!({"session":session,"event":event}),
            });
        }
    }
    fn stop(&mut self, tx: &Sender<Command>, discard: bool) {
        if !discard && self.text_mode && !self.text.is_empty() {
            self.send(tx, json!({"type":"text","text":self.text}));
        }
        if let Some(session) = self.session.take() {
            let _ = tx.send(Command {
                method: "macros.record.stop",
                params: json!({"session":session,"discard":discard}),
            });
        }
        self.text.clear();
        self.modifiers = Default::default();
    }
    pub fn poll(
        &mut self,
        ctx: &egui::Context,
        tx: &Sender<Command>,
        snapshot: &Snapshot,
        page: u8,
    ) {
        if self.session.is_none() {
            return;
        }
        if self
            .started
            .is_some_and(|t| t.elapsed() >= Duration::from_secs(30))
        {
            self.stop(tx, true);
            return;
        }
        if self
            .started
            .is_some_and(|t| t.elapsed() >= Duration::from_secs(3))
            && snapshot.control["macro_status"]["recording"] != true
        {
            self.stop(tx, true);
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(100));
        if page != 2
            || !snapshot.connected
            || !ctx.input(|i| i.focused)
            || !ctx.memory(|m| m.has_focus(Self::id()))
        {
            self.stop(tx, true);
            return;
        }
        if self
            .heartbeat
            .is_none_or(|t| t.elapsed() >= Duration::from_millis(500))
        {
            self.send(tx, json!({"type":"heartbeat"}));
            self.heartbeat = Some(Instant::now());
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            self.stop(tx, false);
            return;
        }
        if self.text_mode {
            return;
        }
        let modifiers = ctx.input(|i| i.modifiers);
        for (name, previous, current) in [
            ("ctrl", self.modifiers.ctrl, modifiers.ctrl),
            ("alt", self.modifiers.alt, modifiers.alt),
            ("shift", self.modifiers.shift, modifiers.shift),
            ("meta", self.modifiers.mac_cmd, modifiers.mac_cmd),
        ] {
            if previous != current {
                self.send(
                    tx,
                    json!({"type":if current {"key_down"} else {"key_up"},"key":name}),
                );
            }
        }
        self.modifiers = modifiers;
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Key {
                key,
                pressed,
                repeat: false,
                ..
            } = event
            {
                let key = match key {
                    egui::Key::ArrowUp => "up".into(),
                    egui::Key::ArrowDown => "down".into(),
                    egui::Key::ArrowLeft => "left".into(),
                    egui::Key::ArrowRight => "right".into(),
                    _ => format!("{key:?}").trim_start_matches("Num").to_lowercase(),
                };
                if freergb_control::macro_key(&key) {
                    self.send(
                        tx,
                        json!({"type":if pressed {"key_down"} else {"key_up"},"key":key}),
                    );
                }
            }
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, tx: &Sender<Command>) {
        ui.group(|ui| {
            ui.label("Focused-pad recording, 30 s maximum. Focus loss discards the buffer. Save explicitly after reviewing.");
            if self.session.is_none() {
                ui.checkbox(&mut self.text_mode,"Record text as one step when stopped");
                if ui.button("Record").clicked() {
                    let session = format!("classic-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
                    let _ = tx.send(Command {method:"macros.record.start",params:json!({"session":session})});
                    self.session = Some(session); self.heartbeat = None; self.started = Some(Instant::now());
                    ui.memory_mut(|m|m.request_focus(Self::id()));
                }
            }
            if self.session.is_some() {
                ui.colored_label(egui::Color32::LIGHT_RED,"● RECORDING (unsaved)");
                if self.text_mode {
                    ui.add(egui::TextEdit::multiline(&mut self.text).id(Self::id()));
                } else {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(),40.0),egui::Sense::hover());
                    let response = ui.interact(rect,Self::id(),egui::Sense::click());
                    ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,"Recording pad — type here; F12 stops and reviews",egui::FontId::default(),ui.visuals().text_color());
                    if response.clicked() { ui.memory_mut(|m|m.request_focus(Self::id())); }
                    ui.memory_mut(|m|m.set_focus_lock_filter(Self::id(),egui::EventFilter {tab:true,horizontal_arrows:true,vertical_arrows:true,escape:true}));
                }
                ui.horizontal(|ui| {
                    if ui.button("Stop and review").clicked() { self.stop(tx,false); }
                    if ui.button("Discard recording").clicked() { self.stop(tx,true); }
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leaving_the_macro_page_or_pad_discards_without_saving() {
        for page in [0, 2] {
            let mut recording = Recording {
                session: Some("fixture".into()),
                ..Default::default()
            };
            let ctx = egui::Context::default();
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("password-or-other-field")));
            let (tx, rx) = std::sync::mpsc::channel();
            let snapshot = Snapshot {
                connected: true,
                ..Default::default()
            };
            recording.poll(&ctx, &tx, &snapshot, page);
            let command = rx.try_recv().unwrap();
            assert_eq!(command.method, "macros.record.stop");
            assert_eq!(command.params["discard"], true);
            assert!(rx.try_recv().is_err());
            assert!(recording.session.is_none());
        }
    }
}
