//! Session-opt-in foreground application profiles. No activity log.
use super::{Command, Snapshot};
use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::Read,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

// Invalidates queued automatic loads when the user disables or edits automation.
static SESSION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    executable: String,
    profile: String,
}
fn executable(text: &str) -> Option<String> {
    let text = text.trim();
    if text.len() <= 4
        || text.len() > 255
        || !text.to_ascii_lowercase().ends_with(".exe")
        || text.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
    {
        return None;
    }
    Some(text.to_ascii_lowercase())
}
fn validate(rules: &[Rule]) -> Result<(), String> {
    if rules.len() > 64 {
        return Err("At most 64 application rules".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for rule in rules {
        let name = executable(&rule.executable)
            .ok_or("Use an executable filename, for example game.exe")?;
        if !seen.insert(name)
            || rule.profile.is_empty()
            || rule.profile.chars().count() > 64
            || rule.profile.trim() != rule.profile
            || !rule
                .profile
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        {
            return Err("Duplicate application or invalid profile name".into());
        }
    }
    Ok(())
}

#[derive(Default)]
struct Planner {
    candidate: Option<(String, f64)>,
    handled: Option<String>,
}
impl Planner {
    fn observe(&mut self, app: Option<&str>, now: f64, blocked: bool) -> Option<String> {
        let Some(app) = app else {
            self.candidate = None;
            return None;
        };
        if self
            .candidate
            .as_ref()
            .map_or(true, |(candidate, _)| candidate != app)
        {
            self.candidate = Some((app.into(), now));
        }
        if blocked || self.handled.as_deref() == Some(app) {
            return None;
        }
        if self
            .candidate
            .as_ref()
            .is_some_and(|(_, since)| now - since >= 2.0)
        {
            self.handled = Some(app.into());
            return Some(app.into());
        }
        None
    }
}

#[cfg(windows)]
fn foreground() -> Option<String> {
    let window = active_win_pos_rs::get_active_window().ok()?;
    executable(window.process_path.file_name()?.to_str()?)
}
#[cfg(not(windows))]
fn foreground() -> Option<String> {
    None
}

/// Called by the API worker, not the GUI thread. Recheck queued work before load.
pub fn load_if_current(params: Value) -> Result<Value, freergb_client::ClientError> {
    let ticket = params["session"].as_u64().unwrap_or(0);
    if ticket == 0 || SESSION.load(Ordering::SeqCst) != ticket {
        return Ok(json!(false));
    }
    let status = freergb_client::call("macros.status", Value::Null)?;
    if !status["running"].is_null() {
        return Ok(json!(false));
    }
    if foreground().as_deref() != params["executable"].as_str()
        || SESSION.load(Ordering::SeqCst) != ticket
    {
        return Ok(json!(false));
    }
    freergb_client::call("profiles.load", json!({"name":params["profile"]}))
}

struct ForegroundWorker {
    requests: std::sync::mpsc::SyncSender<u64>,
    results: std::sync::mpsc::Receiver<(u64, Option<String>)>,
}
impl ForegroundWorker {
    fn new() -> Self {
        let (requests, input) = std::sync::mpsc::sync_channel(1);
        let (output, results) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            while let Ok(ticket) = input.recv() {
                if SESSION.load(Ordering::SeqCst) != ticket {
                    continue;
                }
                let app = foreground();
                if output.send((ticket, app)).is_err() {
                    break;
                }
            }
        });
        Self { requests, results }
    }
}

