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
    #[serde(default)]
    sensors: freergb_control::sensors::SensorGraph,
    /// Composite curves (graph/constant/trigger/mix/follow). Preview only.
    #[serde(default)]
    curve_set: freergb_control::curves::CurveSet,
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
    sensor_evaluator: freergb_control::sensors::SensorEvaluator,
    curve_evaluator: freergb_control::curves::CurveEvaluator,
    /// Outputs read by live `mode:"composite"` sessions.
    pub composite_feed: crate::composite_feed::CompositeFeed,
    sensor_epoch: Option<Instant>,
    sensor_inputs: BTreeMap<String, freergb_control::sensors::InputSample>,
    sensor_sources: BTreeMap<String, String>,
    path: Option<PathBuf>,
    job: Option<Job>,
    pub input_enabled: bool,
    error: Option<String>,
}

impl Controls {
    pub fn refresh_sensors(&mut self, devices: &Value) {
        let now = self.sensor_now();
        self.sensor_inputs.clear();
        self.sensor_sources.clear();
        for device in devices.as_array().into_iter().flatten() {
            if device["status"] != "Connected" {
                continue;
            }
            let Some(id) = device["info"]["id"].as_str() else {
                continue;
            };
            for sensor in device["sensors"].as_array().into_iter().flatten() {
                if sensor["unit"] != "°C" && sensor["unit"] != "C" {
                    continue;
                }
                let (Some(sensor_id), Some(age)) =
                    (sensor["id"].as_str(), sensor["age_ms"].as_u64())
                else {
                    continue;
                };
                let key = serde_json::to_string(&(id, sensor_id)).unwrap_or_default();
                self.sensor_sources.insert(
                    key.clone(),
                    format!(
                        "{} / {}",
                        device["info"]["name"].as_str().unwrap_or(id),
                        sensor["name"].as_str().unwrap_or(sensor_id)
                    ),
                );
                if age <= now {
                    self.sensor_inputs.insert(
                        key,
                        freergb_control::sensors::InputSample {
                            value: sensor["value"].as_f64(),
                            sampled_at_ms: now - age,
                        },
                    );
                }
            }
        }
        let outputs = self.evaluate_curve_set(now);
        self.composite_feed.publish(outputs, Instant::now());
    }

    /// Evaluates the composite curve set against live and derived sensors.
    fn evaluate_curve_set(
        &mut self,
        now: u64,
    ) -> BTreeMap<String, freergb_control::curves::CurveOutput> {
        let derived = self
            .sensor_evaluator
            .evaluate(&self.library.sensors, &self.sensor_inputs, now)
            .unwrap_or_default();
        let inputs = &self.sensor_inputs;
        let lookup = |id: &str| {
            let (value, at) = match derived.get(id) {
                Some(reading) => (reading.value, reading.sampled_at_ms),
                None => inputs
                    .get(id)
                    .map_or((None, None), |s| (s.value, Some(s.sampled_at_ms))),
            };
            freergb_control::curves::SensorValue {
                celsius: value.map(|v| v as f32),
                age_ms: at.map_or(u64::MAX, |at| now.saturating_sub(at)),
            }
        };
        self.curve_evaluator
            .evaluate(&self.library.curve_set, lookup, now)
    }

    pub fn has_composite_output(&self, name: &str) -> bool {
        self.library.curve_set.nodes.contains_key(name)
    }
    fn sensor_now(&mut self) -> u64 {
        self.sensor_epoch
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }

    pub fn with_file(mut self, path: PathBuf) -> Self {
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Library>(&text) {
                Ok(library)
                    if library.curves.len() <= 64
                        && library.macros.len() <= 64
                        && library.sensors.validate().is_ok()
                        && library.curve_set.validate().is_ok()
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
        if method == "cooling.calibration.preview" {
            let request: freergb_control::calibration::CalibrationPreviewRequest =
                serde_json::from_value(params.clone()).map_err(|e| e.to_string())?;
            return serde_json::to_value(request.run()?).map_err(|e| e.to_string());
        }
        if method == "cooling.sensors.sources" {
            return Ok(json!(self.sensor_sources));
        }
        if method == "cooling.sensors.values" {
            let now = self.sensor_now();
            let values =
                self.sensor_evaluator
                    .evaluate(&self.library.sensors, &self.sensor_inputs, now)?;
            return Ok(json!({"now_ms": now, "values": values}));
        }
        if method == "cooling.sensors.get" {
            return Ok(json!(self.library.sensors));
        }
        if method == "cooling.sensors.save" {
            let graph: freergb_control::sensors::SensorGraph =
                serde_json::from_value(params["graph"].clone()).map_err(|e| e.to_string())?;
            graph.validate()?;
            let mut library = self.library.clone();
            library.sensors = graph;
            self.save(&library)?;
            self.library = library;
            return Ok(json!(true));
        }
        if method == "cooling.curve_set.get" {
            return Ok(json!(self.library.curve_set));
        }
        if method == "cooling.curve_set.save" {
            let set: freergb_control::curves::CurveSet =
                serde_json::from_value(params["set"].clone()).map_err(|e| e.to_string())?;
            set.validate()?;
            let mut library = self.library.clone();
            library.curve_set = set;
            self.save(&library)?;
            self.library = library;
            return Ok(json!(true));
        }
        if method == "cooling.curve_set.values" {
            // Preview of the same evaluation that composite sessions read.
            let now = self.sensor_now();
            let outputs = self.evaluate_curve_set(now);
            let outputs: BTreeMap<_, _> = outputs
                .into_iter()
                .map(|(name, o)| (name, json!({"percent": o.percent, "failsafe": o.failsafe})))
                .collect();
            return Ok(json!({"now_ms": now, "outputs": outputs, "simulated": true}));
        }
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
    fn curve_set_persists_and_previews_with_failsafe() {
        let directory =
            std::env::temp_dir().join(format!("freergb-curve-set-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("control.json");
        std::fs::write(&path, r#"{"curves":{},"macros":{}}"#).unwrap();
        let mut controls = Controls::default().with_file(path.clone());
        let set = json!({"nodes":{
            "cpu":{"type":"graph","sensor":"missing","curve":FanCurve::default()},
            "floor":{"type":"constant","percent":35.0},
            "case":{"type":"mix","sources":["floor"],"function":"maximum"}}});
        controls
            .call("cooling.curve_set.save", &json!({"set":set}))
            .unwrap();
        let cyclic = json!({"nodes":{"a":{"type":"follow","source":"a"}}});
        assert!(controls
            .call("cooling.curve_set.save", &json!({"set":cyclic}))
            .is_err());
        let values = controls
            .call("cooling.curve_set.values", &Value::Null)
            .unwrap();
        assert_eq!(values["outputs"]["case"]["percent"], 35.0);
        assert_eq!(values["outputs"]["cpu"]["failsafe"], true);
        let reloaded = Controls::default()
            .with_file(path.clone())
            .call("cooling.curve_set.get", &Value::Null)
            .unwrap();
        assert_eq!(reloaded["nodes"]["floor"]["percent"], 35.0);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn custom_sensor_graph_persists_and_invalid_updates_leave_saved_state_unchanged() {
        let directory =
            std::env::temp_dir().join(format!("freergb-sensor-graph-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("control.json");
        std::fs::write(&path, r#"{"curves":{},"macros":{}}"#).unwrap();
        let mut controls = Controls::default().with_file(path.clone());
        assert_eq!(
            controls.call("cooling.sensors.get", &Value::Null).unwrap(),
            json!({"version":1,"nodes":{}})
        );
        let graph = json!({"version":1,"nodes":{"hub":{"type":"input","source":"native-id"},"hot":{"type":"offset","source":"hub","amount":2.0,"proportional":false}}});
        controls
            .call("cooling.sensors.save", &json!({"graph":graph}))
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(controls.call("cooling.sensors.save",&json!({"graph":{"nodes":{"bad":{"type":"offset","source":"bad","amount":0.0,"proportional":false}}}})).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(
            Controls::default()
                .with_file(path.clone())
                .call("cooling.sensors.get", &Value::Null)
                .unwrap(),
            graph
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn sensor_values_expire_without_gui_or_new_hardware_reads() {
        let mut controls = Controls {
            sensor_epoch: Some(Instant::now() - Duration::from_secs(10)),
            ..Default::default()
        };
        let key = serde_json::to_string(&("board", "cpu")).unwrap();
        controls
            .call(
                "cooling.sensors.save",
                &json!({"graph":{"nodes":{"cpu":{"type":"input","source":key}}}}),
            )
            .unwrap();
        controls.refresh_sensors(&json!([{"status":"Connected","info":{"id":"board","name":"Board"},"sensors":[{"id":"cpu","name":"CPU","unit":"C","value":42.0,"age_ms":0}]}]));
        assert_eq!(
            controls
                .call("cooling.sensors.values", &Value::Null)
                .unwrap()["values"]["cpu"]["value"],
            42.0
        );
        for input in controls.sensor_inputs.values_mut() {
            input.sampled_at_ms -= 4000;
        }
        assert!(controls
            .call("cooling.sensors.values", &Value::Null)
            .unwrap()["values"]["cpu"]["value"]
            .is_null());
        assert!(controls
            .call(
                "cooling.sensors.save",
                &json!({"graph":{"version":2,"nodes":{}}})
            )
            .is_err());
        controls.refresh_sensors(&json!([]));
        assert_eq!(
            controls
                .call("cooling.sensors.sources", &Value::Null)
                .unwrap(),
            json!({})
        );
    }
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
