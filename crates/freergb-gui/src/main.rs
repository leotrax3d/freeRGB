//! freeRGB GUI. Holds no device logic: every action is a JSON-RPC call to
//! freergbd, made on a worker thread so the UI never blocks. While idle the
//! window repaints only when new state arrives (about once per second).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use serde_json::{json, Value};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Latest state fetched from the daemon.
#[derive(Default)]
struct Snapshot {
    connected: bool,
    error: Option<String>,
    devices: Vec<Value>,
    diagnostics: Option<Value>,
    effect: Option<Value>,
    /// `None` when the daemon runs without profile storage (`--no-state`).
    profiles: Option<Vec<String>>,
    /// Increments with every successful fetch.
    generation: u64,
}

/// A call queued by the UI.
struct Command {
    method: &'static str,
    params: Value,
}

fn worker(rx: Receiver<Command>, state: Arc<Mutex<Snapshot>>, ctx: egui::Context) {
    loop {
        let mut last_error = None;
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(cmd) => {
                // Coalesce bursts (e.g. slider drags): keep only the newest
                // command per method.
                let mut pending = vec![cmd];
                while let Ok(c) = rx.try_recv() {
                    pending.retain(|p| p.method != c.method);
                    pending.push(c);
                }
                for c in pending {
                    if let Err(e) = freergb_client::call(c.method, c.params) {
                        last_error = Some(e.to_string());
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        let fetched = (|| -> Result<_, freergb_client::ClientError> {
            Ok((
                freergb_client::call("devices.list", Value::Null)?,
                freergb_client::call("diagnostics.get", Value::Null)?,
                freergb_client::call("effects.get", Value::Null)?,
            ))
        })();
        let profiles = freergb_client::call("profiles.list", Value::Null)
            .ok()
            .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok());
        {
            let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
            match fetched {
                Ok((devices, diag, effect)) => {
                    s.connected = true;
                    s.devices = devices.as_array().cloned().unwrap_or_default();
                    s.diagnostics = Some(diag);
                    s.effect = Some(effect);
                    s.profiles = profiles;
                    s.generation += 1;
                    s.error = last_error;
                }
                Err(e) => {
                    let generation = s.generation;
                    *s = Snapshot {
                        error: Some(e.to_string()),
                        generation,
                        ..Snapshot::default()
                    };
                }
            }
        }
        ctx.request_repaint();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EffectKind {
    Solid,
    RainbowWave,
    Breathing,
}

impl EffectKind {
    const ALL: [EffectKind; 3] = [
        EffectKind::Solid,
        EffectKind::RainbowWave,
        EffectKind::Breathing,
    ];

    fn label(self) -> &'static str {
        match self {
            EffectKind::Solid => "Solid colour",
            EffectKind::RainbowWave => "Rainbow wave",
            EffectKind::Breathing => "Breathing",
        }
    }
}

struct App {
    state: Arc<Mutex<Snapshot>>,
    tx: Sender<Command>,
    /// Controls are (re)loaded from the first snapshot newer than this.
    sync_after_generation: Option<u64>,
    profile_name: String,
    selected_profile: Option<String>,
    kind: EffectKind,
    color: [u8; 3],
    speed: f32,
    scale: f32,
    period_ms: u32,
    brightness: f32,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = Arc::new(Mutex::new(Snapshot::default()));
        let (tx, rx) = mpsc::channel();
        let (s, ctx) = (state.clone(), cc.egui_ctx.clone());
        std::thread::Builder::new()
            .name("api-worker".into())
            .spawn(move || worker(rx, s, ctx))
            .expect("spawn worker thread");
        Self {
            state,
            tx,
            sync_after_generation: Some(0),
            profile_name: String::new(),
            selected_profile: None,
            kind: EffectKind::RainbowWave,
            color: [0, 155, 222],
            speed: 0.2,
            scale: 1.0,
            period_ms: 3000,
            brightness: 1.0,
        }
    }

    fn send(&self, method: &'static str, params: Value) {
        let _ = self.tx.send(Command { method, params });
    }

    fn effect_spec(&self) -> Value {
        let [r, g, b] = self.color;
        match self.kind {
            EffectKind::Solid => json!({"type": "solid", "color": {"r": r, "g": g, "b": b}}),
            EffectKind::RainbowWave => {
                json!({"type": "rainbow_wave", "speed": self.speed, "scale": self.scale})
            }
            EffectKind::Breathing => json!({
                "type": "breathing",
                "color": {"r": r, "g": g, "b": b},
                "period_ms": self.period_ms,
            }),
        }
    }

    /// Takes the controls' initial values from the daemon's current state once.
    fn sync_from(&mut self, effect: &Value, diag: Option<&Value>) {
        let f = |k: &str| effect.get(k).and_then(Value::as_f64).map(|v| v as f32);
        if let Some(c) = effect.get("color") {
            let ch = |k: &str| c.get(k).and_then(Value::as_u64).unwrap_or(0).min(255) as u8;
            self.color = [ch("r"), ch("g"), ch("b")];
        }
        match effect.get("type").and_then(Value::as_str) {
            Some("solid") => self.kind = EffectKind::Solid,
            Some("rainbow_wave") => {
                self.kind = EffectKind::RainbowWave;
                self.speed = f("speed").unwrap_or(self.speed);
                self.scale = f("scale").unwrap_or(self.scale);
            }
            Some("breathing") => {
                self.kind = EffectKind::Breathing;
                self.period_ms = f("period_ms").map_or(self.period_ms, |v| v as u32);
            }
            _ => {}
        }
        if let Some(b) = diag
            .and_then(|d| d.get("brightness"))
            .and_then(Value::as_f64)
        {
            self.brightness = b as f32;
        }
    }
}

fn device_row(ui: &mut egui::Ui, d: &Value) {
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("?").to_owned();
    let info = d.get("info").cloned().unwrap_or(Value::Null);
    let status = match d.get("status") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(o)) => o.keys().next().cloned().unwrap_or_default(),
        _ => "?".into(),
    };
    let color = if status == "Connected" {
        egui::Color32::from_rgb(80, 200, 120)
    } else {
        egui::Color32::from_rgb(230, 160, 60)
    };
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.colored_label(color, "●");
            ui.strong(s(&info, "name"));
        });
        ui.label(format!(
            "{} · {} LEDs · {} frames",
            s(&info, "vendor"),
            d.get("leds").and_then(Value::as_u64).unwrap_or(0),
            d.get("frames").and_then(Value::as_u64).unwrap_or(0),
        ));
        if let Some(fw) = info.get("firmware").and_then(Value::as_str) {
            ui.label(format!("Firmware {fw}"));
        }
        ui.label(format!("Status: {status}"));
        if let Some(err) = d.get("last_error").and_then(Value::as_str) {
            ui.colored_label(egui::Color32::from_rgb(230, 90, 90), err);
        }
    });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (connected, error, devices, diag, effect, profiles, generation) = {
            let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (
                s.connected,
                s.error.clone(),
                s.devices.clone(),
                s.diagnostics.clone(),
                s.effect.clone(),
                s.profiles.clone(),
                s.generation,
            )
        };
        if let (Some(after), Some(e)) = (self.sync_after_generation, &effect) {
            if generation > after {
                self.sync_from(e, diag.as_ref());
                self.sync_after_generation = None;
            }
        }

        egui::TopBottomPanel::top("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("freeRGB");
                ui.separator();
                if connected {
                    let fps = diag
                        .as_ref()
                        .and_then(|d| d.get("measured_fps"))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    ui.colored_label(egui::Color32::from_rgb(80, 200, 120), "daemon connected");
                    ui.label(format!("{fps:.1} fps"));
                } else {
                    ui.colored_label(egui::Color32::from_rgb(230, 90, 90), "daemon not reachable");
                }
            });
            if let Some(e) = &error {
                ui.colored_label(egui::Color32::from_rgb(230, 90, 90), e);
            }
        });

        egui::SidePanel::left("devices")
            .min_width(240.0)
            .show(ctx, |ui| {
                ui.heading("Devices");
                if devices.is_empty() {
                    ui.label(if connected {
                        "No devices."
                    } else {
                        "Start freergbd to see devices."
                    });
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for d in &devices {
                        device_row(ui, d);
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(connected, |ui| {
                ui.heading("Effect");
                let mut changed = false;
                egui::ComboBox::from_id_salt("effect")
                    .selected_text(self.kind.label())
                    .show_ui(ui, |ui| {
                        for k in EffectKind::ALL {
                            changed |= ui.selectable_value(&mut self.kind, k, k.label()).changed();
                        }
                    });

                if matches!(self.kind, EffectKind::Solid | EffectKind::Breathing) {
                    ui.horizontal(|ui| {
                        ui.label("Colour");
                        changed |= ui.color_edit_button_srgb(&mut self.color).changed();
                        for (name, c) in [
                            ("Red", [255, 0, 0]),
                            ("Green", [0, 255, 0]),
                            ("Blue", [0, 0, 255]),
                            ("White", [255, 255, 255]),
                            ("Off", [0, 0, 0]),
                        ] {
                            if ui.button(name).clicked() {
                                self.color = c;
                                changed = true;
                            }
                        }
                    });
                }
                match self.kind {
                    EffectKind::RainbowWave => {
                        changed |= ui
                            .add(egui::Slider::new(&mut self.speed, -2.0..=2.0).text("speed"))
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut self.scale, 0.1..=5.0)
                                    .text("cycles across canvas"),
                            )
                            .changed();
                    }
                    EffectKind::Breathing => {
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut self.period_ms, 500..=10_000)
                                    .text("period (ms)"),
                            )
                            .changed();
                    }
                    EffectKind::Solid => {}
                }
                if changed {
                    self.send("effects.set", self.effect_spec());
                }

                ui.separator();
                ui.heading("Brightness");
                let r =
                    ui.add(egui::Slider::new(&mut self.brightness, 0.0..=1.0).show_value(false));
                ui.label(format!("{:.0} %", self.brightness * 100.0));
                if r.changed() {
                    self.send("brightness.set", json!({"value": self.brightness}));
                }

                ui.separator();
                ui.heading("Profiles");
                match &profiles {
                    None => {
                        ui.label("Profiles are disabled (daemon started with --no-state).");
                    }
                    Some(list) => {
                        if self
                            .selected_profile
                            .as_ref()
                            .is_some_and(|p| !list.contains(p))
                        {
                            self.selected_profile = None;
                        }
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("profiles")
                                .selected_text(
                                    self.selected_profile.as_deref().unwrap_or("(choose)"),
                                )
                                .show_ui(ui, |ui| {
                                    for p in list {
                                        ui.selectable_value(
                                            &mut self.selected_profile,
                                            Some(p.clone()),
                                            p,
                                        );
                                    }
                                });
                            let sel = self.selected_profile.clone();
                            if ui
                                .add_enabled(sel.is_some(), egui::Button::new("Load"))
                                .clicked()
                            {
                                self.send("profiles.load", json!({"name": sel}));
                                self.sync_after_generation = Some(generation);
                            }
                            if ui
                                .add_enabled(sel.is_some(), egui::Button::new("Delete"))
                                .clicked()
                            {
                                self.send("profiles.delete", json!({"name": sel}));
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.profile_name)
                                    .hint_text("new profile name")
                                    .desired_width(180.0),
                            );
                            let name = self.profile_name.trim().to_owned();
                            if ui
                                .add_enabled(!name.is_empty(), egui::Button::new("Save current"))
                                .clicked()
                            {
                                self.send("profiles.save", json!({"name": name}));
                                self.selected_profile = Some(name);
                                self.profile_name.clear();
                            }
                        });
                    }
                }

                ui.separator();
                if ui
                    .button("Stop daemon (devices return to hardware lighting)")
                    .clicked()
                {
                    self.send("daemon.shutdown", Value::Null);
                }
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("freeRGB")
            .with_inner_size([760.0, 460.0]),
        ..Default::default()
    };
    eframe::run_native(
        "freeRGB",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
