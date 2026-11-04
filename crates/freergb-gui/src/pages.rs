use super::toast::Toasts;
use super::{Command, Snapshot};
use eframe::egui;
use freergb_control::{CurvePoint, FanCurve};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

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

pub struct Panels {
    mainboard_cooling: super::mainboard_cooling::MainboardCooling,
    channel_bindings: super::channel_bindings::BindingsPanel,
    mainboard_rgb: super::mainboard_rgb::MainboardRgbPanel,
    hwmon: super::hwmon_cooling::HwmonPanel,
    custom_sensors: super::custom_sensors::CustomSensors,
    alerts: super::alerts::Alerts,
    hotkeys: super::hotkeys::Hotkeys,
    history: super::sensor_history::Histories,
    history_generation: u64,
    last_snapshot_s: f64,
    history_started: std::time::Instant,
    pub page: u8,
    curve: FanCurve,
    curve_name: String,
    /// Curve point currently dragged in the graph, or `None`.
    curve_drag: Option<usize>,
    temperature: f32,
    macro_name: String,
    macro_json: String,
    updates: Arc<Mutex<UpdateView>>,
    update_tx: mpsc::Sender<UpdateCommand>,
    message: Option<String>,
}

pub(super) fn config_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("freeRGB"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
            .map(|p| p.join("freergb"))
    }
}

