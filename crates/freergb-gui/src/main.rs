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
mod alerts;
mod app_profiles;
mod channel_bindings;
mod composite_curves;
mod custom_sensors;
mod diagnostics;
mod hotkeys;
mod hwmon_cooling;
mod keyboard_reactive;
mod macro_device_keys;
mod macro_editor;
mod macro_recording;
mod mainboard_cooling;
mod mainboard_rgb;
mod pages;
mod preview;
mod sensor_history;
mod theme;
mod toast;

/// Latest state fetched from the daemon.
#[derive(Default)]
struct Snapshot {
    exported_report: Option<std::path::PathBuf>,
    recorded_draft: Option<(u64, Value)>,
    control: Value,
    connected: bool,
    error: Option<String>,
    devices: Vec<Value>,
    plugins: Vec<Value>,
    diagnostics: Option<Value>,
    effect: Option<Value>,
    /// `None` when the daemon runs without profile storage (`--no-state`).
    profiles: Option<Vec<String>>,
    /// Saved custom effects; `None` without storage (`--no-state`).
    custom_effects: Option<Vec<String>>,
    /// Increments with every successful fetch.
    generation: u64,
    profile_revision: u64,
}

/// A call queued by the UI.
struct Command {
    method: &'static str,
    params: Value,
}

fn is_setter(method: &str) -> bool {
    matches!(
        method,
        "brightness.set"
            | "color.set"
            | "effects.set"
            | "devices.set_region"
            | "devices.set_brightness"
    )
}

/// Slider writes can merge only after the most recent action/query barrier.
/// Save/run/cancel actions retain their original ordering and parameters.
fn push_command(pending: &mut Vec<Command>, command: Command) {
    if is_setter(command.method) {
        let start = pending
            .iter()
            .rposition(|p| !is_setter(p.method))
            .map_or(0, |index| index + 1);
        let mut index = start;
        while index < pending.len() {
            if pending[index].method == command.method
                && pending[index].params.get("id") == command.params.get("id")
            {
                pending.remove(index);
            } else {
                index += 1;
            }
        }
    }
    pending.push(command);
}

fn optional_query(reachable: &mut bool, method: &str) -> Option<Value> {
    if !*reachable {
        return None;
    }
    match freergb_client::call(method, Value::Null) {
        Ok(value) => Some(value),
        Err(freergb_client::ClientError::Rpc { .. }) => None,
        Err(_) => {
            *reachable = false;
            None
        }
    }
}

