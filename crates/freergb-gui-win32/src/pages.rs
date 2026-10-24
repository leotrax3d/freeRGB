//! Page layouts for the Win32 GUI. Each page mirrors its egui Classic page and
//! sends exactly the same JSON-RPC methods; only the widgets differ. The
//! layer effect editor, sensor alert rules, application profiles and global
//! macro shortcuts remain exclusive to the Classic (egui) and desktop GUIs.

use crate::ffi::{COLORREF, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_RIGHT, DT_SINGLELINE};
use crate::rpc::{Command, Snapshot};
use crate::theme::{self, Font};
use crate::ui::{self, Ui};
use freergb_control::{CurvePoint, FanCurve, MacroDefinition};
use freergb_core::{Region, Rgb};
use freergb_effects::EffectSpec;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const PAGES: [(&str, u8); 5] = [
    ("Lighting", 0),
    ("Cooling", 1),
    ("Macros", 2),
    ("Monitoring", 3),
    ("Updates", 4),
];

/// Display name of an effect (same labels as the egui Classic GUI).
pub fn effect_label(name: &str) -> &'static str {
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

fn send(tx: &Sender<Command>, method: &'static str, params: Value) {
    let _ = tx.send(Command { method, params });
}

fn config_root() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("freeRGB"))
}

// ---- update worker (same files and cadence as the egui Classic GUI) ----

#[derive(Default, Clone)]
struct UpdateView {
    message: String,
    update: Option<freergb_update::Update>,
    downloaded: Option<PathBuf>,
    busy: bool,
    automatic: bool,
}

enum UpdateCommand {
    Check,
    Download,
    Automatic(bool),
}

fn spawn_update_worker() -> (Arc<Mutex<UpdateView>>, Sender<UpdateCommand>) {
    let state = Arc::new(Mutex::new(UpdateView::default()));
    let (tx, rx) = std::sync::mpsc::channel();
    let worker = state.clone();
    std::thread::spawn(move || {
        let root = config_root();
        let preference = root.as_ref().map(|r| r.join("automatic-updates.json"));
        let mut automatic = preference
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<bool>(&s).ok())
            .unwrap_or(false);
        worker.lock().unwrap_or_else(|e| e.into_inner()).automatic = automatic;
        let mut initial = automatic;
        loop {
            let command = if initial {
                initial = false;
                UpdateCommand::Check
            } else {
                match rx.recv_timeout(Duration::from_secs(6 * 3600)) {
                    Ok(command) => command,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) if automatic => UpdateCommand::Check,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => break,
                }
            };
            if let UpdateCommand::Automatic(value) = command {
                if let Some(path) = &preference {
                    let result = std::fs::create_dir_all(path.parent().unwrap()).and_then(|()| {
                        std::fs::write(path, if value { "true" } else { "false" })
                    });
                    if let Err(error) = result {
                        worker.lock().unwrap_or_else(|e| e.into_inner()).message = error.to_string();
                        continue;
                    }
                }
                automatic = value;
                worker.lock().unwrap_or_else(|e| e.into_inner()).automatic = value;
                if value {
                    initial = true;
                }
                continue;
            }
            worker.lock().unwrap_or_else(|e| e.into_inner()).busy = true;
            match command {
                UpdateCommand::Check => {
                    let result = freergb_update::check();
                    let mut view = worker.lock().unwrap_or_else(|e| e.into_inner());
                    view.downloaded = None;
                    match result {
                        Ok(update) => {
                            view.message = if update.is_some() {
                                "Update available".into()
                            } else {
                                "Up to date".into()
                            };
                            view.update = update;
                        }
                        Err(error) => {
                            view.message = error;
                            view.update = None;
                        }
                    }
                }
                UpdateCommand::Download => {
                    let update = worker
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .update
                        .clone();
                    if let Some(update) = update {
                        let directory = root.clone().unwrap_or_else(std::env::temp_dir).join("updates");
                        let result = freergb_update::download(&update, &directory);
                        let mut view = worker.lock().unwrap_or_else(|e| e.into_inner());
                        match result {
                            Ok(path) => {
                                view.downloaded = Some(path);
                                view.message = "Installer downloaded and SHA-256 verified".into();
                            }
                            Err(error) => view.message = error,
                        }
                    }
                }
                _ => {}
            }
            let should_download = {
                let mut view = worker.lock().unwrap_or_else(|e| e.into_inner());
                let download = automatic
                    && matches!(command, UpdateCommand::Check)
                    && view.update.is_some();
                view.busy = download;
                download
            };
            if should_download {
                let update = worker
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .update
                    .clone()
                    .unwrap();
                let directory = root.clone().unwrap_or_else(std::env::temp_dir).join("updates");
                let result = freergb_update::download(&update, &directory);
                let mut view = worker.lock().unwrap_or_else(|e| e.into_inner());
                view.busy = false;
                match result {
                    Ok(path) => {
                        view.downloaded = Some(path);
                        view.message = "Update downloaded; ready to install".into();
                    }
                    Err(error) => view.message = error,
                }
            }
        }
    });
    (state, tx)
}

// ---- cached sensor histories for the monitoring graphs ----

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// Kept for time-based plots (parity with the egui history model);
    /// series today space samples evenly.
    #[allow(dead_code)]
    pub at_s: f64,
    pub value: Option<f64>,
}

#[derive(Default)]
pub struct Histories {
    series: HashMap<(String, String), VecDeque<Sample>>,
    last_seen: HashMap<(String, String), f64>,
}

impl Histories {
    const CAPACITY: usize = 600;
    const MAX_SENSORS: usize = 128;

    pub fn observe(&mut self, devices: &[Value], now_s: f64) {
        let mut current = HashMap::new();
        for device in devices {
            let Some(id) = device["info"]["id"].as_str() else {
                continue;
            };
            let Some(sensors) = device["sensors"].as_array() else {
                continue;
            };
            for sensor in sensors.iter().take(Self::MAX_SENSORS) {
                let Some(sensor_id) = sensor["id"].as_str() else {
                    continue;
                };
                let key = (id.to_owned(), sensor_id.to_owned());
                if !self.series.contains_key(&key) && self.series.len() >= Self::MAX_SENSORS {
                    continue;
                }
                self.series.entry(key.clone()).or_default();
                self.last_seen.insert(key.clone(), now_s);
                let fresh = device["status"] == "Connected"
                    && sensor["age_ms"].as_u64().is_some_and(|age| age <= 3000);
                current.insert(
                    key,
                    sensor["value"].as_f64().filter(|v| fresh && v.is_finite()),
                );
            }
        }
        self.series.retain(|key, _| {
            self.last_seen
                .get(key)
                .is_some_and(|seen| now_s - seen <= 60.0)
        });
        self.last_seen.retain(|key, _| self.series.contains_key(key));
        for series in self.series.values_mut() {
            if series.len() == Self::CAPACITY {
                series.pop_front();
            }
            series.push_back(Sample {
                at_s: now_s,
                value: None,
            });
        }
        for (key, value) in current {
            if let Some(series) = self.series.get_mut(&key) {
                series.back_mut().unwrap().value = value;
            }
        }
    }
}

// ---- live motherboard cooling (experimental; same RPC as Classic) ----

#[derive(Default)]
pub struct Mainboard {
    control: String,
    rpm: String,
    temperature: String,
    confirmed: bool,
    manual_duty: f32,
    profile: String,
    profile_name: String,
    /// Typed confirmation for unlocking untested channels.
    confirmation: String,
    /// Whether the last job report is expanded.
    show_report: bool,
}

pub fn tier_label(tier: &str) -> &'static str {
    match tier {
        "verified" => "Verified",
        "lhm_supported" => "LHM-supported",
        "unrestricted" => "Unrestricted",
        "read_only" => "Read-only",
        _ => "Unknown",
    }
}

fn channel_tier(channel: &Value) -> &'static str {
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

fn refusal(channel: &Value) -> Option<&'static str> {
    if channel["support"]["software_mode"] == true {
        Some("Another program already controls this header (software mode); freeRGB will not take it over.")
    } else if channel["support"]["held_full"] == true {
        Some("Held at 100% after a worker crash. Restart Windows to return it to firmware control.")
    } else {
        None
    }
}

// ---- all mutable page state ----