impl Default for Panels {
    fn default() -> Self {
        let state = Arc::new(Mutex::new(UpdateView::default()));
        let (tx, rx) = mpsc::channel();
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
                        Err(mpsc::RecvTimeoutError::Timeout) if automatic => UpdateCommand::Check,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    }
                };
                if let UpdateCommand::Automatic(value) = command {
                    if let Some(path) = &preference {
                        let result =
                            std::fs::create_dir_all(path.parent().unwrap()).and_then(|()| {
                                std::fs::write(path, if value { "true" } else { "false" })
                            });
                        if let Err(error) = result {
                            worker.lock().unwrap_or_else(|e| e.into_inner()).message =
                                error.to_string();
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
                            let directory = root
                                .clone()
                                .unwrap_or_else(std::env::temp_dir)
                                .join("updates");
                            let result = freergb_update::download(&update, &directory);
                            let mut view = worker.lock().unwrap_or_else(|e| e.into_inner());
                            match result {
                                Ok(path) => {
                                    view.downloaded = Some(path);
                                    view.message =
                                        "Installer downloaded and SHA-256 verified".into();
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
                    let directory = root
                        .clone()
                        .unwrap_or_else(std::env::temp_dir)
                        .join("updates");
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
        Self { mainboard_cooling:Default::default(),mainboard_rgb:Default::default(),hwmon:Default::default(),channel_bindings:Default::default(),custom_sensors:Default::default(),alerts:Default::default(),hotkeys:Default::default(),history:Default::default(),history_generation:0,last_snapshot_s:0.0,history_started:std::time::Instant::now(),page: 0, curve: FanCurve::default(), curve_name: "Balanced".into(), curve_drag: None, temperature: 40.0,
            macro_name: "RGB demo".into(), macro_json: "{\n  \"repeat\": 1,\n  \"steps\": [\n    {\"type\": \"color\", \"color\": {\"r\": 255, \"g\": 0, \"b\": 0}},\n    {\"type\": \"delay\", \"ms\": 1000},\n    {\"type\": \"color\", \"color\": {\"r\": 0, \"g\": 100, \"b\": 255}}\n  ]\n}".into(), updates: state, update_tx: tx, message: None }
    }
}

impl Panels {
    pub fn sensor_warnings(&self) -> usize {
        self.alerts.unacknowledged()
    }
    pub fn poll_hotkeys(&mut self, tx: &mpsc::Sender<Command>, snapshot: &Snapshot) {
        self.hotkeys.poll(tx, snapshot);
    }
    pub fn hotkeys_active(&self) -> bool {
        self.hotkeys.active()
    }
    pub fn observe(&mut self, snapshot: &Snapshot) {
        self.custom_sensors.observe(snapshot);
        let now = self.history_started.elapsed().as_secs_f64();
        if snapshot.generation != self.history_generation {
            self.last_snapshot_s = now;
        }
        self.alerts.observe(
            &snapshot.devices,
            snapshot.connected && now - self.last_snapshot_s <= 5.0,
            now,
        );
        if snapshot.generation == self.history_generation {
            return;
        }
        self.history_generation = snapshot.generation;
        self.history.observe(
            &snapshot.devices,
            self.history_started.elapsed().as_secs_f64(),
        );
    }
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        tx: &mpsc::Sender<Command>,
        snapshot: &Snapshot,
        toasts: &mut Toasts,
    ) {
        let send = |method, params| {
            let _ = tx.send(Command { method, params });
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.page {
                1 => {
                    ui.heading("Cooling / fan curves");
                    if snapshot.control["hwmon"]["started"] == true {
                        let curve = self.curve.clone();
                        ui.collapsing("Motherboard fans (Linux hwmon)", |ui| self.hwmon.show(ui, snapshot, tx, &curve));
                    }
                    ui.collapsing("Motherboard channel assignments", |ui| {
                        self.channel_bindings.show(ui, snapshot, tx);
                    });
                    ui.collapsing("Motherboard providers and channels", |ui| {
                        if let Some(providers) = snapshot.control["cooling_providers"].as_array() {
                            for provider in providers {
                                ui.label(format!("{}: {}", provider["name"].as_str().unwrap_or("Provider"), provider["status"].as_str().unwrap_or("unavailable")));
                                ui.small(provider["message"].as_str().unwrap_or(""));
                            }
                        }
                        if let Some(channels) = snapshot.control["cooling_channels"].as_array() {
                            for channel in channels {
                                ui.label(format!("{} · {} · {} {} · read only", channel["name"].as_str().unwrap_or("Channel"), channel["kind"].as_str().unwrap_or(""), channel["reading"]["value"], channel["reading"]["unit"].as_str().unwrap_or("")));
                            }
                        }
                        ui.small("Header/control pairing and PWM/DC mode require verification. RGB/ARGB is a separate connection.");
                    });
                    ui.colored_label(egui::Color32::YELLOW, "Curve editor and simulation; live fan control is a separate explicit action below.");
                    ui.label("Edit, save and preview a temperature curve. Missing or stale sensors select 100% duty.");
                    ui.horizontal(|ui| { ui.label("Name"); ui.text_edit_singleline(&mut self.curve_name); });
                    if let Some(curves) = snapshot.control.get("curves").and_then(Value::as_object) {
                        ui.horizontal_wrapped(|ui| { for (name, curve) in curves { if ui.button(name).clicked() { if let Ok(curve) = serde_json::from_value(curve.clone()) { self.curve = curve; self.curve_name = name.clone(); } } } });
                    }
                    egui::Grid::new("curve-points").striped(true).show(ui, |ui| {
                        ui.label("Temperature °C"); ui.label("Fan duty %"); ui.end_row();
                        let mut remove = None;
                        for (i, point) in self.curve.points.iter_mut().enumerate() {
                            ui.add(egui::DragValue::new(&mut point.temperature_c).range(-20.0..=110.0));
                            ui.add(egui::DragValue::new(&mut point.duty_percent).range(20.0..=100.0));
                            if ui.small_button("Remove").clicked() { remove = Some(i); } ui.end_row();
                        }
                        if let Some(index) = remove { if self.curve.points.len() > 2 { self.curve.points.remove(index); } }
                    });
                    if ui.add_enabled(self.curve.points.len() < 16, egui::Button::new("Add point")).clicked() {
                        let temperature = self.curve.points.last().map_or(25.0, |p| p.temperature_c + 5.0);
                        self.curve.points.push(CurvePoint { temperature_c: temperature, duty_percent: 100.0 });
                    }
                    ui.add(egui::Slider::new(&mut self.curve.minimum_percent, 0.0..=100.0).text("Minimum duty %"));
                    for (limit, label) in [(&mut self.curve.max_rise_percent_per_s, "Step up %/s (0 = off)"), (&mut self.curve.max_fall_percent_per_s, "Step down %/s (0 = off)")] {
                        let mut value = limit.unwrap_or(0.0);
                        if ui.add(egui::Slider::new(&mut value, 0.0..=100.0).text(label)).changed() {
                            *limit = (value >= 1.0).then_some(value);
                        }
                    }
                    ui.add(egui::Slider::new(&mut self.curve.critical_c, 40.0..=110.0).text("Emergency temperature °C"));
                    ui.add(egui::Slider::new(&mut self.curve.hysteresis_c, 0.0..=10.0).text("Cooling hysteresis °C"));
                    let validation = self.curve.validate();
                    if let Err(error) = &validation { ui.colored_label(egui::Color32::LIGHT_RED, error); }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(snapshot.connected && validation.is_ok(), egui::Button::new("Save curve")).clicked() { send("cooling.curves.save", json!({"name":self.curve_name,"curve":self.curve})); toasts.success(format!("Curve “{}” saved", self.curve_name)); }
                        if ui.button("Delete saved curve").clicked() { send("cooling.curves.delete", json!({"name":self.curve_name})); toasts.success(format!("Curve “{}” deleted", self.curve_name)); }
                    });
                    draw_curve_graph(self, ui);
                    ui.small("Drag points · double-click to add · right-click to remove. Edits clamp to their neighbours so the curve stays valid.");
                    ui.add(egui::Slider::new(&mut self.temperature, 20.0..=100.0).text("Simulated temperature °C"));
                    ui.label(format!("Target fan output: {:.1}% (preview)", self.curve.duty(Some(self.temperature), 0)));
                    self.custom_sensors.ui(ui,tx,snapshot,&self.curve);
                    ui.collapsing("Live motherboard fan control",|ui|{self.mainboard_cooling.show(ui,snapshot,tx,&self.curve);});
                }
                2 => {
                    ui.heading("Macros");
                    self.hotkeys.ui(ui,snapshot);
                    let running = &snapshot.control["macro_status"];
                    ui.label(format!("Running: {}", running["running"].as_str().unwrap_or("none")));
                    if !running["input_enabled"].as_bool().unwrap_or(false) { ui.label("RGB macros enabled. Text/keys require daemon --enable-input-macros (Windows)."); }
                    if let Some(error) = running["error"].as_str() { ui.colored_label(egui::Color32::LIGHT_RED, error); }
                    if let Some(macros) = snapshot.control["macros"].as_object() {
                        ui.horizontal_wrapped(|ui| { for (name, definition) in macros { if ui.button(name).clicked() { self.macro_name = name.clone(); self.macro_json = serde_json::to_string_pretty(definition).unwrap_or_default(); } } });
                    }
                    ui.horizontal(|ui| { ui.label("Name"); ui.text_edit_singleline(&mut self.macro_name); });
                    ui.label("Add, edit and reorder steps. Text, keys and shortcuts target the foreground application.");
                    super::macro_editor::edit(ui, &mut self.macro_json);
                    let parsed = serde_json::from_str::<freergb_control::MacroDefinition>(&self.macro_json).map_err(|e| e.to_string()).and_then(|m| m.validate().map(|()| m));
                    if let Err(error) = &parsed { ui.colored_label(egui::Color32::LIGHT_RED, error); }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(snapshot.connected && parsed.is_ok(), egui::Button::new("Save macro")).clicked() { send("macros.save", json!({"name":self.macro_name,"definition":parsed.as_ref().unwrap()})); toasts.success(format!("Macro “{}” saved", self.macro_name)); }
                        if ui.button("Run saved macro").clicked() { send("macros.run", json!({"name":self.macro_name})); toasts.success(format!("Macro “{}” started", self.macro_name)); }
                        if ui.button("Cancel").clicked() { send("macros.cancel", json!({})); }
                        if ui.button("Delete saved macro").clicked() { send("macros.delete", json!({"name":self.macro_name})); toasts.success(format!("Macro “{}” deleted", self.macro_name)); }
                    });
                }
                3 => {
                    ui.heading("Updates & installation");
                    ui.label(format!("Installed build: {}", env!("CARGO_PKG_VERSION")));
                    let view = self.updates.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    let mut automatic = view.automatic;
                    if ui.checkbox(&mut automatic, "Automatically check and download updates every 6 hours").changed() { let _ = self.update_tx.send(UpdateCommand::Automatic(automatic)); }
                    ui.label("Installers come from GitHub Releases and are SHA-256 verified. Installation starts when you choose it.");
                    ui.label("Public releases need no token. Private releases can use FREERGB_GITHUB_TOKEN from the environment.");
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!view.busy, egui::Button::new("Check for updates")).clicked() { let _ = self.update_tx.send(UpdateCommand::Check); }
                        if ui.add_enabled(!view.busy && view.update.is_some(), egui::Button::new("Download update")).clicked() { let _ = self.update_tx.send(UpdateCommand::Download); }
                    });
                    ui.label(&view.message);
                    if let Some(update) = &view.update { ui.label(format!("Available version: {}", update.version)); }
                    if let (Some(path), Some(update)) = (&view.downloaded, &view.update) {
                        if ui.button("Run verified installer").clicked() {
                            let result = freergb_update::install_verified(path, update);
                            self.message = Some(match result { Ok(()) => "Installer started. Close freeRGB when asked by setup.".into(), Err(error) => error });
                        }
                    }
                    if let Some(message) = &self.message { ui.label(message); }
                }
                4 => {
                    ui.heading("Monitoring");
                    ui.collapsing("Motherboard RGB", |ui| self.mainboard_rgb.show(ui, snapshot, tx));
                    self.alerts.ui(ui);
                    if ui.add_enabled(snapshot.connected,egui::Button::new("Save diagnostic report")).clicked() { send("gui.export_diagnostics",Value::Null); toasts.success("Diagnostic report exported"); }
                    if ui.add_enabled(snapshot.connected,egui::Button::new("…with fan hardware details")).on_hover_text("Adds board model, BIOS, chip, support tier and channel-test result (no serials or readings).").clicked() { send("gui.export_diagnostics",serde_json::json!({"include_cooling_hardware":true})); toasts.success("Diagnostic report exported (with cooling hardware)"); }
                    if let Some(path)=&snapshot.exported_report { ui.label(format!("Report saved: {}",path.display())); }
                    ui.label("Cached device sensors; missing or stale samples are marked. No synthetic CPU/GPU readings.");
                    if let Some(entries) = snapshot.control["monitoring_sensors"].as_array() {
                        for entry in entries.iter().filter(|entry| entry["provider"] == "mainboard.lhm") {
                            let sensor = &entry["sensor"];
                            ui.label(format!("{} / {}: {} {} · {} ms", entry["device_name"].as_str().unwrap_or("Mainboard"), sensor["name"].as_str().unwrap_or("Sensor"), sensor["value"], sensor["unit"].as_str().unwrap_or(""), sensor["age_ms"]));
                        }
                    }
                    for device in &snapshot.devices {
                        if let Some(sensors) = device["sensors"].as_array().filter(|s| !s.is_empty()) {
                            ui.heading(device["info"]["name"].as_str().unwrap_or("Device"));
                            for sensor in sensors {
                                let age = sensor["age_ms"].as_u64().unwrap_or(u64::MAX);
                                let value = sensor["value"].as_f64().map_or_else(|| "missing".into(), |v| format!("{v:.0}"));
                                let text = format!("{}: {} {} · {} ms{}", sensor["name"].as_str().unwrap_or("Sensor"), value, sensor["unit"].as_str().unwrap_or(""), age, if age > 3000 { " · STALE" } else { "" });
                                ui.label(text);
                                self.alerts.add_sensor_ui(ui,device,sensor);
                                let key=(device["info"]["id"].as_str().unwrap_or_default().to_owned(),sensor["id"].as_str().unwrap_or_default().to_owned());
                                if let Some(series)=self.history.series.get(&key) { super::sensor_history::graph(ui,series); }
                            }
                        }
                    }
                    if let Some(diagnostics) = &snapshot.diagnostics {
                        ui.collapsing("Diagnostics JSON", |ui| {
                            ui.monospace(serde_json::to_string_pretty(diagnostics).unwrap_or_default());
                        });
                    }
                }
                _ => {}
            }
        });
    }
}