fn worker(rx: Receiver<Command>, state: Arc<Mutex<Snapshot>>, ctx: egui::Context) {
    let mut profile_revision = 0;
    loop {
        let mut last_error = None;
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(cmd) => {
                // Coalesce bursts (e.g. slider drags): keep only the newest
                // command per method and target device.
                let mut pending = vec![cmd];
                while let Ok(c) = rx.try_recv() {
                    push_command(&mut pending, c);
                }
                for c in pending {
                    if c.method == "macros.run"
                        && !state.lock().unwrap_or_else(|e| e.into_inner()).connected
                    {
                        last_error = Some("Macro discarded: daemon is disconnected".into());
                        continue;
                    }
                    // GUI-side shortcut: fetch a saved custom effect and apply it.
                    let result = if c.method == "gui.load_app_profile" {
                        app_profiles::load_if_current(c.params)
                    } else if c.method == "gui.export_diagnostics" {
                        match freergb_client::call("diagnostics.export", c.params.clone()) {
                            Ok(report) => match diagnostics::save(&report) {
                                Ok(path) => {
                                    state
                                        .lock()
                                        .unwrap_or_else(|e| e.into_inner())
                                        .exported_report = Some(path);
                                    Ok(Value::Bool(true))
                                }
                                Err(error) => {
                                    last_error = Some(error);
                                    continue;
                                }
                            },
                            Err(error) => Err(error),
                        }
                    } else if c.method == "gui.apply_custom_effect" {
                        freergb_client::call("custom_effects.get", c.params)
                            .and_then(|spec| freergb_client::call("effects.set", spec))
                    } else {
                        freergb_client::call(c.method, c.params)
                    };
                    if matches!(c.method, "profiles.load" | "gui.load_app_profile")
                        && result
                            .as_ref()
                            .is_ok_and(|value| value == &Value::Bool(true))
                    {
                        profile_revision += 1;
                    }
                    if c.method == "macros.record.stop" {
                        if let Ok(value) = &result {
                            if value.is_object() {
                                let mut snapshot = state.lock().unwrap_or_else(|e| e.into_inner());
                                let revision =
                                    snapshot.recorded_draft.as_ref().map_or(1, |(r, _)| r + 1);
                                snapshot.recorded_draft = Some((revision, value.clone()));
                            }
                        }
                    }
                    if let Err(e) = result {
                        let transport_failed =
                            !matches!(e, freergb_client::ClientError::Rpc { .. });
                        last_error = Some(e.to_string());
                        if transport_failed {
                            break;
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        let mut fetched = (|| -> Result<_, freergb_client::ClientError> {
            Ok((
                freergb_client::call("devices.list", Value::Null)?,
                freergb_client::call("diagnostics.get", Value::Null)?,
                freergb_client::call("effects.get", Value::Null)?,
            ))
        })();
        let mut reachable = fetched.is_ok();
        let mut get = |method| optional_query(&mut reachable, method);
        let profiles =
            get("profiles.list").and_then(|v| serde_json::from_value::<Vec<String>>(v).ok());
        let plugins = get("plugins.list")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        let custom_effects =
            get("custom_effects.list").and_then(|v| serde_json::from_value::<Vec<String>>(v).ok());
        let control = json!({
            "keyboard_reactive": get("keyboard.reactive.get").unwrap_or(Value::Null),
            "curves": get("cooling.curves.list").unwrap_or(Value::Null),
            "macros": get("macros.list").unwrap_or(Value::Null),
            "macro_status": get("macros.status").unwrap_or(Value::Null),
            "macro_device_keys": get("macros.device_keys.get").unwrap_or(Value::Null),
            "sensor_sources": get("cooling.sensors.sources").unwrap_or(Value::Null),
            "sensor_values": get("cooling.sensors.values").unwrap_or(Value::Null),
            "sensor_graph": get("cooling.sensors.get").unwrap_or(Value::Null),
            "curve_set": get("cooling.curve_set.get").unwrap_or(Value::Null),
            "curve_set_values": get("cooling.curve_set.values").unwrap_or(Value::Null),
            "cooling_providers": get("cooling.providers.list").unwrap_or(Value::Null),
            "cooling_channels": get("cooling.channels.list").unwrap_or(Value::Null),
            "cooling_session": get("cooling.sessions.status").unwrap_or(Value::Null),
            "cooling_background": get("cooling.background.get").unwrap_or(Value::Null),
            "cooling_support": get("cooling.support.get").unwrap_or(Value::Null),
            "cooling_profiles": get("cooling.profiles.list").unwrap_or(Value::Null),
            "channel_bindings": get("cooling.bindings.get").unwrap_or(Value::Null),
            "mainboard_rgb": get("mainboard_rgb.get").unwrap_or(Value::Null),
            "hwmon": get("cooling.hwmon.get").unwrap_or(Value::Null),
            "monitoring_sensors": get("monitoring.sensors.list").unwrap_or(Value::Null),
        });
        if !reachable && fetched.is_ok() {
            fetched = Err(freergb_client::ClientError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "connection lost while refreshing daemon state",
            )));
        }
        {
            let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
            match fetched {
                Ok((devices, diag, effect)) => {
                    s.connected = true;
                    s.control = control;
                    s.devices = devices.as_array().cloned().unwrap_or_default();
                    s.plugins = plugins;
                    s.diagnostics = Some(diag);
                    s.effect = Some(effect);
                    s.profiles = profiles;
                    s.custom_effects = custom_effects;
                    s.generation += 1;
                    s.profile_revision = profile_revision;
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
    keyboard_panel: keyboard_reactive::KeyboardPanel,
    profile_revision: u64,
    app_profiles: app_profiles::AppProfiles,
    panels: pages::Panels,
    toasts: toast::Toasts,
    preview: preview::Preview,
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
        theme::install(&cc.egui_ctx);
        let state = Arc::new(Mutex::new(Snapshot::default()));
        let (tx, rx) = mpsc::channel();
        let (s, ctx) = (state.clone(), cc.egui_ctx.clone());
        std::thread::Builder::new()
            .name("api-worker".into())
            .spawn(move || worker(rx, s, ctx))
            .expect("spawn worker thread");
        Self {
            profile_revision: 0,
            app_profiles: Default::default(),
            panels: Default::default(),
            toasts: Default::default(),
            preview: Default::default(),
            keyboard_panel: Default::default(),
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
        EffectSpec::Marketplace { id, version, .. } => {
            ui.label(format!(
                "{id} {version} — configure in the modern Marketplace"
            ));
        }
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
/// Drag to move, drag the bottom-right corner to resize, right-click to reset
/// a device to the full canvas. `local` holds positions changed here but not
/// yet reported back by the daemon. Returns the region to send when a drag
/// ends, and the device id whose region should reset to `Region::FULL`.
fn layout_editor(
    ui: &mut egui::Ui,
    devices: &[Value],
    local: &mut std::collections::HashMap<String, Region>,
) -> (Option<(String, Region)>, Option<String>) {
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
    let mut reset = None;

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
        if body.secondary_clicked() {
            reset = Some(id.clone());
            local.remove(&id);
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
    (done, reset)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_secs(1));
        self.toasts.show(ctx);
        {
            let snapshot = self.state.lock().unwrap_or_else(|e| e.into_inner());
            self.panels.observe(&snapshot);
            self.panels.poll_recording(ctx, &self.tx, &snapshot);
            self.panels.poll_hotkeys(&self.tx, &snapshot);
            self.app_profiles.poll(&self.tx, &snapshot);
        }
        let (
            connected,
            error,
            devices,
            plugins,
            diag,
            effect,
            profiles,
            custom_effects,
            generation,
            profile_revision,
        ) = {
            let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (
                s.connected,
                s.error.clone(),
                s.devices.clone(),
                s.plugins.clone(),
                s.diagnostics.clone(),
                s.effect.clone(),
                s.profiles.clone(),
                s.custom_effects.clone(),
                s.generation,
                s.profile_revision,
            )
        };
        if self.profile_revision != profile_revision {
            if let Some(effect) = &effect {
                self.sync_from(effect, diag.as_ref());
                self.profile_revision = profile_revision;
                self.sync_after_generation = None;
            }
        }
        if let (Some(after), Some(e)) = (self.sync_after_generation, &effect) {
            if generation > after {
                self.sync_from(e, diag.as_ref());
                self.sync_after_generation = None;
            }
        }

        let warnings = self.panels.sensor_warnings();
        egui::TopBottomPanel::top("status").show(ctx, |ui| {
            if self.app_profiles.active()
                && ui.button("Application profiles active — pause").clicked()
            {
                self.app_profiles.disable();
                self.toasts.success("Application profiles paused");
            }
            if warnings > 0
                && ui
                    .button(format!("⚠ {warnings} sensor warning(s) — open Monitoring"))
                    .clicked()
            {
                self.panels.page = 4;
            }
            if self.panels.hotkeys_active() {
                ui.colored_label(
                    theme::GOOD,
                    "Global macro shortcuts active · Ctrl+Shift+F11 cancels",
                );
            }
            if let Some(e) = &error {
                ui.colored_label(theme::BAD, e);
            }
            let conflicts: Vec<&str> = diag
                .as_ref()
                .and_then(|d| d.get("conflicts"))
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if !conflicts.is_empty() {
                ui.colored_label(
                    theme::WARN,
                    format!(
                        "{} is running – freeRGB leaves your hardware alone until it is closed.",
                        conflicts.join(", ")
                    ),
                );
            }
        });

        // Navigation rail with the daemon status pinned to the bottom.
        egui::SidePanel::left("nav")
            .resizable(false)
            .min_width(170.0)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.add_space(2.0);
                    ui.strong("freeRGB · Classic");
                    ui.add_space(6.0);
                    for (page, name) in [
                        (0, "Lighting"),
                        (1, "Cooling"),
                        (5, "Headers"),
                        (2, "Macros"),
                        (4, "Monitoring"),
                        (3, "Updates"),
                    ] {
                        let text = if page == 4 && warnings > 0 {
                            format!("{name}  ⚠ {warnings}")
                        } else {
                            name.to_owned()
                        };
                        let mut button = egui::Button::new(egui::RichText::new(text).size(15.0));
                        if self.panels.page == page {
                            button = button
                                .fill(theme::SELECTION)
                                .stroke(egui::Stroke::new(1.0_f32, theme::ACCENT));
                        }
                        if ui.add_sized([140.0, 30.0], button).clicked() {
                            self.panels.page = page;
                        }
                    }
                });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    if connected {
                        let fps = diag
                            .as_ref()
                            .and_then(|d| d.get("measured_fps"))
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0);
                        ui.small(format!("{fps:.1} fps"));
                        ui.colored_label(theme::GOOD, "● daemon connected");
                    } else {
                        ui.colored_label(theme::BAD, "● daemon not reachable");
                        ui.small("Start freergbd to connect.");
                    }
                });
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
                    if !plugins.is_empty() {
                        ui.separator();
                        ui.heading("Plugins");
                        for plugin in &plugins {
                            let id = plugin["id"].as_str().unwrap_or_default();
                            ui.push_id(id, |ui| {
                                ui.label(plugin["name"].as_str().unwrap_or(id));
                                let enabled = plugin["enabled"].as_bool().unwrap_or(false);
                                let allowed = plugin["allowed"].as_bool().unwrap_or(false);
                                if ui
                                    .add_enabled(
                                        connected && allowed,
                                        egui::Button::new(if enabled {
                                            "Stop plugin"
                                        } else {
                                            "Start plugin"
                                        }),
                                    )
                                    .clicked()
                                {
                                    self.send(
                                        if enabled {
                                            "plugins.stop"
                                        } else {
                                            "plugins.start"
                                        },
                                        json!({"id": id}),
                                    );
                                }
                                if !allowed {
                                    ui.small("Hardware disabled for this daemon.");
                                }
                                if let Some(error) = plugin["last_error"].as_str() {
                                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                                }
                            });
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.panels.page != 0 {
                let snapshot = self.state.lock().unwrap_or_else(|e| e.into_inner());
                self.panels.draw(ui, &self.tx, &snapshot, &mut self.toasts);
                return;
            }
            ui.add_enabled_ui(connected, |ui| {
                ui.heading("Effect");
                ui.small(
                    "Live preview, rendered locally with the effect engine (audio and screen effects stay static).",
                );
                self.preview.show(ui, &self.spec);
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
                                self.toasts.success("Custom effect applied");
                            }
                            if ui.add_enabled(sel.is_some(), egui::Button::new("Delete")).clicked() {
                                self.send("custom_effects.delete", json!({"name": sel}));
                                self.selected_custom = None;
                                self.toasts.success("Custom effect deleted");
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
                                    self.toasts.success("Custom effect saved");
                                }
                            }
                        });
                        ui.label("Tip: choose \"Custom (layers)\" above to build an effect from several layers.");
                    });
                }

                ui.separator();
                ui.collapsing("Keyboard reactions", |ui| {
                    let snapshot = self.state.lock().unwrap_or_else(|e|e.into_inner());
                    self.keyboard_panel.show(ui, &snapshot, &self.tx)
                });
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
                ui.label("Where each device sits on the effect canvas. Drag to move, drag the corner to resize, right-click to reset to the full canvas.");
                let (layout_changed, layout_reset) =
                    layout_editor(ui, &devices, &mut self.device_regions);
                if let Some((id, r)) = layout_changed {
                    self.send(
                        "devices.set_region",
                        json!({"id": id, "x": r.x, "y": r.y, "w": r.w, "h": r.h}),
                    );
                }
                if let Some(id) = layout_reset {
                    let mut params = serde_json::to_value(Region::FULL).unwrap_or(Value::Null);
                    params["id"] = json!(id);
                    self.send("devices.set_region", params);
                }

                ui.separator();
                ui.heading("Profiles");
                let cooling: Vec<String> = self.state.lock().unwrap_or_else(|e| e.into_inner()).control["cooling_profiles"]["profiles"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| p["name"].as_str().map(str::to_owned))
                    .collect();
                self.app_profiles.ui(ui, profiles.as_deref(), &cooling);
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
                                self.app_profiles.disable();
                                self.sync_after_generation = Some(generation);
                                self.toasts.success("Profile loaded");
                            }
                            if ui
                                .add_enabled(sel.is_some(), egui::Button::new("Delete"))
                                .clicked()
                            {
                                self.send("profiles.delete", json!({"name": sel}));
                                self.toasts.success("Profile deleted");
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
                                self.toasts.success("Profile saved");
                            }
                        });
                    }
                }

            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("freeRGB")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!(
                    "../../../assets/icons/freergb.png"
                ))
                .expect("embedded project icon"),
            )
            .with_inner_size([760.0, 460.0]),
        ..Default::default()
    };
    eframe::run_native(
        "freeRGB",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(test)]
