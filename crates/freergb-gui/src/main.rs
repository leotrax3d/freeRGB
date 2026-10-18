//! freeRGB GUI. Holds no device logic: every action is a JSON-RPC call to
//! freergbd, made on a worker thread so the UI never blocks. While idle the
//! window repaints only when new state arrives (about once per second).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use freergb_core::{Region, Rgb};
use freergb_effects::layers::{BlendMode, Layer, MAX_LAYERS};
use freergb_effects::EffectSpec;
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
    /// Saved custom effects; `None` without storage (`--no-state`).
    custom_effects: Option<Vec<String>>,
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
                // command per method and target device.
                let mut pending = vec![cmd];
                while let Ok(c) = rx.try_recv() {
                    pending.retain(|p| {
                        p.method != c.method || p.params.get("id") != c.params.get("id")
                    });
                    pending.push(c);
                }
                for c in pending {
                    // GUI-side shortcut: fetch a saved custom effect and apply it.
                    let result = if c.method == "gui.apply_custom_effect" {
                        freergb_client::call("custom_effects.get", c.params)
                            .and_then(|spec| freergb_client::call("effects.set", spec))
                    } else {
                        freergb_client::call(c.method, c.params)
                    };
                    if let Err(e) = result {
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
        let custom_effects = freergb_client::call("custom_effects.list", Value::Null)
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
                    s.custom_effects = custom_effects;
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

fn label(name: &str) -> &'static str {
    match name {
        "solid" => "Solid colour",
        "rainbow_wave" => "Rainbow wave",
        "breathing" => "Breathing",
        "gradient" => "Gradient",
        "color_cycle" => "Colour cycle",
        "radial_rainbow" => "Radial rainbow",
        "scanner" => "Scanner",
        "twinkle" => "Twinkle",
        "audio_pulse" => "Audio pulse",
        "audio_spectrum" => "Audio spectrum",
        "screen_ambience" => "Screen ambience",
        "layers" => "Custom (layers)",
        _ => "Unknown",
    }
}

struct App {
    state: Arc<Mutex<Snapshot>>,
    tx: Sender<Command>,
    /// Controls are (re)loaded from the first snapshot newer than this.
    sync_after_generation: Option<u64>,
    profile_name: String,
    custom_name: String,
    selected_custom: Option<String>,
    selected_profile: Option<String>,
    spec: EffectSpec,
    brightness: f32,
    /// Per-device slider values not yet confirmed by the daemon.
    device_brightness: std::collections::HashMap<String, f32>,
    /// Canvas regions dragged in the layout editor, not yet confirmed.
    device_regions: std::collections::HashMap<String, Region>,
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
            custom_name: String::new(),
            selected_custom: None,
            selected_profile: None,
            spec: EffectSpec::default_for("rainbow_wave").expect("known effect"),
            brightness: 1.0,
            device_brightness: Default::default(),
            device_regions: Default::default(),
        }
    }

    fn send(&self, method: &'static str, params: Value) {
        let _ = self.tx.send(Command { method, params });
    }

    /// Takes the controls' values from the daemon's current state.
    fn sync_from(&mut self, effect: &Value, diag: Option<&Value>) {
        if let Ok(spec) = serde_json::from_value::<EffectSpec>(effect.clone()) {
            self.spec = spec;
        }
        if let Some(b) = diag
            .and_then(|d| d.get("brightness"))
            .and_then(Value::as_f64)
        {
            self.brightness = b as f32;
        }
    }
}

fn color_edit(ui: &mut egui::Ui, text: &str, c: &mut Rgb) -> bool {
    let mut arr = c.to_array();
    let changed = ui
        .horizontal(|ui| {
            ui.label(text);
            ui.color_edit_button_srgb(&mut arr).changed()
        })
        .inner;
    if changed {
        *c = Rgb::from(arr);
    }
    changed
}