pub struct Pages {
    pub page: u8,
    pub scroll: f32,
    pub content_height: f32,
    pub devices_scroll: f32,
    pub devices_height: f32,
    /// Short transient feedback shown under the header.
    pub note: Option<(String, Instant)>,
    pub started: Instant,
    // Lighting
    pub spec: EffectSpec,
    pub brightness: f32,
    pub device_brightness: HashMap<String, f32>,
    pub device_regions: HashMap<String, Region>,
    layout_drag: Option<(String, u8)>,
    pub selected_custom: Option<usize>,
    pub custom_name: String,
    pub selected_profile: Option<usize>,
    pub profile_name: String,
    pub preview: crate::preview::Preview,
    pub sync_after_generation: Option<u64>,
    pub profile_revision: u64,
    // Cooling
    pub curve: FanCurve,
    pub curve_name: String,
    curve_drag: Option<usize>,
    pub temperature: f32,
    pub mainboard: Mainboard,
    // Macros
    pub macro_name: String,
    pub macro_json: String,
    // Lighting layout editor drag state: (device id, 0 = move / 1 = corner).
    layout_last: (f32, f32),
    // Monitoring
    pub history: Histories,
    history_generation: u64,
    last_snapshot_s: f64,
    pub show_diagnostics: bool,
    // Updates
    update_view: Arc<Mutex<UpdateView>>,
    update_tx: Sender<UpdateCommand>,
}

impl Default for Pages {
    fn default() -> Self {
        let (update_view, update_tx) = spawn_update_worker();
        Self {
            page: 0,
            scroll: 0.0,
            content_height: 0.0,
            devices_scroll: 0.0,
            devices_height: 0.0,
            note: None,
            started: Instant::now(),
            spec: EffectSpec::default_for("rainbow_wave").expect("known effect"),
            brightness: 1.0,
            device_brightness: HashMap::new(),
            device_regions: HashMap::new(),
            layout_drag: None,
            selected_custom: None,
            custom_name: String::new(),
            selected_profile: None,
            profile_name: String::new(),
            preview: Default::default(),
            sync_after_generation: Some(0),
            profile_revision: 0,
            curve: FanCurve::default(),
            curve_name: "Balanced".into(),
            curve_drag: None,
            temperature: 40.0,
            mainboard: Mainboard {
                manual_duty: 60.0,
                ..Default::default()
            },
            layout_last: (0.0, 0.0),
            macro_name: "RGB demo".into(),
            macro_json: "{\n  \"repeat\": 1,\n  \"steps\": [\n    {\"type\": \"color\", \"color\": {\"r\": 255, \"g\": 0, \"b\": 0}},\n    {\"type\": \"delay\", \"ms\": 1000},\n    {\"type\": \"color\", \"color\": {\"r\": 0, \"g\": 100, \"b\": 255}}\n  ]\n}".into(),
            history: Histories::default(),
            history_generation: 0,
            last_snapshot_s: 0.0,
            show_diagnostics: false,
            update_view,
            update_tx,
        }
    }
}

impl Pages {
    /// Pulls effect and brightness from the daemon state (same sync rules as
    /// the egui Classic GUI).
    pub fn sync_from(&mut self, effect: &Value, diag: Option<&Value>) {
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

    pub fn note(&mut self, text: impl Into<String>) {
        self.note = Some((text.into(), Instant::now()));
    }

    fn observe(&mut self, snapshot: &Snapshot) {
        let now = self.started.elapsed().as_secs_f64();
        if snapshot.generation != self.history_generation {
            self.last_snapshot_s = now;
            self.history_generation = snapshot.generation;
            self.history.observe(&snapshot.devices, now);
        }
        let _ = self.last_snapshot_s;
    }
}

// ---- frame entry ----

pub fn draw(ui: &mut Ui, pages: &mut Pages, snapshot: &Snapshot, tx: &Sender<Command>) {
    pages.observe(snapshot);
    let generation = snapshot.generation;
    if pages.profile_revision != snapshot.profile_revision {
        if let Some(effect) = &snapshot.effect {
            pages.sync_from(effect, snapshot.diagnostics.as_ref());
            pages.profile_revision = snapshot.profile_revision;
            pages.sync_after_generation = None;
        }
    }
    if let (Some(after), Some(effect)) = (pages.sync_after_generation, snapshot.effect.clone()) {
        if generation > after {
            pages.sync_from(&effect, snapshot.diagnostics.as_ref());
            pages.sync_after_generation = None;
        }
    }
    match pages.page {
        1 => cooling(ui, pages, snapshot, tx),
        2 => macros(ui, pages, snapshot, tx),
        3 => monitoring(ui, pages, snapshot, tx),
        4 => updates(ui, pages),
        _ => lighting(ui, pages, snapshot, tx),
    }
    pages.content_height = ui.cursor.1;
}

fn unreachable_hint(ui: &mut Ui) {
    let color = theme::BAD;
    ui.label_color(
        "Daemon not reachable — start freergbd to edit this page.",
        Font::Strong,
        color,
    );
}

// ---- shared helpers ----

/// Horizontal button row, mirroring the egui GUI's `ui.horizontal` look:
/// place widgets left to right with [`Row::next`], then reserve the row's
/// height once with [`Row::end`].
struct Row {
    x: f32,
    y: f32,
}

impl Row {
    fn start(ui: &Ui) -> Self {
        Row { x: ui.bounds.0, y: ui.cursor.1 }
    }

    fn next(&mut self, width: f32, height: f32) -> ui::Rect {
        let rect = ui::Rect::new(self.x, self.y, width, height);
        self.x += width + 8.0;
        rect
    }

