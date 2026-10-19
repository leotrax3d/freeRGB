//! Saved cooling policy and user-triggered macros. No cooling hardware writes.
use freergb_control::{FanCurve, MacroDefinition, MacroStep};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Library {
    curves: BTreeMap<String, FanCurve>,
    macros: BTreeMap<String, MacroDefinition>,
}

struct Job {
    name: String,
    steps: Vec<MacroStep>,
    index: usize,
    due: Instant,
}

#[derive(Default)]
pub struct Controls {
    library: Library,
    path: Option<PathBuf>,
    job: Option<Job>,
    pub input_enabled: bool,
    error: Option<String>,
}

impl Controls {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Library>(&text) {
                Ok(library)
                    if library.curves.len() <= 64
                        && library.macros.len() <= 64
                        && library.curves.iter().all(|(n, c)| {
                            crate::profiles::validate_name(n).is_ok() && c.validate().is_ok()
                        })
                        && library.macros.iter().all(|(n, m)| {
                            crate::profiles::validate_name(n).is_ok() && m.validate().is_ok()
                        }) =>
                {
                    self.library = library
                }
                _ => self.error = Some("invalid cooling/macro library ignored".into()),
            },
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                self.error = Some(error.to_string())
            }
            _ => {}
        }
        self.path = Some(path);
        self
    }

    fn save(&self, library: &Library) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(library).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(tmp, path).map_err(|e| e.to_string())
    }

    pub fn call(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        if method == "cooling.status" {
            return Ok(
                json!({"hardware_control":false,"reason":"Cooling write commands are not hardware-validated. Curve preview only.","error":self.error}),
            );
        }
        if method == "cooling.curves.list" {
            return Ok(json!(self.library.curves));
        }
        if method == "macros.list" {
            return Ok(json!(self.library.macros));
        }
        if method == "macros.status" {
            return Ok(
                json!({"running":self.job.as_ref().map(|j| &j.name),"input_enabled":self.input_enabled,"error":self.error}),
            );
        }
        if method == "macros.cancel" {
            self.job = None;
            return Ok(json!(true));
        }
        if method == "cooling.preview" {
            let curve: FanCurve =
                serde_json::from_value(params["curve"].clone()).map_err(|e| e.to_string())?;
            curve.validate()?;
            let temperature = params
                .get("temperature_c")
                .and_then(Value::as_f64)
                .map(|v| v as f32);
            let age = params.get("age_ms").and_then(Value::as_u64).unwrap_or(0);
            return Ok(json!({"duty_percent":curve.duty(temperature, age),"simulated":true}));
        }
        let name = params["name"].as_str().ok_or("expected name")?;
        crate::profiles::validate_name(name).map_err(|e| e.to_string())?;
        if method == "macros.run" {
            if self.job.is_some() {
                return Err("a macro is already running; cancel it first".into());
            }
            let definition = self.library.macros.get(name).ok_or("macro not found")?;
            if definition.needs_input() && !self.input_enabled {
                return Err("keyboard input disabled; restart with --enable-input-macros".into());
            }
            #[cfg(not(windows))]
            if definition.needs_input() {
                return Err("keyboard macro output is currently Windows-only".into());
            }
            let steps = (0..definition.repeat)
                .flat_map(|_| definition.steps.clone())
                .collect();
            self.job = Some(Job {
                name: name.into(),
                steps,
                index: 0,
                due: Instant::now(),
            });
            self.error = None;
            return Ok(json!(true));
        }
        // Clone before writing: storage failure must not change live definitions.
        let mut library = self.library.clone();
        match method {
            "cooling.curves.save" => {
                let curve: FanCurve =
                    serde_json::from_value(params["curve"].clone()).map_err(|e| e.to_string())?;
                curve.validate()?;
                if library.curves.len() >= 64 && !library.curves.contains_key(name) {
                    return Err("curve library is full".into());
                }
                library.curves.insert(name.into(), curve);
            }
            "cooling.curves.delete" => {
                if library.curves.remove(name).is_none() {
                    return Err("curve not found".into());
                }
            }
            "macros.save" => {
                let definition: MacroDefinition =
                    serde_json::from_value(params["definition"].clone())
                        .map_err(|e| e.to_string())?;
                definition.validate()?;
                if library.macros.len() >= 64 && !library.macros.contains_key(name) {
                    return Err("macro library is full".into());
                }
                library.macros.insert(name.into(), definition);
            }
            "macros.delete" => {
                if library.macros.remove(name).is_none() {
                    return Err("macro not found".into());
                }
            }
            _ => return Err("unknown control method".into()),
        }
        self.save(&library)?;
        self.library = library;
        Ok(json!(true))
    }

    pub fn tick(&mut self, now: Instant) -> Option<MacroStep> {
        let job = self.job.as_mut()?;
        if now < job.due {
            return None;
        }
        let Some(step) = job.steps.get(job.index).cloned() else {
            self.job = None;
            return None;
        };
        job.index += 1;
        if let MacroStep::Delay { ms } = step {
            job.due = now + Duration::from_millis(u64::from(ms));
            return None;
        }
        Some(step)
    }

    pub fn fail(&mut self, error: String) {
        self.job = None;
        self.error = Some(error);
    }
}