fn period_edit(ui: &mut egui::Ui, ms: &mut u32, range: std::ops::RangeInclusive<u32>) -> bool {
    ui.add(egui::Slider::new(ms, range).text("period (ms)"))
        .changed()
}

fn blend_label(b: BlendMode) -> &'static str {
    match b {
        BlendMode::Normal => "Normal",
        BlendMode::Add => "Add",
        BlendMode::Multiply => "Multiply",
        BlendMode::Screen => "Screen",
        BlendMode::Max => "Max",
    }
}

/// Editor for a custom (layered) effect. Returns `true` on any change.
fn layers_editor(ui: &mut egui::Ui, layers: &mut Vec<Layer>) -> bool {
    let mut changed = false;
    let mut action: Option<(usize, i8)> = None; // (index, -1 up / +1 down / 0 delete)
    let count = layers.len();
    for (i, layer) in layers.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            egui::CollapsingHeader::new(format!(
                "Layer {} – {}",
                i + 1,
                label(layer.effect.name())
            ))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let current = layer.effect.name();
                    egui::ComboBox::from_id_salt("layer-effect")
                        .selected_text(label(current))
                        .show_ui(ui, |ui| {
                            // Nested custom effects are not allowed.
                            for name in EffectSpec::NAMES.iter().filter(|n| **n != "layers") {
                                if ui.selectable_label(*name == current, label(name)).clicked()
                                    && *name != current
                                {
                                    if let Some(s) = EffectSpec::default_for(name) {
                                        layer.effect = s;
                                        changed = true;
                                    }
                                }
                            }
                        });
                    egui::ComboBox::from_id_salt("layer-blend")
                        .selected_text(blend_label(layer.blend))
                        .show_ui(ui, |ui| {
                            for b in BlendMode::ALL {
                                changed |= ui
                                    .selectable_value(&mut layer.blend, b, blend_label(b))
                                    .changed();
                            }
                        });
                    if ui.add_enabled(i > 0, egui::Button::new("▲")).clicked() {
                        action = Some((i, -1));
                    }
                    if ui
                        .add_enabled(i + 1 < count, egui::Button::new("▼"))
                        .clicked()
                    {
                        action = Some((i, 1));
                    }
                    if ui.add_enabled(count > 1, egui::Button::new("✕")).clicked() {
                        action = Some((i, 0));
                    }
                });
                changed |= ui
                    .add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).text("opacity"))
                    .changed();
                let mut limited = layer.region.is_some();
                if ui
                    .checkbox(&mut limited, "Only part of the canvas")
                    .changed()
                {
                    layer.region = limited.then_some(Region::FULL);
                    changed = true;
                }
                if let Some(r) = &mut layer.region {
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(egui::Slider::new(&mut r.x, 0.0..=0.99).text("x"))
                            .changed();
                        changed |= ui
                            .add(egui::Slider::new(&mut r.y, 0.0..=0.99).text("y"))
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(egui::Slider::new(&mut r.w, 0.01..=1.0).text("w"))
                            .changed();
                        changed |= ui
                            .add(egui::Slider::new(&mut r.h, 0.01..=1.0).text("h"))
                            .changed();
                    });
                }
                changed |= effect_params(ui, &mut layer.effect);
            });
        });
    }
    match action {
        Some((i, 0)) => {
            layers.remove(i);
            changed = true;
        }
        Some((i, d)) => {
            let j = if d < 0 { i - 1 } else { i + 1 };
            layers.swap(i, j);
            changed = true;
        }
        None => {}
    }
    if layers.len() < MAX_LAYERS && ui.button("+ Add layer").clicked() {
        let mut l = Layer::new(EffectSpec::default_for("solid").expect("known effect"));
        l.blend = BlendMode::Add;
        l.opacity = 0.5;
        layers.push(l);
        changed = true;
    }
    changed
}