    fn end(self, ui: &mut Ui, height: f32) {
        ui.advance(height + 8.0);
    }
}

/// Colour for plain GDI fills (0x00BBGGRR) from an sRGB colour.
pub fn colorref(c: Rgb) -> COLORREF {
    theme::rgb(c.r, c.g, c.b)
}

/// Colour editor: quick picks on one row, swatch plus channel sliders.
/// Returns `true` on any change.
fn color_edit(
    ui: &mut Ui,
    page: &'static str,
    name: &'static str,
    hint: &'static str,
    c: &mut Rgb,
) -> bool {
    let mut changed = false;
    ui.hint(hint);
    const PRESETS: [(&str, Rgb); 5] = [
        ("Red", Rgb::RED),
        ("Green", Rgb::GREEN),
        ("Blue", Rgb::BLUE),
        ("White", Rgb::WHITE),
        ("Off", Rgb::BLACK),
    ];
    let preset_h = 24.0;
    let preset_w = 74.0;
    let y = ui.cursor.1;
    for (i, (label, preset)) in PRESETS.iter().enumerate() {
        let rect = ui::Rect::new(
            ui.bounds.0 + i as f32 * (preset_w + 8.0),
            y,
            preset_w,
            preset_h,
        );
        let id = ui::id(&[page, name, "preset", label]);
        if ui.button_at(id, &rect, label, c.to_array() == preset.to_array()) && *c != *preset {
            *c = *preset;
            changed = true;
        }
    }
    ui.advance(preset_h + 8.0);
    // Swatch beside the three channel sliders.
    let swatch = ui::Rect::new(ui.bounds.0, ui.cursor.1, 34.0, 44.0);
    let (x0, x1) = (ui.bounds.0, ui.bounds.1);
    ui.set_bounds(x0 + 42.0, x1);
    ui.set_cursor(x0 + 42.0, ui.cursor.1 + 2.0);
    let mut arr = c.to_array();
    for (label, index) in [("R", 0usize), ("G", 1), ("B", 2)] {
        let mut value = f32::from(arr[index]);
        let display = format!("{}", value.round() as u32);
        if ui.slider(
            ui::id(&[page, name, "channel", label]),
            &mut value,
            0.0,
            255.0,
            label,
            &display,
        ) {
            arr[index] = value.round() as u8;
            changed = true;
        }
    }
    if changed {
        *c = Rgb::from(arr);
    }
    ui.box_(&swatch, Some((colorref(*c), colorref(*c))), Some(ui.palette.border));
    ui.set_cursor(x0, ui.cursor.1);
    ui.set_bounds(x0, x1);
    changed
}

/// Parameter controls for the current effect. Returns `true` on any change.
fn effect_params(ui: &mut Ui, spec: &mut EffectSpec) -> bool {
    let page = "lighting";
    let mut changed = false;
    let period = |ui: &mut Ui, id: u64, ms: &mut u32, range: (f32, f32)| -> bool {
        let mut value = *ms as f32;
        let display = format!("{} ms", value.round() as u32);
        if ui.slider(
            id,
            &mut value,
            range.0,
            range.1,
            "period (ms)",
            &display,
        ) {
            *ms = value.round() as u32;
            true
        } else {
            false
        }
    };
    match spec {
        EffectSpec::Solid { color } => {
            changed = color_edit(ui, page, "solid", "Colour", color);
        }
        EffectSpec::RainbowWave { speed, scale } | EffectSpec::RadialRainbow { speed, scale } => {
            changed |= ui.slider(
                ui::id(&[page, "speed"]),
                speed,
                -2.0,
                2.0,
                "speed",
                &format!("{speed:.2}"),
            );
            changed |= ui.slider(
                ui::id(&[page, "scale"]),
                scale,
                0.1,
                5.0,
                "cycles",
                &format!("{scale:.2}"),
            );
        }
        EffectSpec::Breathing { color, period_ms } => {
            changed |= color_edit(ui, page, "breathing", "Colour", color);
            changed |= period(ui, ui::id(&[page, "period"]), period_ms, (500.0, 10_000.0));
        }
        EffectSpec::Gradient {
            from,
            to,
            angle_deg,
        } => {
            changed |= color_edit(ui, page, "gradient-from", "From", from);
            changed |= color_edit(ui, page, "gradient-to", "To", to);
            changed |= ui.slider(
                ui::id(&[page, "angle"]),
                angle_deg,
                0.0,
                360.0,
                "angle (°)",
                &format!("{:.0} °", angle_deg.round()),
            );
        }
        EffectSpec::ColorCycle { period_ms } => {
            changed |= period(ui, ui::id(&[page, "period"]), period_ms, (1_000.0, 60_000.0));
        }
        EffectSpec::Scanner {
            color,
            background,
            period_ms,
            width,
        } => {
            changed |= color_edit(ui, page, "scanner-color", "Colour", color);
            changed |= color_edit(ui, page, "scanner-background", "Background", background);
            changed |= period(ui, ui::id(&[page, "period"]), period_ms, (300.0, 10_000.0));
            changed |= ui.slider(
                ui::id(&[page, "width"]),
                width,
                0.02,
                0.5,
                "width",
                &format!("{width:.2}"),
            );
        }
        EffectSpec::Twinkle {
            color,
            background,
            density,
            period_ms,
        } => {
            changed |= color_edit(ui, page, "twinkle-color", "Colour", color);
            changed |= color_edit(ui, page, "twinkle-background", "Background", background);
            changed |= ui.slider(
                ui::id(&[page, "density"]),
                density,
                0.0,
                1.0,
                "density",
                &format!("{density:.2}"),
            );
            changed |= period(ui, ui::id(&[page, "period"]), period_ms, (200.0, 5_000.0));
        }
        EffectSpec::AudioPulse {
            color,
            background,
            sensitivity,
        } => {
            changed |= color_edit(ui, page, "pulse-color", "Colour", color);
            changed |= color_edit(ui, page, "pulse-background", "Background", background);
            changed |= ui.slider(
                ui::id(&[page, "sensitivity"]),
                sensitivity,
                0.5,
                4.0,
                "sensitivity",
                &format!("{sensitivity:.2}"),
            );
            ui.hint("Reacts to the sound your PC plays.");
        }
        EffectSpec::AudioSpectrum {
            low,
            high,
            background,
        } => {
            changed |= color_edit(ui, page, "spectrum-low", "Bass colour", low);
            changed |= color_edit(ui, page, "spectrum-high", "Treble colour", high);
            changed |= color_edit(ui, page, "spectrum-background", "Background", background);
            ui.hint("8 frequency bands across the canvas, bass on the left.");
        }
        EffectSpec::ScreenAmbience { saturation } => {
            changed |= ui.slider(
                ui::id(&[page, "saturation"]),
                saturation,
                0.5,
                2.5,
                "saturation",
                &format!("{saturation:.2}"),
            );
            ui.hint("Mirrors the primary monitor. Place devices in the Layout section to match where they sit around the screen.");
        }
        EffectSpec::Layers { .. } => {
            ui.hint("Layer editing stays exclusive to the Classic (egui) GUI; the Win32 GUI applies layered effects but cannot edit them.");
        }
    }
    changed
}

fn lighting(ui: &mut Ui, pages: &mut Pages, snapshot: &Snapshot, tx: &Sender<Command>) {
    ui.heading("Effect");
    ui.hint("Live preview, rendered locally with the effect engine (audio and screen effects stay static).");
    let width = ui.column_width().clamp(120.0, 420.0);
    let rect = ui.item(width, width * 32.0 / 128.0);
    ui.box_(&rect, Some((0x000000, 0x000000)), Some(ui.palette.border));
    let spec = pages.spec.clone();
    pages.preview.blit(ui.dc, ui.scale, &rect.with_margin(2.0), &spec);

    let items: Vec<String> = EffectSpec::NAMES
        .iter()
        .map(|name| effect_label(name).to_owned())
        .collect();
    let current = pages.spec.name();
    let selected = EffectSpec::NAMES.iter().position(|name| *name == current);
    if let Some(choice) = ui.combo(ui::id(&["lighting", "effect"]), &items, selected, "(choose an effect)") {
        let name = EffectSpec::NAMES[choice];
        if name != current {
            if let Some(s) = EffectSpec::default_for(name) {
                pages.spec = s;
                if snapshot.connected {
                    if let Ok(value) = serde_json::to_value(&pages.spec) {
                        send(tx, "effects.set", value);
                    }
                }
            }
        }
    }
    let mut changed = false;
    changed |= effect_params(ui, &mut pages.spec);
    if changed && snapshot.connected {
        if let Ok(value) = serde_json::to_value(&pages.spec) {
            send(tx, "effects.set", value);
        }
    }

    // Saved custom effects (same methods as the Classic GUI).
    if let Some(list) = &snapshot.custom_effects {
        ui.strong("My effects");
        let index = pages.selected_custom.filter(|i| *i < list.len());
        if let Some(choice) = ui.combo(ui::id(&["lighting", "custom"]), list, index, "(choose)") {
            pages.selected_custom = Some(choice);
        }
        let selected_name = pages.selected_custom.and_then(|i| list.get(i).cloned());
        if let Some(name) = selected_name.clone() {
            let mut row = Row::start(ui);
            if ui.button_at_enabled(
                ui::id(&["lighting", "custom-apply"]),
                &row.next(110.0, 30.0),
                "Apply",
                true,
                snapshot.connected,
            ) {
                send(tx, "gui.apply_custom_effect", json!({"name": name}));
                pages.sync_after_generation = Some(snapshot.generation);
                pages.note("Custom effect applied");
            }
            if ui.button_at_enabled(
                ui::id(&["lighting", "custom-delete"]),
                &row.next(110.0, 30.0),
                "Delete",
                false,
                snapshot.connected,
            ) {
                send(tx, "custom_effects.delete", json!({"name": name}));
                pages.selected_custom = None;
                pages.note("Custom effect deleted");
            }
            row.end(ui, 30.0);
        }
        ui.text_line(ui::id(&["lighting", "custom-name"]), &mut pages.custom_name, "name for the current effect");
        let name = pages.custom_name.trim().to_owned();
        {
            let mut row = Row::start(ui);
            if ui.button_at_enabled(
                ui::id(&["lighting", "custom-save"]),
                &row.next(110.0, 30.0),
                "Save",
                true,
                snapshot.connected && !name.is_empty(),
            ) {
                if let Ok(value) = serde_json::to_value(&pages.spec) {
                    send(tx, "custom_effects.save", json!({"name": name, "effect": value}));
                    pages.selected_custom = None;
                    pages.custom_name.clear();
                    pages.note("Custom effect saved");
                }
            }
            row.end(ui, 30.0);
        }
        ui.hint("Tip: choose \"Custom (layers)\" above to build an effect from several layers.");
    }

    ui.separator();
    ui.strong("Brightness");
    let mut value = pages.brightness;
    let display = format!("{:.0} %", value * 100.0);
    if ui.slider(
        ui::id(&["lighting", "brightness"]),
        &mut value,
        0.0,
        1.0,
        "",
        &display,
    ) {
        pages.brightness = value;
        if snapshot.connected {
            send(tx, "brightness.set", json!({"value": pages.brightness}));
        }
    }

    ui.separator();
    ui.strong("Layout");
    ui.hint("Where each device sits on the effect canvas. Drag to move, drag the corner to resize, right-click to reset to the full canvas.");
    let (moved, reset) = layout_editor(ui, pages, &snapshot.devices);
    if snapshot.connected {
        if let Some((id, region)) = moved {
            send(
                tx,
                "devices.set_region",
                json!({"id": id, "x": region.x, "y": region.y, "w": region.w, "h": region.h}),
            );
        }
        if let Some(id) = reset {
            send(
                tx,
                "devices.set_region",
                json!({"id": id, "x": 0.0, "y": 0.0, "w": 1.0, "h": 1.0}),
            );
        }
    }

    ui.separator();
    ui.strong("Profiles");
    match &snapshot.profiles {
        None => {
            ui.hint("Profiles are disabled (daemon started with --no-state).");
        }
        Some(list) => {
            if pages
                .selected_profile
                .is_some_and(|i| i >= list.len())
            {
                pages.selected_profile = None;
            }
            if let Some(choice) = ui.combo(
                ui::id(&["lighting", "profiles"]),
                list,
                pages.selected_profile,
                "(choose)",
            ) {
                pages.selected_profile = Some(choice);
            }
            let selected_name = pages.selected_profile.and_then(|i| list.get(i).cloned());
            if let Some(name) = selected_name.clone() {
                let mut row = Row::start(ui);
                if ui.button_at_enabled(
                    ui::id(&["lighting", "profile-load"]),
                    &row.next(110.0, 30.0),
                    "Load",
                    true,
                    snapshot.connected,
                ) {
                    send(tx, "profiles.load", json!({"name": name}));
                    pages.sync_after_generation = Some(snapshot.generation);
                    pages.note("Profile loaded");
                }
                if ui.button_at_enabled(
                    ui::id(&["lighting", "profile-delete"]),
                    &row.next(110.0, 30.0),
                    "Delete",
                    false,
                    snapshot.connected,
                ) {
                    send(tx, "profiles.delete", json!({"name": name}));
                    pages.note("Profile deleted");
                }
                row.end(ui, 30.0);
            }
            ui.text_line(
                ui::id(&["lighting", "profile-name"]),
                &mut pages.profile_name,
                "new profile name",
            );
            let name = pages.profile_name.trim().to_owned();
            {
                let mut row = Row::start(ui);
                if ui.button_at_enabled(
                    ui::id(&["lighting", "profile-save"]),
                    &row.next(150.0, 30.0),
                    "Save current",
                    true,
                    snapshot.connected && !name.is_empty(),
                ) {
                    send(tx, "profiles.save", json!({"name": name}));
                    pages.selected_profile = None;
                    pages.profile_name.clear();
                    pages.note("Profile saved");
                }
                row.end(ui, 30.0);
            }
        }
    }
}

// ---- lighting layout editor ----

const LAYOUT_COLORS: [COLORREF; 6] = [
    theme::rgb(90, 160, 250),
    theme::rgb(250, 140, 70),
    theme::rgb(120, 210, 120),
    theme::rgb(220, 110, 220),
    theme::rgb(240, 210, 80),
    theme::rgb(110, 210, 210),
];

/// Canvas layout editor: every device is a rectangle; drag to move, drag the
/// corner to resize, right-click to reset. Returns `(moved, reset)` like the
/// egui editor: the region to send on drag end, and the id to reset.
fn layout_editor(
    ui: &mut Ui,
    pages: &mut Pages,
    devices: &[Value],
) -> (Option<(String, Region)>, Option<String>) {
    let mut moved = None;
    let mut reset = None;
    let width = ui.column_width().min(480.0);
    let outer = ui.item(width, width * 0.625);
    let claim = ui::id(&["lighting", "layout"]);
    for (i, device) in devices.iter().enumerate() {
        let Some(id) = device
            .get("info")
            .and_then(|info| info.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            continue;
        };
        let name = device
            .get("info")
            .and_then(|info| info.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("?");
        let reported: Region = device
            .get("region")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or(Region::FULL);
        let synced = |local: &Region| {
            (local.x - reported.x).abs() < 0.002
                && (local.y - reported.y).abs() < 0.002
                && (local.w - reported.w).abs() < 0.002
                && (local.h - reported.h).abs() < 0.002
        };
        if pages.device_regions.get(&id).is_some_and(synced) {
            pages.device_regions.remove(&id);
        }
        let mut region = pages.device_regions.get(&id).copied().unwrap_or(reported);
        let rect = ui::Rect::new(
            outer.x + region.x * outer.w,
            outer.y + region.y * outer.h,
            (region.w * outer.w).max(8.0),
            (region.h * outer.h).max(8.0),
        );
        let corner = ui::Rect::new(rect.right() - 10.0, rect.bottom() - 10.0, 10.0, 10.0);
        if ui.input.just_down && ui.widgets.press.is_none() && ui.contains(&corner) {
            pages.layout_drag = Some((id.clone(), 1));
            pages.layout_last = ui.input.mouse;
            ui.widgets.press = Some(claim);
        } else if ui.input.just_down && ui.widgets.press.is_none() && ui.contains(&rect) {
            pages.layout_drag = Some((id.clone(), 0));
            pages.layout_last = ui.input.mouse;
            ui.widgets.press = Some(claim);
        }
        if ui.widgets.press == Some(claim) {
            if let Some((drag_id, mode)) = pages.layout_drag.as_ref() {
                if *drag_id == id {
                    let dx = ui.input.mouse.0 - pages.layout_last.0;
                    let dy = ui.input.mouse.1 - pages.layout_last.1;
                    pages.layout_last = ui.input.mouse;
                    if *mode == 1 {
                        region.w = (region.w + dx / outer.w).clamp(0.02, 1.0 - region.x);
                        region.h = (region.h + dy / outer.h).clamp(0.02, 1.0 - region.y);
                    } else {
                        region.x = (region.x + dx / outer.w).clamp(0.0, 1.0 - region.w);
                        region.y = (region.y + dy / outer.h).clamp(0.0, 1.0 - region.h);
                    }
                    pages.device_regions.insert(id.clone(), region);
                }
            }
        }
        if ui.input.just_up && ui.widgets.press == Some(claim) {
            if let Some((drag_id, _)) = pages.layout_drag.as_ref() {
                if *drag_id == id {
                    moved = Some((id.clone(), region));
                }
            }
            pages.layout_drag = None;
        }
        if ui.input.right_released && ui.contains(&rect) && ui.widgets.press != Some(claim) {
            reset = Some(id.clone());
            pages.device_regions.remove(&id);
        }
        let color = LAYOUT_COLORS[i % LAYOUT_COLORS.len()];
        ui.box_(&rect, Some((color, color)), Some(ui.palette.border));
        let knob = ui::Rect::new(rect.right() - 8.0, rect.bottom() - 8.0, 8.0, 8.0);
        ui.fill(&knob, color);
        ui.text(
            &ui::Rect::new(rect.x + 3.0, rect.y + 1.0, (rect.w - 6.0).max(10.0), 14.0),
            name,
            Font::Small,
            ui.palette.text_inv,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }
    (moved, reset)
}

// ---- cooling (page 1) ----

/// Temperature/duty axes for the curve graph; matches the ranges the
/// sliders allow (`-20..=110 °C`).
const T_MIN: f32 = -20.0;
const T_MAX: f32 = 110.0;
/// Mouse distance (logical px) at which a graph point counts as picked.
const PICK_RADIUS: f32 = 18.0;

/// Interactive curve graph: drag points, double-click to add, right-click to
/// remove. The editing rules (`clamp_moved_point`/`insert_curve_point`) live
/// on `FanCurve`, so both GUIs share exactly the same geometry.
fn curve_graph(ui: &mut Ui, pages: &mut Pages) {
    let outer = ui.item(ui.column_width().min(620.0), 210.0);
    let rect = ui::Rect::new(outer.x, outer.y, outer.w, outer.h - 16.0);
    let id = ui::id(&["cooling", "curve-graph"]);
    if ui.input.just_down && ui.contains(&rect) && ui.widgets.press.is_none() {
        ui.widgets.press = Some(id);
        pages.curve_drag = None;
    }
    let to_pos = |t: f32, d: f32| -> (f32, f32) {
        (
            rect.x + ((t - T_MIN) / (T_MAX - T_MIN)).clamp(0.0, 1.0) * rect.w,
            rect.bottom() - (d.clamp(0.0, 100.0) / 100.0) * rect.h,
        )
    };
    let t_at = |x: f32| T_MIN + ((x - rect.x).clamp(0.0, rect.w) / rect.w) * (T_MAX - T_MIN);
    let d_at = |y: f32| ((rect.bottom() - y).clamp(0.0, rect.h) / rect.h) * 100.0;

    for t in (0..=100).step_by(20) {
        let (x, _) = to_pos(t as f32, 0.0);
        ui.line(x, rect.y, x, rect.bottom(), ui.palette.grid, 1);
        ui.text(
            &ui::Rect::new(x - 22.0, rect.bottom() + 2.0, 44.0, 12.0),
            &format!("{t}"),
            Font::Small,
            ui.palette.text_dim,
            DT_CENTER | DT_SINGLELINE,
        );
    }
    for d in (0..=100).step_by(25) {
        let (_, y) = to_pos(0.0, d as f32);
        ui.line(rect.x, y, rect.right(), y, ui.palette.grid, 1);
        ui.text(
            &ui::Rect::new(rect.x + 2.0, y - 13.0, 42.0, 12.0),
            &format!("{d}%"),
            Font::Small,
            ui.palette.text_dim,
            DT_LEFT | DT_SINGLELINE,
        );
    }
    // Minimum duty band and the critical temperature marker.
    let (_, min_y) = to_pos(0.0, pages.curve.minimum_percent);
    ui.fill(
        &ui::Rect::new(rect.x, min_y, rect.w, rect.bottom().max(min_y) - min_y),
        ui.palette.accent_dim,
    );
    ui.line(rect.x, min_y, rect.right(), min_y, ui.palette.accent, 1);
    if (T_MIN..=T_MAX).contains(&pages.curve.critical_c) {
        let (x, _) = to_pos(pages.curve.critical_c, 0.0);
        ui.line(x, rect.y, x, rect.bottom(), theme::WARN, 1);
        ui.text(
            &ui::Rect::new(x - 46.0, rect.y + 3.0, 44.0, 14.0),
            "critical",
            Font::Small,
            theme::WARN,
            DT_RIGHT | DT_SINGLELINE,
        );
    }

    let pts_before: Vec<(f32, f32)> = pages
        .curve
        .points
        .iter()
        .map(|point| to_pos(point.temperature_c, point.duty_percent))
        .collect();
    let picked = |mouse: (f32, f32)| -> Option<usize> {
        let (index, best) = pts_before.iter().enumerate().fold(
            (None, f32::INFINITY),
            |(index, best), (i, (x, y))| {
                let d = ((mouse.0 - x).powi(2) + (mouse.1 - y).powi(2)).sqrt();
                if d < best {
                    (Some(i), d)
                } else {
                    (index, best)
                }
            },
        );
        (best <= PICK_RADIUS).then_some(index).flatten()
    };
    if ui.widgets.press == Some(id) && ui.input.valid {
        if pages.curve_drag.is_none() {
            pages.curve_drag = picked(ui.input.mouse);
        }
        if let Some(index) = pages.curve_drag {
            pages.curve
                .clamp_moved_point(index, t_at(ui.input.mouse.0), d_at(ui.input.mouse.1));
        }
    }
    if ui.input.double_click && ui.contains(&rect) && picked(ui.input.mouse).is_none() {
        pages.curve.insert_curve_point(t_at(ui.input.mouse.0), d_at(ui.input.mouse.1));
    }
    if ui.input.right_released && ui.contains(&rect) {
        if let Some(index) = picked(ui.input.mouse) {
            if pages.curve.points.len() > 2 {
                pages.curve.points.remove(index);
            }
            pages.curve_drag = None;
        }
    }
    if ui.input.just_up {
        pages.curve_drag = None;
    }

    for pair in pts_before.windows(2) {
        ui.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, ui.palette.accent, 2);
    }
    for (index, (x, y)) in pts_before.iter().enumerate() {
        let radius = if pages.curve_drag == Some(index) { 5.5 } else { 4.0 };
        let fill = if pages.curve_drag == Some(index) {
            ui.palette.accent
        } else {
            ui.palette.card
        };
        ui.circle(*x, *y, radius, fill, ui.palette.accent);
    }
    // Simulated temperature marker with the resulting output duty.
    let duty_now = pages.curve.duty(Some(pages.temperature), 0);
    let (sx, _) = to_pos(pages.temperature, 0.0);
    ui.line(sx, rect.y, sx, rect.bottom(), theme::WARN, 1);
    let (px, py) = to_pos(pages.temperature, duty_now);
    ui.circle(px, py, 3.0, theme::WARN, theme::WARN);
    ui.text(
        &ui::Rect::new(sx + 4.0, rect.y + 20.0, 120.0, 14.0),
        &format!("{:.0} °C → {:.0} %", pages.temperature, duty_now),
        Font::Small,
        theme::WARN,
        DT_LEFT | DT_SINGLELINE,
    );
}

/// Live motherboard cooling (experimental; same RPC as the Classic GUI).
fn mainboard_section(ui: &mut Ui, pages: &mut Pages, snapshot: &Snapshot, tx: &Sender<Command>, curve: &FanCurve) {
    let status = snapshot.control["cooling_session"].clone();
    let board = &mut pages.mainboard;
    ui.hint("Experimental motherboard fan control. Selecting a channel does not prove its physical header or PWM/DC mode.");
    if status["available"] != true {
        ui.hint("Install/enable the optional mainboard provider, then restart through the launcher. Live control requires the current protected service.");
        return;
    }
    // Channel/temperature pickers.
    let channels = |kind: &'static str| -> (Vec<String>, Vec<String>) {
        let mut labels = Vec::new();
        let mut ids = Vec::new();
        if let Some(list) = snapshot.control["cooling_channels"].as_array() {
            for channel in list {
                if channel["kind"] != kind {
                    continue;
                }
                let Some(id) = channel["reading"]["id"].as_str() else { continue };
                labels.push(id.to_owned());
                ids.push(id.to_owned());
            }
        }
        (labels, ids)
    };
    let (control_items, control_ids) = channels("control");
    let (fan_items, fan_ids) = channels("fan");
    let mut temperature_items = Vec::new();
    let mut temperature_ids = Vec::new();
    if let Some(entries) = snapshot.control["monitoring_sensors"].as_array() {
        for entry in entries {
            if entry["provider"] != "mainboard.lhm" || entry["sensor"]["unit"] != "°C" {
                continue;
            }
            let Some(id) = entry["sensor"]["id"].as_str() else { continue };
            temperature_items.push(id.to_owned());
            temperature_ids.push(id.to_owned());
        }
    }
    let pick = |items: &[String], current: &str| items.iter().position(|item| item == current);
    if let Some(choice) = ui.combo(
        ui::id(&["cooling", "live-control"]),
        &control_items,
        pick(&control_items, &board.control),
        "Fan output",
    ) {
        board.control = control_ids[choice].clone();
        board.confirmed = false;
    }
    if let Some(choice) = ui.combo(
        ui::id(&["cooling", "live-rpm"]),
        &fan_items,
        pick(&fan_items, &board.rpm),
        "Matching fan RPM",
    ) {
        board.rpm = fan_ids[choice].clone();
        board.confirmed = false;
    }
    if let Some(choice) = ui.combo(
        ui::id(&["cooling", "live-temperature"]),
        &temperature_items,
        pick(&temperature_items, &board.temperature),
        "Temperature source",
    ) {
        board.temperature = temperature_ids[choice].clone();
        board.confirmed = false;
    }
    let selected = snapshot.control["cooling_channels"]
        .as_array()
        .and_then(|list| {
            list.iter()
                .find(|channel| channel["kind"] == "control" && channel["reading"]["id"] == board.control.as_str())
        });
    let blocked = selected.and_then(refusal);
    if let Some(channel) = selected {
        ui.label_color(
            &format!("Support: {}", tier_label(channel_tier(channel))),
            Font::Body,
            ui.palette.text,
        );
        let evidence = snapshot.control["cooling_support"]["channels"][board.control.as_str()].clone();
        ui.label_color(
            &format!("Channel test: {}", evidence["test"].as_str().unwrap_or("none")),
            Font::Body,
            ui.palette.text,
        );
        if channel_tier(channel) == "lhm_supported" {
            ui.hint("LHM-supported: reduced-speed curves need a passed channel test; calibration (100 → 40%) is available.");
        }
    }
    if let Some(reason) = blocked {
        ui.label_color(reason, Font::Body, theme::WARN);
    }
    ui.checkbox(
        ui::id(&["cooling", "live-confirm"]),
        &mut board.confirmed,
        "I confirm this is a normal fan, not a pump, and the selected RPM belongs to it.",
    );
    ui.hint("Starting requests Windows UAC for this selected fan. Native curves require minimum ≥40% and critical temperature ≤100°C.");
    let busy = ["authorizing", "executing", "stopping"]
        .contains(&status["state"].as_str().unwrap_or(""));
    let ready = snapshot.connected
        && board.confirmed
        && !board.control.is_empty()
        && !board.rpm.is_empty()
        && !board.temperature.is_empty()
        && status["state"] != "blocked"
        && blocked.is_none()
        && !busy;
    {
        let mut row = Row::start(ui);
        if ui.button_at_enabled(
            ui::id(&["cooling", "live-curve"]),
            &row.next(210.0, 30.0),
            "Start current curve (UAC)",
            true,
            ready,
        ) {
            send(
                tx,
                "cooling.sessions.start",
                json!({"mode": "curve", "control_id": board.control, "temperature_id": board.temperature,
                       "rpm_id": board.rpm, "confirmed_fan": board.confirmed, "curve": curve}),
            );
        }
        if ui.button_at_enabled(
            ui::id(&["cooling", "live-calibrate"]),
            &row.next(170.0, 30.0),
            "Calibrate fan (UAC)",
            false,
            ready,
        ) {
            send(
                tx,
                "cooling.sessions.start",
                json!({"mode": "calibration", "control_id": board.control, "temperature_id": board.temperature,
                       "rpm_id": board.rpm, "confirmed_fan": board.confirmed, "critical_c": curve.critical_c}),
            );
        }
        if ui.button_at_enabled(
            ui::id(&["cooling", "live-test"]),
            &row.next(180.0, 30.0),
            "Run channel test (UAC)",
            false,
            ready,
        ) {
            send(
                tx,
                "cooling.sessions.start",
                json!({"mode": "channel_test", "control_id": board.control, "temperature_id": board.temperature,
                       "rpm_id": board.rpm, "confirmed_fan": board.confirmed, "critical_c": curve.critical_c}),
            );
        }
        row.end(ui, 30.0);
    }
    let mut duty = board.manual_duty;
    let display = format!("{:.0} %", duty);
    if ui.slider(
        ui::id(&["cooling", "live-manual"]),
        &mut duty,
        0.0,
        100.0,
        "Fixed %",
        &display,
    ) {
        board.manual_duty = duty;
    }
    {
        let mut row = Row::start(ui);
        if ui.button_at_enabled(
            ui::id(&["cooling", "live-manual-run"]),
            &row.next(180.0, 30.0),
            "Run fixed duty (UAC)",
            false,
            ready,
        ) {
            send(
                tx,
                "cooling.sessions.start",
                json!({"mode": "manual", "control_id": board.control, "temperature_id": board.temperature,
                       "rpm_id": board.rpm, "confirmed_fan": board.confirmed, "duty_percent": board.manual_duty}),
            );
        }
        if ui.button_at_enabled(
            ui::id(&["cooling", "live-stop"]),
            &row.next(230.0, 30.0),
            "Stop and return to firmware",
            true,
            snapshot.connected && busy,
        ) {
            send(tx, "cooling.sessions.stop", json!({"job_id": status["job_id"]}));
        }
        row.end(ui, 30.0);
    }
    // Unrestricted control (untested hardware).
    let support = snapshot.control["cooling_support"].clone();
    if !support.is_null() {
        ui.separator();
        ui.label_color(
            "Unrestricted control (untested hardware): no channel test, down to 0% (fan stop), RPM deviations only warn. Always kept: firmware return on stop/crash/telemetry loss/watchdog, 100% at critical temperature, no takeover of foreign software mode.",
            Font::Small,
            theme::WARN,
        );
        let mut enabled = support["unrestricted_enabled"] == true;
        if ui.checkbox(ui::id(&["cooling", "unrestricted"]), &mut enabled, "Allow unrestricted control (off by default)") {
            send(tx, "cooling.support.settings", json!({"unrestricted_enabled": enabled}));
        }
        if enabled && !board.control.is_empty() {
            let state = support["channels"][board.control.as_str()]["unlock"]
                .as_str()
                .unwrap_or("none")
                .to_owned();
            ui.label_color(
                &format!("Selected channel unlock: {state}. Type the channel name to unlock, or 'PUMP <name>' to confirm a pump."),
                Font::Body,
                ui.palette.text,
            );
            ui.text_line(ui::id(&["cooling", "unlock-text"]), &mut board.confirmation, "confirmation");
            let text = board.confirmation.trim().to_owned();
            let unlocked = state == "unlocked";
            let mut row = Row::start(ui);
            if ui.button_at_enabled(
                ui::id(&["cooling", "unlock"]),
                &row.next(160.0, 30.0),
                "Unlock channel",
                false,
                snapshot.connected && !text.is_empty(),
            ) {
                send(
                    tx,
                    "cooling.channels.unlock",
                    json!({"control_id": board.control, "confirmation": text}),
                );
            }
            if ui.button_at_enabled(
                ui::id(&["cooling", "pump"]),
                &row.next(190.0, 30.0),
                "Confirm pump control",
                false,
                snapshot.connected && unlocked,
            ) {
                send(
                    tx,
                    "cooling.channels.confirm_pump",
                    json!({"control_id": board.control, "confirmation": board.confirmation.trim()}),
                );
            }
            if ui.button_at_enabled(
                ui::id(&["cooling", "revoke"]),
                &row.next(110.0, 30.0),
                "Revoke",
                false,
                snapshot.connected && state != "none",
            ) {
                send(tx, "cooling.channels.revoke", json!({"control_id": board.control}));
            }
            row.end(ui, 30.0);
        }
    }
    // Background control in the protected service.
    let background = snapshot.control["cooling_background"].clone();
    if background["available"] == true {
        ui.separator();
        ui.hint("Background control: saved curves run in the protected service from Windows startup, without this window.");
        let saved: Vec<Value> = background["saved"]["channels"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for channel in background["service"]["channels"].as_array().into_iter().flatten() {
            ui.label_color(
                &format!(
                    "{} · {} · {}% · {}{}",
                    channel["control_id"].as_str().unwrap_or("?"),
                    channel["state"].as_str().unwrap_or("?"),
                    channel["duty_percent"],
                    tier_label(channel["tier"].as_str().unwrap_or("")),
                    channel["fault"]
                        .as_str()
                        .map(|fault| format!(" · {fault}"))
                        .unwrap_or_default()
                ),
                Font::Small,
                ui.palette.text_dim,
            );
        }
        let applying = background["apply"]["state"] == "applying";
        let duplicate = saved.iter().any(|channel| channel["control_id"] == board.control.as_str());
        {
            let mut row = Row::start(ui);
            if ui.button_at_enabled(
                ui::id(&["cooling", "bg-run"]),
                &row.next(280.0, 30.0),
                "Run current curve in background (UAC)",
                true,
                ready && !applying && !duplicate && saved.len() < 8,
            ) {
                let mut next = saved.clone();
                next.push(json!({"control_id": board.control, "temperature_id": board.temperature,
                                 "rpm_id": board.rpm, "curve": curve, "confirmed_fan": true}));
                send(tx, "cooling.background.apply", json!({"channels": next}));
            }
            if ui.button_at_enabled(
                ui::id(&["cooling", "bg-disable"]),
                &row.next(260.0, 30.0),
                "Disable background control (UAC)",
                false,
                snapshot.connected && !applying && !saved.is_empty(),
            ) {
                send(tx, "cooling.background.apply", json!({"channels": []}));
            }
            row.end(ui, 30.0);
        }
        // Cooling profiles.
        let profiles = snapshot.control["cooling_profiles"].clone();
        ui.label_color(
            &format!("Cooling profiles (active: {})", profiles["active"].as_str().unwrap_or("none")),
            Font::Body,
            ui.palette.text,
        );
        let profile_items: Vec<String> = profiles["profiles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                entry["name"].as_str().map(|name| format!("{name} · {} channels", entry["channels"]))
            })
            .collect();
        let profile_names: Vec<String> = profiles["profiles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
            .collect();
        let index = profile_names.iter().position(|name| name == &board.profile);
        if let Some(choice) = ui.combo(
            ui::id(&["cooling", "bg-profile"]),
            &profile_items,
            index,
            "Profile",
        ) {
            board.profile = profile_names[choice].clone();
        }
        {
            let mut row = Row::start(ui);
            if ui.button_at_enabled(
                ui::id(&["cooling", "bg-activate"]),
                &row.next(160.0, 30.0),
                "Activate (UAC)",
                true,
                snapshot.connected && !applying && !board.profile.is_empty(),
            ) {
                send(tx, "cooling.profiles.activate", json!({"name": board.profile}));
            }
            if ui.button_at_enabled(
                ui::id(&["cooling", "bg-delete"]),
                &row.next(110.0, 30.0),
                "Delete",
                false,
                snapshot.connected && !board.profile.is_empty(),
            ) {
                send(tx, "cooling.profiles.delete", json!({"name": board.profile}));
                board.profile.clear();
            }
            row.end(ui, 30.0);
        }
        ui.text_line(ui::id(&["cooling", "bg-profile-name"]), &mut board.profile_name, "profile name");
        let name = board.profile_name.trim().to_owned();
        if snapshot.connected && !name.is_empty()
            && ui.button(ui::id(&["cooling", "bg-profile-save"]), "Save current background as profile", false)
        {
            send(tx, "cooling.profiles.save", json!({"name": name}));
            board.profile = name;
            board.profile_name.clear();
        }
        if let Some(error) = profiles["error"].as_str() {
            ui.label_color(error, Font::Body, theme::BAD);
        }
        if let Some(error) = background["apply"]["error"].as_str() {
            ui.label_color(error, Font::Body, theme::BAD);
        }
    }
    ui.label_color(
        &format!(
            "Job {} · {}{}",
            status["job_id"],
            status["state"].as_str().unwrap_or("unknown"),
            status["tier"]
                .as_str()
                .map(|tier| format!(" · {}", tier_label(tier)))
                .unwrap_or_default()
        ),
        Font::Body,
        ui.palette.text_dim,
    );
    if let Some(error) = status["error"].as_str() {
        ui.label_color(error, Font::Body, theme::BAD);
    }
    if ui.button(
        ui::id(&["cooling", "report-toggle"]),
        {
            if board.show_report {
                "Hide last result"
            } else {
                "Show last execution / calibration result"
            }
        },
        false,
    ) {
        board.show_report = !board.show_report;
    }
    if board.show_report && !status["report"].is_null() {
        let text = serde_json::to_string_pretty(&status["report"]).unwrap_or_default();
        ui.clip(
            &ui::Rect::new(ui.bounds.0, ui.cursor.1, ui.column_width(), 220.0),
            |ui| {
                ui.label_color(&text, Font::Mono, ui.palette.text_dim);
            },
        );
        ui.advance(220.0 + 8.0);
    }
    ui.hint("Closing the GUI leaves the daemon running. Use Stop to return this fan to firmware control.");
}

fn cooling(ui: &mut Ui, pages: &mut Pages, snapshot: &Snapshot, tx: &Sender<Command>) {
    if !snapshot.connected {
        unreachable_hint(ui);
        return;
    }
    ui.heading("Cooling / fan curves");
    ui.label_color(
        "Curve editor and simulation; live fan control is a separate explicit action below.",
        Font::Small,
        theme::WARN,
    );
    ui.hint("Edit, save and preview a temperature curve. Missing or stale sensors select 100% duty.");
    ui.label_color("Name", Font::Strong, ui.palette.text);
    ui.text_line(ui::id(&["cooling", "curve-name"]), &mut pages.curve_name, "curve name");
    if let Some(curves) = snapshot.control.get("curves").and_then(Value::as_object) {
        ui.strong("Saved curves");
        for (name, curve) in curves {
            if ui.button(ui::id(&["cooling", "curve", name]), name, false) {
                if let Ok(parsed) = serde_json::from_value::<FanCurve>(curve.clone()) {
                    pages.curve = parsed;
                    pages.curve_name = name.clone();
                }
            }
        }
    }
    curve_graph(ui, pages);
    ui.hint("Drag points · double-click to add · right-click to remove. Edits clamp to their neighbours so the curve stays valid.");
    {
        let mut row = Row::start(ui);
        if ui.button_at_enabled(
            ui::id(&["cooling", "curve-add"]),
            &row.next(140.0, 30.0),
            "Add point",
            false,
            pages.curve.points.len() < 16,
        ) {
            let temperature = pages
                .curve
                .points
                .last()
                .map_or(25.0, |point| point.temperature_c + 5.0);
            pages.curve.points.push(CurvePoint {
                temperature_c: temperature,
                duty_percent: 100.0,
            });
        }
        if ui.button_at_enabled(
            ui::id(&["cooling", "curve-remove"]),
            &row.next(170.0, 30.0),
            "Remove last point",
            false,
            pages.curve.points.len() > 2,
        ) {
            pages.curve.points.pop();
        }
        row.end(ui, 30.0);
    }
    let mut value = pages.curve.minimum_percent;
    let display = format!("{value:.0} %");
    if ui.slider(
        ui::id(&["cooling", "min-duty"]),
        &mut value,
        0.0,
        100.0,
        "Minimum duty %",
        &display,
    ) {
        pages.curve.minimum_percent = value;
    }
    for (step, (slot, label)) in [
        (&mut pages.curve.max_rise_percent_per_s, "Step up %/s (0 = off)"),
        (&mut pages.curve.max_fall_percent_per_s, "Step down %/s (0 = off)"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut limit = slot.unwrap_or(0.0);
        let display = format!("{limit:.0}");
        if ui.slider(
            ui::id(&["cooling", if step == 0 { "rise" } else { "fall" }]),
            &mut limit,
            0.0,
            100.0,
            label,
            &display,
        ) {
            *slot = (limit >= 1.0).then_some(limit);
        }
    }
    let mut value = pages.curve.critical_c;
    let display = format!("{value:.0} °C");
    if ui.slider(
        ui::id(&["cooling", "critical"]),
        &mut value,
        40.0,
        110.0,
        "Emergency temperature °C",
        &display,
    ) {
        pages.curve.critical_c = value;
    }
    let mut value = pages.curve.hysteresis_c;
    let display = format!("{value:.1} °C");
    if ui.slider(
        ui::id(&["cooling", "hysteresis"]),
        &mut value,
        0.0,
        10.0,
        "Cooling hysteresis °C",
        &display,
    ) {
        pages.curve.hysteresis_c = value;
    }
    let validation = pages.curve.validate();
    if let Err(error) = &validation {
        ui.label_color(error, Font::Body, theme::BAD);
    }
    let valid = validation.is_ok();
    let mut row = Row::start(ui);
    if ui.button_at_enabled(
        ui::id(&["cooling", "curve-save"]),
        &row.next(170.0, 30.0),
        "Save curve",
        true,
        snapshot.connected && valid,
    ) {
        send(
            tx,
            "cooling.curves.save",
            json!({"name": pages.curve_name, "curve": pages.curve}),
        );
        pages.note(format!("Curve “{}” saved", pages.curve_name));
    }
    if ui.button_at_enabled(
        ui::id(&["cooling", "curve-delete"]),
        &row.next(190.0, 30.0),
        "Delete saved curve",
        false,
        snapshot.connected,
    ) {
        send(tx, "cooling.curves.delete", json!({"name": pages.curve_name}));
        pages.note(format!("Curve “{}” deleted", pages.curve_name));
    }
    row.end(ui, 30.0);
    let mut value = pages.temperature;
    let display = format!("{value:.0} °C");
    if ui.slider(
        ui::id(&["cooling", "simulated"]),
        &mut value,
        20.0,
        100.0,
        "Simulated temperature °C",
        &display,
    ) {
        pages.temperature = value;
    }
    ui.label_color(
        &format!(
            "Target fan output: {:.1}% (preview)",
            pages.curve.duty(Some(pages.temperature), 0)
        ),
        Font::Body,
        ui.palette.text,
    );
    // Read-only provider and channel inventory.
    if let Some(providers) = snapshot.control["cooling_providers"].as_array() {
        ui.strong("Motherboard providers and channels");
        for provider in providers {
            ui.label_color(
                &format!(
                    "{}: {}",
                    provider["name"].as_str().unwrap_or("Provider"),
                    provider["status"].as_str().unwrap_or("unavailable")
                ),
                Font::Body,
                ui.palette.text,
            );
            ui.hint(
                provider["message"].as_str().unwrap_or(""),
            );
        }
    }
    if let Some(channels) = snapshot.control["cooling_channels"].as_array() {
        for channel in channels {
            ui.label_color(
                &format!(
                    "{} · {} · {} {} · read only",
                    channel["name"].as_str().unwrap_or("Channel"),
                    channel["kind"].as_str().unwrap_or(""),
                    channel["reading"]["value"],
                    channel["reading"]["unit"].as_str().unwrap_or("")
                ),
                Font::Small,
                ui.palette.text_dim,
            );
        }
    }
    ui.hint("Header/control pairing and PWM/DC mode require verification. RGB/ARGB is a separate connection.");
    ui.strong("Live motherboard fan control (experimental)");
    mainboard_section(ui, pages, snapshot, tx, &pages.curve.clone());
}

// ---- macros (page 2) ----

fn macros(ui: &mut Ui, pages: &mut Pages, snapshot: &Snapshot, tx: &Sender<Command>) {
    if !snapshot.connected {
        unreachable_hint(ui);
        return;
    }
    ui.heading("Macros");
    let running = snapshot.control["macro_status"].clone();
    ui.label_color(
        &format!("Running: {}", running["running"].as_str().unwrap_or("none")),
        Font::Body,
        ui.palette.text,
    );
    if !running["input_enabled"].as_bool().unwrap_or(false) {
        ui.hint("RGB macros enabled. Text/keys require daemon --enable-input-macros (Windows).");
    }
    if let Some(error) = running["error"].as_str() {
        ui.label_color(error, Font::Body, theme::BAD);
    }
    if let Some(saved) = snapshot.control["macros"].as_object() {
        for (name, definition) in saved {
            if ui.button(ui::id(&["macros", "open", name]), name, false) {
                pages.macro_name = name.clone();
                pages.macro_json =
                    serde_json::to_string_pretty(definition).unwrap_or_default();
            }
        }
    }
    ui.label_color("Name", Font::Strong, ui.palette.text);
    ui.text_line(ui::id(&["macros", "name"]), &mut pages.macro_name, "macro name");
    ui.hint("Add, edit and reorder steps. Text, keys and shortcuts target the foreground application.");
    ui.hint("Steps are stored as JSON; the Classic (egui) GUI offers the guided step editor.");
    ui.text_multiline(ui::id(&["macros", "json"]), &mut pages.macro_json, 190.0);
    let parsed = serde_json::from_str::<MacroDefinition>(&pages.macro_json)
        .map_err(|error| error.to_string())
        .and_then(|definition| definition.validate().map(|()| definition));
    if let Err(error) = &parsed {
        ui.label_color(error, Font::Body, theme::BAD);
    }
    let mut row = Row::start(ui);
    if ui.button_at_enabled(
        ui::id(&["macros", "save"]),
        &row.next(140.0, 30.0),
        "Save macro",
        true,
        snapshot.connected && parsed.is_ok(),
    ) {
        if let Ok(definition) = serde_json::to_value(parsed.as_ref()) {
            send(tx, "macros.save", json!({"name": pages.macro_name, "definition": definition}));
            pages.note(format!("Macro “{}” saved", pages.macro_name));
        }
    }
    if ui.button_at(ui::id(&["macros", "run"]), &row.next(150.0, 30.0), "Run saved macro", false) {
        send(tx, "macros.run", json!({"name": pages.macro_name}));
        pages.note(format!("Macro “{}” started", pages.macro_name));
    }
    if ui.button_at(ui::id(&["macros", "cancel"]), &row.next(90.0, 30.0), "Cancel", false) {
        send(tx, "macros.cancel", json!({}));
    }
    if ui.button_at(ui::id(&["macros", "delete"]), &row.next(170.0, 30.0), "Delete saved macro", false) {
        send(tx, "macros.delete", json!({"name": pages.macro_name}));
        pages.note(format!("Macro “{}” deleted", pages.macro_name));
    }
    row.end(ui, 30.0);
}

// ---- monitoring (page 3) ----

/// Small line chart of one sensor series.
fn series_graph(ui: &mut Ui, series: &std::collections::VecDeque<Sample>) {
    let values: Vec<Option<f64>> = series.iter().map(|sample| sample.value).collect();
    let rect = ui.item(ui.column_width().min(320.0), 56.0);
    ui.box_(
        &rect,
        Some((ui.palette.card, ui.palette.card)),
        Some(ui.palette.border),
    );
    let plotted: Vec<f64> = values.iter().filter_map(|value| *value).collect();
    if plotted.is_empty() {
        return;
    }
    let mut low = plotted.iter().copied().fold(f64::INFINITY, f64::min);
    let mut high = plotted.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if (high - low).abs() < f64::EPSILON {
        low -= 1.0;
        high += 1.0;
    }
    let pad = (high - low) * 0.15;
    let (low, high) = (low - pad, high + pad);
    let (w, h) = (rect.w - 8.0, rect.h - 8.0);
    let at = |index: usize, value: f64| -> (f32, f32) {
        (
            rect.x + 4.0 + w * index as f32 / (values.len() - 1).max(1) as f32,
            rect.bottom() - 4.0 - h * ((value - low) / (high - low)) as f32,
        )
    };
    let points: Vec<(f32, f32)> = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.map(|value| at(index, value)))
        .collect();
    for pair in points.windows(2) {
        ui.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, ui.palette.accent, 1);
    }
}

fn monitoring(ui: &mut Ui, pages: &mut Pages, snapshot: &Snapshot, tx: &Sender<Command>) {
    if !snapshot.connected {
        unreachable_hint(ui);
        return;
    }
    ui.heading("Monitoring");
    {
        let connected = snapshot.connected;
        let mut row = Row::start(ui);
        if ui.button_at_enabled(
            ui::id(&["monitoring", "export"]),
            &row.next(200.0, 30.0),
            "Save diagnostic report",
            true,
            connected,
        ) {
            send(tx, "gui.export_diagnostics", Value::Null);
            pages.note("Diagnostic report exported");
        }
        if ui.button_at_enabled(
            ui::id(&["monitoring", "export-cooling"]),
            &row.next(200.0, 30.0),
            "…with fan hardware details",
            false,
            connected,
        ) {
            send(tx, "gui.export_diagnostics", json!({"include_cooling_hardware": true}));
            pages.note("Diagnostic report exported (with cooling hardware)");
        }
        row.end(ui, 30.0);
    }
    ui.hint("Adds board model, BIOS, chip, support tier and channel-test result (no serials or readings).");
    if let Some(path) = &snapshot.exported_report {
        ui.label_color(
            &format!("Report saved: {}", path.display()),
            Font::Body,
            ui.palette.text_dim,
        );
    }
    ui.hint("Cached device sensors; missing or stale samples are marked. No synthetic CPU/GPU readings.");
    if let Some(entries) = snapshot.control["monitoring_sensors"].as_array() {
        for entry in entries.iter().filter(|entry| entry["provider"] == "mainboard.lhm") {
            let sensor = &entry["sensor"];
            ui.label_color(
                &format!(
                    "{} / {}: {} {} · {} ms",
                    entry["device_name"].as_str().unwrap_or("Mainboard"),
                    sensor["name"].as_str().unwrap_or("Sensor"),
                    sensor["value"],
                    sensor["unit"].as_str().unwrap_or(""),
                    sensor["age_ms"]
                ),
                Font::Body,
                ui.palette.text,
            );
        }
    }
    for device in &snapshot.devices {
        let Some(sensors) = device["sensors"].as_array().filter(|sensors| !sensors.is_empty()) else {
            continue;
        };
        ui.heading(
            device["info"]["name"].as_str().unwrap_or("Device"),
        );
        for sensor in sensors {
            let age = sensor["age_ms"].as_u64().unwrap_or(u64::MAX);
            let value = sensor["value"]
                .as_f64()
                .map_or_else(|| "missing".into(), |value| format!("{value:.0}"));
            let text = format!(
                "{}: {} {} · {} ms{}",
                sensor["name"].as_str().unwrap_or("Sensor"),
                value,
                sensor["unit"].as_str().unwrap_or(""),
                age,
                if age > 3000 { " · STALE" } else { "" }
            );
            ui.label_color(&text, Font::Body, ui.palette.text);
            let key = (
                device["info"]["id"].as_str().unwrap_or_default().to_owned(),
                sensor["id"].as_str().unwrap_or_default().to_owned(),
            );
            if let Some(series) = pages.history.series.get(&key) {
                series_graph(ui, series);
            }
        }
    }
    if ui.button(
        ui::id(&["monitoring", "diagnostics"]),
        {
            if pages.show_diagnostics {
                "Hide diagnostics JSON"
            } else {
                "Show diagnostics JSON"
            }
        },
        false,
    ) {
        pages.show_diagnostics = !pages.show_diagnostics;
    }
    if pages.show_diagnostics {
        if let Some(diagnostics) = &snapshot.diagnostics {
            let mut text = serde_json::to_string_pretty(diagnostics).unwrap_or_default();
            text.truncate(4096);
            ui.label_color(&text, Font::Mono, ui.palette.text_dim);
        }
    }
}

// ---- updates (page 4) ----

fn updates(ui: &mut Ui, pages: &mut Pages) {
    ui.heading("Updates & installation");
    ui.label_color(
        &format!("Installed build: {}", env!("CARGO_PKG_VERSION")),
        Font::Body,
        ui.palette.text,
    );
    let view = pages
        .update_view
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let mut automatic = view.automatic;
    if ui.checkbox(
        ui::id(&["updates", "automatic"]),
        &mut automatic,
        "Automatically check and download updates every 6 hours",
    ) {
        let _ = pages.update_tx.send(UpdateCommand::Automatic(automatic));
    }
    ui.hint("Installers come from GitHub Releases and are SHA-256 verified. Installation starts when you choose it.");
    ui.hint("Private repository: FREERGB_GITHUB_TOKEN needs read access; tokens are read from the environment only.");
    let mut row = Row::start(ui);
    if ui.button_at_enabled(
        ui::id(&["updates", "check"]),
        &row.next(170.0, 30.0),
        "Check for updates",
        true,
        !view.busy,
    ) {
        let _ = pages.update_tx.send(UpdateCommand::Check);
    }
    if ui.button_at_enabled(
        ui::id(&["updates", "download"]),
        &row.next(170.0, 30.0),
        "Download update",
        false,
        !view.busy && view.update.is_some(),
    ) {
        let _ = pages.update_tx.send(UpdateCommand::Download);
    }
    row.end(ui, 30.0);
    ui.label_color(&view.message, Font::Body, ui.palette.text);
    if let Some(update) = &view.update {
        ui.label_color(
            &format!("Available version: {}", update.version),
            Font::Body,
            ui.palette.text_dim,
        );
    }
    if let (Some(path), Some(update)) = (&view.downloaded, &view.update) {
        if ui.button(ui::id(&["updates", "install"]), "Run verified installer", true) {
            let result = freergb_update::verify(path, update).and_then(|()| {
                std::process::Command::new(path)
                    .spawn()
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            });
            pages.note(match result {
                Ok(()) => "Installer started. Close freeRGB when asked by setup.".to_owned(),
                Err(error) => error,
            });
        }
    }
}