pub struct AppProfiles {
    rules: Vec<Rule>,
    enabled: bool,
    ticket: u64,
    planner: Planner,
    worker: Option<ForegroundWorker>,
    started: Instant,
    last_poll: Option<Instant>,
    error: Option<String>,
    current: Option<String>,
}
impl Default for AppProfiles {
    fn default() -> Self {
        let loaded = (|| -> Result<Vec<Rule>, String> {
            let Some(path) =
                super::pages::config_root().map(|p| p.join("application-profiles.json"))
            else {
                return Ok(Vec::new());
            };
            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
                Err(e) => return Err(e.to_string()),
            };
            let mut text = String::new();
            file.take(128 * 1024 + 1)
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            if text.len() > 128 * 1024 {
                return Err("Application profile configuration too large".into());
            }
            let rules: Vec<Rule> = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            validate(&rules)?;
            Ok(rules)
        })();
        let (rules, error) = match loaded {
            Ok(rules) => (rules, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        Self {
            rules,
            error,
            enabled: false,
            ticket: 0,
            planner: Planner::default(),
            worker: None,
            started: Instant::now(),
            last_poll: None,
            current: None,
        }
    }
}
impl AppProfiles {
    pub fn active(&self) -> bool {
        self.enabled
    }
    pub fn disable(&mut self) {
        if self.enabled {
            SESSION.fetch_add(1, Ordering::SeqCst);
        }
        self.enabled = false;
        self.planner = Planner::default();
        self.current = None;
        self.last_poll = None;
    }
    pub fn poll(&mut self, tx: &std::sync::mpsc::Sender<Command>, snapshot: &Snapshot) -> bool {
        if !self.enabled {
            return false;
        }
        if !snapshot.connected || snapshot.profiles.is_none() {
            self.disable();
            self.error =
                Some("Daemon/profiles unavailable; enable again after reconnecting".into());
            return false;
        }
        let mut sent = false;
        if let Some(worker) = &self.worker {
            for (ticket, app) in worker.results.try_iter() {
                if ticket != self.ticket {
                    continue;
                }
                self.current = app.clone();
                let blocked = !snapshot.control["macro_status"]["running"].is_null();
                if let Some(app) = self.planner.observe(
                    app.as_deref(),
                    self.started.elapsed().as_secs_f64(),
                    blocked,
                ) {
                    if let Some(rule) = self
                        .rules
                        .iter()
                        .find(|r| executable(&r.executable).as_deref() == Some(&app))
                    {
                        if !snapshot
                            .profiles
                            .as_ref()
                            .is_some_and(|list| list.contains(&rule.profile))
                        {
                            self.error =
                                Some(format!("Profile {:?} no longer exists", rule.profile));
                            continue;
                        }
                        sent |= tx.send(Command { method: "gui.load_app_profile", params: json!({"executable":app,"profile":rule.profile,"session":self.ticket}) }).is_ok();
                    }
                }
            }
            if self
                .last_poll
                .map_or(true, |last| last.elapsed().as_secs_f64() >= 1.0)
            {
                let _ = worker.requests.try_send(self.ticket);
                self.last_poll = Some(Instant::now());
            }
        }
        sent
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, profiles: Option<&[String]>) {
        ui.collapsing("Profiles by foreground application", |ui| {
            ui.label("Windows only; off at each launch. Matches executable filenames after 2 seconds. Unmatched applications keep the current lighting. No activity history is saved.");
            let mut enabled = self.enabled;
            if ui.add_enabled(cfg!(windows) && profiles.is_some(), egui::Checkbox::new(&mut enabled,"Enable application profiles for this session")).changed() {
                if !enabled { self.disable(); }
                else if self.rules.is_empty() { self.error = Some("Add an application rule first".into()); }
                else if let Err(error) = validate(&self.rules) { self.error = Some(error); }
                else if self.rules.iter().any(|r| !profiles.unwrap_or_default().contains(&r.profile)) { self.error = Some("Choose existing saved profiles".into()); }
                else {
                    self.ticket = SESSION.fetch_add(1,Ordering::SeqCst) + 1;
                    self.enabled = true; self.error = None; self.planner = Planner::default();
                    if self.worker.is_none() { self.worker = Some(ForegroundWorker::new()); }
                }
            }
            if self.enabled {
                ui.label(format!("Foreground: {}",self.current.as_deref().unwrap_or("unavailable / waiting")));
            }
            ui.add_enabled_ui(!self.enabled, |ui| {
                let mut remove = None;
                for (index, rule) in self.rules.iter_mut().enumerate() {
                    ui.push_id(index, |ui| { ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut rule.executable).hint_text("game.exe").desired_width(160.0));
                        egui::ComboBox::from_id_salt("profile").selected_text(&rule.profile).show_ui(ui, |ui| {
                            for name in profiles.unwrap_or_default() { ui.selectable_value(&mut rule.profile,name.clone(),name); }
                        });
                        if ui.small_button("Remove").clicked() { remove = Some(index); }
                    }); });
                }
                if let Some(index) = remove { self.rules.remove(index); }
                if ui.add_enabled(self.rules.len()<64,egui::Button::new("Add application")).clicked() { self.rules.push(Rule { executable: String::new(), profile: String::new() }); }
                if ui.button("Save application profile rules").clicked() {
                    self.error = (|| -> Result<(),String> {
                        validate(&self.rules)?;
                        let path = super::pages::config_root().ok_or("Configuration directory unavailable")?.join("application-profiles.json");
                        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;
                        let temporary = path.with_extension("json.tmp");
                        std::fs::write(&temporary,serde_json::to_vec_pretty(&self.rules).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                        std::fs::rename(temporary,path).map_err(|e|e.to_string())
                    })().err();
                }
            });
            if let Some(error) = &self.error { ui.colored_label(egui::Color32::LIGHT_RED,error); }
        });
    }
}
impl Drop for AppProfiles {
    fn drop(&mut self) {
        self.disable();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_wildcards_and_duplicate_executables_are_rejected() {
        for name in ["C:\\game.exe", "../game.exe", "*.exe", "game", "bad\n.exe"] {
            assert!(executable(name).is_none());
        }
        assert_eq!(executable(" Game.EXE ").as_deref(), Some("game.exe"));
        assert!(validate(&[
            Rule {
                executable: "Game.exe".into(),
                profile: "Gaming".into()
            },
            Rule {
                executable: "GAME.EXE".into(),
                profile: "Other".into()
            }
        ])
        .is_err());
    }
    #[test]
    fn focus_must_stabilize_and_does_not_reload_repeatedly() {
        let mut p = Planner::default();
        assert_eq!(p.observe(Some("a.exe"), 0.0, false), None);
        assert_eq!(p.observe(Some("b.exe"), 1.0, false), None);
        assert_eq!(p.observe(Some("a.exe"), 2.0, false), None);
        assert_eq!(p.observe(Some("a.exe"), 4.0, false), Some("a.exe".into()));
        assert_eq!(p.observe(Some("a.exe"), 100.0, false), None);
        p.observe(Some("other.exe"), 101.0, false);
        p.observe(Some("other.exe"), 103.0, false);
        p.observe(Some("a.exe"), 104.0, false);
        assert_eq!(p.observe(Some("a.exe"), 106.0, false), Some("a.exe".into()));
    }
    #[test]
    fn running_macro_defers_selection_and_missing_focus_resets_debounce() {
        let mut p = Planner::default();
        p.observe(Some("a.exe"), 0.0, true);
        assert_eq!(p.observe(Some("a.exe"), 10.0, true), None);
        assert_eq!(p.observe(Some("a.exe"), 11.0, false), Some("a.exe".into()));
        let mut p = Planner::default();
        p.observe(Some("a.exe"), 0.0, false);
        p.observe(None, 1.0, false);
        assert_eq!(p.observe(Some("a.exe"), 2.0, false), None);
        assert_eq!(p.observe(Some("a.exe"), 4.0, false), Some("a.exe".into()));
    }
    #[test]
    fn invalidated_command_never_contacts_daemon() {
        assert_eq!(load_if_current(json!({"session":0})).unwrap(), json!(false));
    }
}