/// Parameter controls for the current effect. Returns `true` on any change.
fn effect_params(ui: &mut egui::Ui, spec: &mut EffectSpec) -> bool {
    let mut changed = false;
    match spec {
        EffectSpec::Solid { color } => {
            changed |= color_edit(ui, "Colour", color);
            ui.horizontal(|ui| {
                for (name, c) in [
                    ("Red", Rgb::RED),
                    ("Green", Rgb::GREEN),
                    ("Blue", Rgb::BLUE),
                    ("White", Rgb::WHITE),
                    ("Off", Rgb::BLACK),
                ] {
                    if ui.button(name).clicked() {
                        *color = c;
                        changed = true;
                    }
                }
            });
        }
        EffectSpec::RainbowWave { speed, scale } | EffectSpec::RadialRainbow { speed, scale } => {
            changed |= ui
                .add(egui::Slider::new(speed, -2.0..=2.0).text("speed"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(scale, 0.1..=5.0).text("cycles"))
                .changed();
        }
        EffectSpec::Breathing { color, period_ms } => {
            changed |= color_edit(ui, "Colour", color);
            changed |= period_edit(ui, period_ms, 500..=10_000);
        }
        EffectSpec::Gradient {
            from,
            to,
            angle_deg,
        } => {
            changed |= color_edit(ui, "From", from);
            changed |= color_edit(ui, "To", to);
            changed |= ui
                .add(egui::Slider::new(angle_deg, 0.0..=360.0).text("angle (°)"))
                .changed();
        }
        EffectSpec::ColorCycle { period_ms } => {
            changed |= period_edit(ui, period_ms, 1_000..=60_000);
        }
        EffectSpec::Scanner {
            color,
            background,
            period_ms,
            width,
        } => {
            changed |= color_edit(ui, "Colour", color);
            changed |= color_edit(ui, "Background", background);
            changed |= period_edit(ui, period_ms, 300..=10_000);
            changed |= ui
                .add(egui::Slider::new(width, 0.02..=0.5).text("width"))
                .changed();
        }
        EffectSpec::Twinkle {
            color,
            background,
            density,
            period_ms,
        } => {
            changed |= color_edit(ui, "Colour", color);
            changed |= color_edit(ui, "Background", background);
            changed |= ui
                .add(egui::Slider::new(density, 0.0..=1.0).text("density"))
                .changed();
            changed |= period_edit(ui, period_ms, 200..=5_000);
        }
        EffectSpec::AudioPulse {
            color,
            background,
            sensitivity,
        } => {
            changed |= color_edit(ui, "Colour", color);
            changed |= color_edit(ui, "Background", background);
            changed |= ui
                .add(egui::Slider::new(sensitivity, 0.5..=4.0).text("sensitivity"))
                .changed();
            ui.label("Reacts to the sound your PC plays.");
        }
        EffectSpec::AudioSpectrum {
            low,
            high,
            background,
        } => {
            changed |= color_edit(ui, "Bass colour", low);
            changed |= color_edit(ui, "Treble colour", high);
            changed |= color_edit(ui, "Background", background);
            ui.label("8 frequency bands across the canvas, bass on the left.");
        }
        EffectSpec::ScreenAmbience { saturation } => {
            changed |= ui
                .add(egui::Slider::new(saturation, 0.5..=2.5).text("saturation"))
                .changed();
            ui.label("Mirrors the primary monitor. Place devices in the Layout section to match where they sit around the screen.");
        }
        EffectSpec::Layers { layers } => {
            ui.label("Custom effect: layers are drawn bottom to top.");
            changed |= layers_editor(ui, layers);
        }
    }
    changed
}

/// One device card. `local` holds slider values the user changed but the
/// daemon has not reported back yet. Returns `(id, brightness)` on change.
fn device_row(
    ui: &mut egui::Ui,
    d: &Value,
    local: &mut std::collections::HashMap<String, f32>,
) -> Option<(String, f32)> {
    let mut changed = None;
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
        let id = s(&info, "id");
        let reported = d.get("brightness").and_then(Value::as_f64).unwrap_or(1.0) as f32;
        if local.get(&id).is_some_and(|v| (v - reported).abs() < 0.005) {
            local.remove(&id);
        }
        let mut b = local.get(&id).copied().unwrap_or(reported);
        ui.horizontal(|ui| {
            ui.label("Brightness");
            if ui
                .add(egui::Slider::new(&mut b, 0.0..=1.0).show_value(false))
                .changed()
            {
                local.insert(id.clone(), b);
                changed = Some((id.clone(), b));
            }
            ui.label(format!("{:.0} %", b * 100.0));
        });
    });
    changed
}