/// Temperature/duty axes for the interactive curve graph; matches the range
/// the numeric editor allows (`-20..=110 °C`).
const T_MIN: f32 = -20.0;
const T_MAX: f32 = 110.0;
/// Mouse distance (px) at which a graph point counts as picked.
const PICK_RADIUS: f32 = 18.0;

/// Interactive graph: drag points, double-click to add, right-click to
/// remove. Shows the minimum duty band, the critical temperature marker and
/// the simulated temperature from the slider below. The geometry helpers
/// (`clamp_moved_point`/`insert_curve_point`) live on `FanCurve` so both GUIs
/// share exactly the same editing rules.
fn draw_curve_graph(p: &mut Panels, ui: &mut egui::Ui) {
    let (outer, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(600.0), 190.0),
        egui::Sense::click_and_drag(),
    );
    let rect =
        egui::Rect::from_min_max(outer.min, egui::pos2(outer.right(), outer.bottom() - 14.0));
    let painter = ui.painter();
    painter.rect_filled(outer, 4.0, egui::Color32::from_gray(24));

    let to_pos = |t: f32, d: f32| {
        egui::pos2(
            rect.left() + ((t - T_MIN) / (T_MAX - T_MIN)).clamp(0.0, 1.0) * rect.width(),
            rect.bottom() - (d.clamp(0.0, 100.0) / 100.0) * rect.height(),
        )
    };
    let t_at = |x: f32| {
        T_MIN + (x - rect.left()).clamp(0.0, rect.width()) / rect.width() * (T_MAX - T_MIN)
    };
    let d_at = |y: f32| (rect.bottom() - y).clamp(0.0, rect.height()) / rect.height() * 100.0;

    for t in (0..=100).step_by(20) {
        let x = to_pos(t as f32, 0.0).x;
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0_f32, egui::Color32::from_gray(50)),
        );
        painter.text(
            egui::pos2(x, rect.bottom() + 3.0),
            egui::Align2::CENTER_TOP,
            format!("{t}"),
            egui::FontId::proportional(9.0),
            egui::Color32::from_gray(120),
        );
    }
    for d in (0..=100).step_by(25) {
        let y = to_pos(0.0, d as f32).y;
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0_f32, egui::Color32::from_gray(50)),
        );
        painter.text(
            egui::pos2(rect.left() + 3.0, y - 11.0),
            egui::Align2::LEFT_TOP,
            format!("{d}%"),
            egui::FontId::proportional(9.0),
            egui::Color32::from_gray(120),
        );
    }
    let min_y = to_pos(0.0, p.curve.minimum_percent).y;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), min_y),
            egui::pos2(rect.right(), rect.bottom()),
        ),
        0.0,
        crate::theme::SELECTION.gamma_multiply(0.35),
    );
    painter.line_segment(
        [
            egui::pos2(rect.left(), min_y),
            egui::pos2(rect.right(), min_y),
        ],
        egui::Stroke::new(1.0_f32, crate::theme::ACCENT.gamma_multiply(0.6)),
    );
    if (T_MIN..=T_MAX).contains(&p.curve.critical_c) {
        let x = to_pos(p.curve.critical_c, 0.0).x;
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.5_f32, crate::theme::WARN),
        );
        painter.text(
            egui::pos2(x - 4.0, rect.top() + 3.0),
            egui::Align2::RIGHT_TOP,
            "⚠",
            egui::FontId::proportional(12.0),
            crate::theme::WARN,
        );
    }

    let pts_before: Vec<egui::Pos2> = p
        .curve
        .points
        .iter()
        .map(|point| to_pos(point.temperature_c, point.duty_percent))
        .collect();
    let picked = |position: Option<egui::Pos2>, pts: &[egui::Pos2]| -> Option<usize> {
        let pointer = position?;
        let (index, closest) = pts.iter().enumerate().min_by(|a, b| {
            a.1.distance_sq(pointer)
                .total_cmp(&b.1.distance_sq(pointer))
        })?;
        (closest.distance_sq(pointer).sqrt() <= PICK_RADIUS).then_some(index)
    };
    if resp.dragged() {
        let pointer = resp.interact_pointer_pos();
        if p.curve_drag.is_none() {
            p.curve_drag = picked(pointer, &pts_before);
        }
        if let (Some(pointer), Some(index)) = (pointer, p.curve_drag) {
            p.curve
                .clamp_moved_point(index, t_at(pointer.x), d_at(pointer.y));
        }
    } else {
        p.curve_drag = None;
    }
    if resp.double_clicked() {
        if let Some(pointer) = resp.interact_pointer_pos() {
            if picked(Some(pointer), &pts_before).is_none() {
                p.curve.insert_curve_point(t_at(pointer.x), d_at(pointer.y));
            }
        }
    }
    if resp.clicked_by(egui::PointerButton::Secondary) {
        if let Some(pointer) = resp.interact_pointer_pos() {
            if let Some(index) = picked(Some(pointer), &pts_before) {
                if p.curve.points.len() > 2 {
                    p.curve.points.remove(index);
                }
                p.curve_drag = None;
            }
        }
    }

    let pts: Vec<egui::Pos2> = p
        .curve
        .points
        .iter()
        .map(|point| to_pos(point.temperature_c, point.duty_percent))
        .collect();
    painter.add(egui::Shape::line(
        pts.clone(),
        egui::Stroke::new(2.0_f32, crate::theme::ACCENT),
    ));
    for (index, pos) in pts.iter().enumerate() {
        let radius = if p.curve_drag == Some(index) {
            5.5
        } else {
            4.0
        };
        let fill = if p.curve_drag == Some(index) {
            crate::theme::ACCENT
        } else {
            egui::Color32::WHITE
        };
        painter.circle_filled(*pos, radius, fill);
    }

    // Simulated temperature marker with the resulting output duty.
    let duty_now = p.curve.duty(Some(p.temperature), 0);
    let sim_x = to_pos(p.temperature, 0.0).x;
    painter.line_segment(
        [
            egui::pos2(sim_x, rect.top()),
            egui::pos2(sim_x, rect.bottom()),
        ],
        egui::Stroke::new(1.0_f32, crate::theme::WARN.gamma_multiply(0.55)),
    );
    painter.circle_filled(to_pos(p.temperature, duty_now), 3.0, crate::theme::WARN);
    painter.text(
        egui::pos2(sim_x + 4.0, rect.top() + 18.0),
        egui::Align2::LEFT_TOP,
        format!("{:.0} °C → {:.0} %", p.temperature, duty_now),
        egui::FontId::proportional(11.0),
        crate::theme::WARN,
    );
    ui.label("X: −20–110 °C · Y: 0–100 % duty");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> FanCurve {
        FanCurve {
            points: vec![
                CurvePoint {
                    temperature_c: 25.0,
                    duty_percent: 30.0,
                },
                CurvePoint {
                    temperature_c: 50.0,
                    duty_percent: 60.0,
                },
                CurvePoint {
                    temperature_c: 75.0,
                    duty_percent: 100.0,
                },
            ],
            ..FanCurve::default()
        }
    }

    #[test]
    fn dragged_points_stay_between_their_neighbours() {
        let mut c = curve();
        // The last point cannot move beyond the critical temperature and
        // duty cannot fall below the previous point.
        c.clamp_moved_point(2, 105.0, 30.0);
        assert!(c.validate().is_ok());
        assert_eq!(c.points[2].duty_percent, 60.0);
        // Dragged duty must never fall below the previous point.
        c.clamp_moved_point(1, 40.0, 10.0);
        assert!(c.validate().is_ok());
        assert!(c.points[1].duty_percent >= c.points[0].duty_percent);
        // The first point cannot cross the second.
        c.clamp_moved_point(0, 90.0, 20.0);
        assert!(c.validate().is_ok());
        assert!(c.points[0].temperature_c < c.points[1].temperature_c);
    }

    #[test]
    fn double_click_inserts_a_sorted_point() {
        let mut c = curve();
        assert_eq!(c.insert_curve_point(40.0, 45.0), Some(1));
        assert!(c.validate().is_ok());
        assert_eq!(c.points[1].temperature_c, 40.0);
        assert_eq!(c.points[1].duty_percent, 45.0);
        assert_eq!(c.insert_curve_point(10.0, 30.0), Some(0));
        assert!(c.validate().is_ok());
        // The curve is full at 64 points.
        let mut full = FanCurve::default();
        for s in 0..64 {
            full.insert_curve_point(10.0 + s as f32 * 0.5, 100.0);
        }
        assert_eq!(full.insert_curve_point(120.0, 100.0), None);
    }
}