pub fn input(step: &MacroStep) -> Result<(), String> {
    #[cfg(windows)]
    {
        use enigo::{Direction, Enigo, Key, Keyboard, Settings};
        let mut input = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        let map_key = |key: &str| -> Result<Key, String> {
            Ok(match key {
                "ctrl" => Key::Control,
                "alt" => Key::Alt,
                "shift" => Key::Shift,
                "meta" => Key::Meta,
                "enter" => Key::Return,
                "escape" => Key::Escape,
                "tab" => Key::Tab,
                "space" => Key::Space,
                "backspace" => Key::Backspace,
                "up" => Key::UpArrow,
                "down" => Key::DownArrow,
                "left" => Key::LeftArrow,
                "right" => Key::RightArrow,
                "media_play_pause" => Key::MediaPlayPause,
                "volume_up" => Key::VolumeUp,
                "volume_down" => Key::VolumeDown,
                "volume_mute" => Key::VolumeMute,
                single
                    if single.len() == 1
                        && single
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) =>
                {
                    Key::Unicode(single.chars().next().unwrap())
                }
                _ => return Err("unsupported key".into()),
            })
        };
        match step {
            MacroStep::Chord { keys } => freergb_control::execute_chord(keys, |key, down| {
                input
                    .key(
                        map_key(key)?,
                        if down {
                            Direction::Press
                        } else {
                            Direction::Release
                        },
                    )
                    .map_err(|e| e.to_string())
            }),
            MacroStep::Text { text } => input.text(text).map_err(|e| e.to_string()),
            MacroStep::Key { key } => input
                .key(map_key(key)?, Direction::Click)
                .map_err(|e| e.to_string()),
            _ => Ok(()),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = step;
        Err("keyboard output is Windows-only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn macro_scheduler_delay_cancel_and_input_gate() {
        let mut controls = Controls::default();
        controls.call("macros.save", &json!({"name":"Test","definition":{"repeat":1,"steps":[{"type":"delay","ms":100},{"type":"color","color":{"r":1,"g":2,"b":3}}]}})).unwrap();
        controls
            .call("macros.run", &json!({"name":"Test"}))
            .unwrap();
        let now = Instant::now();
        assert!(controls.tick(now).is_none());
        assert!(controls.tick(now + Duration::from_millis(50)).is_none());
        assert!(matches!(
            controls.tick(now + Duration::from_millis(100)),
            Some(MacroStep::Color { .. })
        ));
        controls.call("macros.cancel", &json!({})).unwrap();
        assert!(controls.tick(now).is_none());
        controls.call("macros.save", &json!({"name":"Input","definition":{"repeat":1,"steps":[{"type":"text","text":"test"}]}})).unwrap();
        assert!(controls
            .call("macros.run", &json!({"name":"Input"}))
            .is_err());
    }

    #[test]
    fn library_persists_valid_definitions_and_reports_invalid_files() {
        let directory =
            std::env::temp_dir().join(format!("freergb-controls-{}", std::process::id()));
        let path = directory.join("control.json");
        let mut controls = Controls::default().with_file(path.clone());
        controls
            .call(
                "cooling.curves.save",
                &json!({"name":"Balanced","curve":FanCurve::default()}),
            )
            .unwrap();
        let mut restored = Controls::default().with_file(path.clone());
        assert!(restored.call("cooling.curves.list", &json!({})).unwrap()["Balanced"].is_object());
        let invalid = FanCurve {
            minimum_percent: 101.0,
            ..FanCurve::default()
        };
        assert!(restored
            .call(
                "cooling.curves.save",
                &json!({"name":"Broken","curve":invalid})
            )
            .is_err());
        std::fs::write(&path, "{}").unwrap();
        let mut corrupt = Controls::default().with_file(path.clone());
        assert!(corrupt.call("cooling.status", &json!({})).unwrap()["error"].is_string());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