/// Canvas layout editor: every device is a rectangle on the effect canvas.
/// Drag to move, drag the bottom-right corner to resize. `local` holds
/// positions changed here but not yet reported back by the daemon. Returns the
/// region to send when a drag ends.
fn layout_editor(
    ui: &mut egui::Ui,
    devices: &[Value],
    local: &mut std::collections::HashMap<String, Region>,
) -> Option<(String, Region)> {
    const PALETTE: [egui::Color32; 6] = [
        egui::Color32::from_rgb(90, 160, 250),
        egui::Color32::from_rgb(250, 140, 70),
        egui::Color32::from_rgb(120, 210, 120),
        egui::Color32::from_rgb(220, 110, 220),
        egui::Color32::from_rgb(240, 210, 80),
        egui::Color32::from_rgb(110, 210, 210),
    ];
    let width = ui.available_width().min(480.0);
    let (resp, painter) =
        ui.allocate_painter(egui::vec2(width, width * 0.625), egui::Sense::hover());
    let canvas = resp.rect;
    painter.rect_filled(canvas, 4.0, egui::Color32::from_gray(30));
    let (cw, ch) = (canvas.width(), canvas.height());
    let mut done = None;

    for (i, d) in devices.iter().enumerate() {
        let Some(id) = d
            .get("info")
            .and_then(|v| v.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            continue;
        };
        let name = d
            .get("info")
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("?");
        let reported: Region = d
            .get("region")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or(Region::FULL);
        if local.get(&id).is_some_and(|r| {
            (r.x - reported.x).abs() < 0.002
                && (r.y - reported.y).abs() < 0.002
                && (r.w - reported.w).abs() < 0.002
                && (r.h - reported.h).abs() < 0.002
        }) {
            local.remove(&id);
        }
        let mut r = local.get(&id).copied().unwrap_or(reported);

        let rect = egui::Rect::from_min_size(
            canvas.min + egui::vec2(r.x * cw, r.y * ch),
            egui::vec2(r.w * cw, r.h * ch),
        );
        let handle = egui::Rect::from_center_size(rect.max, egui::vec2(12.0, 12.0));
        let body = ui.interact(
            rect,
            egui::Id::new(("layout-body", &id)),
            egui::Sense::drag(),
        );
        let corner = ui.interact(
            handle,
            egui::Id::new(("layout-corner", &id)),
            egui::Sense::drag(),
        );

        let mut moved = false;
        if corner.dragged() {
            let dlt = corner.drag_delta();
            r.w = (r.w + dlt.x / cw).clamp(0.02, 1.0 - r.x);
            r.h = (r.h + dlt.y / ch).clamp(0.02, 1.0 - r.y);
            moved = true;
        } else if body.dragged() {
            let dlt = body.drag_delta();
            r.x = (r.x + dlt.x / cw).clamp(0.0, 1.0 - r.w);
            r.y = (r.y + dlt.y / ch).clamp(0.0, 1.0 - r.h);
            moved = true;
        }
        if moved {
            local.insert(id.clone(), r);
        }
        if body.drag_stopped() || corner.drag_stopped() {
            done = Some((id.clone(), r));
        }

        let color = PALETTE[i % PALETTE.len()];
        let rect = egui::Rect::from_min_size(
            canvas.min + egui::vec2(r.x * cw, r.y * ch),
            egui::vec2(r.w * cw, r.h * ch),
        );
        painter.rect_filled(rect, 3.0, color.gamma_multiply(0.25));
        painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.5_f32, color));
        painter.rect_filled(
            egui::Rect::from_center_size(rect.max, egui::vec2(8.0, 8.0)),
            1.0,
            color,
        );
        painter.text(
            rect.min + egui::vec2(4.0, 3.0),
            egui::Align2::LEFT_TOP,
            name,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }
    done
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (connected, error, devices, diag, effect, profiles, custom_effects, generation) = {
            let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (
                s.connected,
                s.error.clone(),
                s.devices.clone(),
                s.diagnostics.clone(),
                s.effect.clone(),
                s.profiles.clone(),
                s.custom_effects.clone(),
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
            let conflicts: Vec<&str> = diag
                .as_ref()
                .and_then(|d| d.get("conflicts"))
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if !conflicts.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(240, 180, 60),
                    format!(
                        "{} is running – freeRGB leaves your hardware alone until it is closed.",
                        conflicts.join(", ")
                    ),
                );
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
                        if let Some((id, b)) = device_row(ui, d, &mut self.device_brightness) {
                            self.send("devices.set_brightness", json!({"id": id, "value": b}));
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(connected, |ui| {
                ui.heading("Effect");
                let mut changed = false;
                let current = self.spec.name();
                egui::ComboBox::from_id_salt("effect")
                    .selected_text(label(current))
                    .show_ui(ui, |ui| {
                        for name in EffectSpec::NAMES {
                            if ui.selectable_label(name == current, label(name)).clicked()
                                && name != current
                            {
                                if let Some(s) = EffectSpec::default_for(name) {
                                    self.spec = s;
                                    changed = true;
                                }
                            }
                        }
                    });
                changed |= effect_params(ui, &mut self.spec);
                if changed {
                    if let Ok(v) = serde_json::to_value(&self.spec) {
                        self.send("effects.set", v);
                    }
                }

                if let Some(list) = &custom_effects {
                    ui.collapsing("My effects", |ui| {
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("custom-effects")
                                .selected_text(self.selected_custom.as_deref().unwrap_or("(choose)"))
                                .show_ui(ui, |ui| {
                                    for n in list {
                                        ui.selectable_value(&mut self.selected_custom, Some(n.clone()), n);
                                    }
                                });
                            let sel = self.selected_custom.clone();
                            if ui.add_enabled(sel.is_some(), egui::Button::new("Apply")).clicked() {
                                self.send("gui.apply_custom_effect", json!({"name": sel}));
                                self.sync_after_generation = Some(generation);
                            }
                            if ui.add_enabled(sel.is_some(), egui::Button::new("Delete")).clicked() {
                                self.send("custom_effects.delete", json!({"name": sel}));
                                self.selected_custom = None;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.custom_name)
                                    .hint_text("name for the current effect")
                                    .desired_width(180.0),
                            );
                            let name = self.custom_name.trim().to_owned();
                            if ui.add_enabled(!name.is_empty(), egui::Button::new("Save")).clicked() {
                                if let Ok(v) = serde_json::to_value(&self.spec) {
                                    self.send("custom_effects.save", json!({"name": name, "effect": v}));
                                    self.selected_custom = Some(name);
                                    self.custom_name.clear();
                                }
                            }
                        });
                        ui.label("Tip: choose \"Custom (layers)\" above to build an effect from several layers.");
                    });
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
                ui.heading("Layout");
                ui.label("Where each device sits on the effect canvas. Drag to move, drag the corner to resize.");
                if let Some((id, r)) = layout_editor(ui, &devices, &mut self.device_regions) {
                    self.send(
                        "devices.set_region",
                        json!({"id": id, "x": r.x, "y": r.y, "w": r.w, "h": r.h}),
                    );
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
