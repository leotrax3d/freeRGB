?//! Page layouts for the Win32 GUI. Each page mirrors its egui Classic page and
//! sends exactly the same JSON-RPC methods; only the widgets differ. The
//! layer effect editor, sensor alert rules, application profiles and global
//! macro shortcuts remain exclusive to the Classic (egui) and desktop GUIs.

use crate::rpc::{Command, Snapshot};
use crate::theme::{self, Font};
use crate::ui::{self, Ui};
use freergb_control::FanCurve;
use freergb_core::Region;
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