mod command_tests {
    use super::*;
    #[test]
    fn save_actions_are_barriers_and_macro_actions_are_never_coalesced() {
        let mut pending = Vec::new();
        push_command(
            &mut pending,
            Command {
                method: "brightness.set",
                params: json!({"value":0.2}),
            },
        );
        push_command(
            &mut pending,
            Command {
                method: "profiles.save",
                params: json!({"name":"Dim"}),
            },
        );
        push_command(
            &mut pending,
            Command {
                method: "brightness.set",
                params: json!({"value":0.8}),
            },
        );
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].params["value"], 0.2);
        assert_eq!(pending[1].method, "profiles.save");
        push_command(
            &mut pending,
            Command {
                method: "macros.run",
                params: json!({"name":"A"}),
            },
        );
        push_command(
            &mut pending,
            Command {
                method: "macros.run",
                params: json!({"name":"B"}),
            },
        );
        assert_eq!(pending.len(), 5);
        assert_eq!(pending[3].params["name"], "A");
        assert_eq!(pending[4].params["name"], "B");
    }
    #[test]
    fn slider_bursts_merge_per_device_after_the_last_action() {
        let mut pending = Vec::new();
        for (id, value) in [("left", 0.1), ("right", 0.2), ("left", 0.9)] {
            push_command(
                &mut pending,
                Command {
                    method: "devices.set_brightness",
                    params: json!({"id":id,"value":value}),
                },
            );
        }
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].params["id"], "right");
        assert_eq!(pending[1].params["id"], "left");
        assert_eq!(pending[1].params["value"], 0.9);
    }
}
