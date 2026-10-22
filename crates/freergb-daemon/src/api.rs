//! JSON-RPC 2.0 handler, one request per line. Transport-agnostic so it can be
//! tested without sockets. Method reference: `docs/LOCAL_API.md`.

use crate::{lock, SharedEngine};
use freergb_core::Rgb;
use freergb_effects::EffectSpec;
use serde_json::{json, Value};

pub const API_VERSION: u32 = 1;

/// Outcome of one request: the response line and whether the daemon should
/// shut down afterwards.
pub struct Reply {
    pub line: String,
    pub shutdown: bool,
}

fn error(id: Value, code: i64, msg: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": msg.into()}})
}

fn invalid_params(e: impl std::fmt::Display) -> (i64, String) {
    (-32602, format!("invalid params: {e}"))
}

/// `custom_effects.*`: the effect editor's library (same storage rules as profiles).
fn custom_effect_call(
    engine: &SharedEngine,
    method: &str,
    params: &Value,
) -> Result<Value, (i64, String)> {
    use crate::profiles::ProfileError;

    let dir = lock(engine).custom_effects_dir().ok_or((
        -32003,
        "custom effects are disabled (daemon runs with --no-state)".to_owned(),
    ))?;
    let map_err = |e: ProfileError| match e {
        ProfileError::InvalidName(_) | ProfileError::Corrupt(..) => (-32602, e.to_string()),
        ProfileError::NotFound(_) => (-32004, e.to_string()),
        ProfileError::Io(_) => (-32000, e.to_string()),
    };
    let name = || {
        params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_params("expected {\"name\": \"…\"}"))
    };
    match method {
        "custom_effects.list" => crate::custom_effects::list(&dir)
            .map(|l| json!(l))
            .map_err(map_err),
        "custom_effects.get" => crate::custom_effects::load(&dir, name()?)
            .map(|s| json!(s))
            .map_err(map_err),
        "custom_effects.save" => {
            let n = name()?;
            let spec: EffectSpec =
                serde_json::from_value(params.get("effect").cloned().unwrap_or(Value::Null))
                    .map_err(invalid_params)?;
            crate::custom_effects::save(&dir, n, &spec)
                .map(|()| json!(true))
                .map_err(map_err)
        }
        _ => crate::custom_effects::delete(&dir, name()?)
            .map(|()| json!(true))
            .map_err(map_err),
    }
}

/// `profiles.*` methods. Errors: -32602 bad name, -32004 not found,
/// -32003 profiles disabled (`--no-state`), -32000 I/O.
fn profile_call(
    engine: &SharedEngine,
    method: &str,
    params: &Value,
) -> Result<Value, (i64, String)> {
    use crate::profiles::{self, ProfileError};

    let dir = lock(engine)
        .profiles_dir()
        .map(std::path::Path::to_path_buf)
        .ok_or((
            -32003,
            "profiles are disabled (daemon runs with --no-state)".to_owned(),
        ))?;
    let map_err = |e: ProfileError| match e {
        ProfileError::InvalidName(_) | ProfileError::Corrupt(..) => (-32602, e.to_string()),
        ProfileError::NotFound(_) => (-32004, e.to_string()),
        _ => (-32000, e.to_string()),
    };
    let name = || {
        params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_params("expected {\"name\": \"…\"}"))
    };

    match method {
        "profiles.list" => profiles::list(&dir).map(|l| json!(l)).map_err(map_err),
        "profiles.save" => {
            let n = name()?;
            let s = lock(engine).current_state();
            profiles::save(&dir, n, &s)
                .map(|()| json!(true))
                .map_err(map_err)
        }
        "profiles.load" => {
            let s = profiles::load(&dir, name()?).map_err(map_err)?;
            lock(engine).apply_state(s);
            Ok(json!(true))
        }
        _ => profiles::delete(&dir, name()?)
            .map(|()| json!(true))
            .map_err(map_err),
    }
}

pub fn handle_line(engine: &SharedEngine, line: &str) -> Reply {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return Reply {
                line: error(Value::Null, -32700, format!("parse error: {e}")).to_string(),
                shutdown: false,
            }
        }
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = req.get("method").and_then(Value::as_str) else {
        return Reply {
            line: error(id, -32600, "missing method").to_string(),
            shutdown: false,
        };
    };
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let mut shutdown = false;

    let result: Result<Value, (i64, String)> = match method {
        "daemon.version" => Ok(json!({
            "api_version": API_VERSION,
            "daemon_version": env!("CARGO_PKG_VERSION"),
        })),
        "devices.list" => Ok(json!(lock(engine).devices())),
        "keyboard.reactive.get" => Ok(lock(engine).keyboard_reactive()),
        "keyboard.reactive.disable" => Ok(lock(engine).keyboard.disable()),
        "keyboard.reactive.configure" => serde_json::from_value(params["config"].clone())
            .map_err(invalid_params)
            .and_then(|config| {
                lock(engine)
                    .keyboard
                    .configure(config)
                    .map(|()| json!(true))
                    .map_err(invalid_params)
            }),
        "keyboard.reactive.preview" => params["usage"]
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .ok_or_else(|| invalid_params("expected USB keyboard usage"))
            .and_then(|usage| {
                lock(engine)
                    .keyboard
                    .preview(usage)
                    .map(|()| json!(true))
                    .map_err(invalid_params)
            }),
        "monitoring.sensors.list" => Ok(json!(lock(engine).monitoring_sensors())),
        "cooling.providers.list" => Ok(json!([lock(engine).mainboard.summary()])),
        "cooling.channels.list" => Ok(json!(lock(engine).mainboard.channels())),
        "cooling.sessions.status" => Ok(json!(lock(engine).cooling_sessions.status())),
        "cooling.sessions.start" => serde_json::from_value(params.clone())
            .map_err(invalid_params)
            .and_then(|request: crate::cooling_sessions::StartRequest| {
                let mut engine = lock(engine);
                let policy = engine
                    .cooling_policy(&request.control_id)
                    .map_err(|error| (-32006, error))?;
                engine
                    .cooling_sessions
                    .start(request, &policy)
                    .map(|job| json!({"job_id":job}))
                    .map_err(|error| (-32006, error))
            }),
        "cooling.background.get" => Ok(lock(engine).background.view()),
        "cooling.background.apply" => serde_json::from_value(params.clone())
            .map_err(invalid_params)
            .and_then(|request: crate::background::BackgroundRequest| {
                let mut engine = lock(engine);
                let mut entries = Vec::new();
                for channel in &request.channels {
                    let policy = engine
                        .cooling_policy(&channel.control_id)
                        .map_err(|error| (-32006, error))?;
                    entries.push(
                        crate::background::protected_entry(channel, &policy, None)
                            .map_err(invalid_params)?,
                    );
                }
                engine
                    .background
                    .apply(request, entries)
                    .map(|()| json!(true))
                    .map_err(|error| (-32006, error))
            }),
        "cooling.sessions.stop" => params["job_id"]
            .as_u64()
            .ok_or_else(|| invalid_params("job_id required"))
            .and_then(|job| {
                lock(engine)
                    .cooling_sessions
                    .stop(job)
                    .map(|()| json!(true))
                    .map_err(|error| (-32006, error))
            }),
        "cooling.bindings.get" => {
            let engine = lock(engine);
            Ok(engine.channel_bindings.view(&engine.mainboard.channels()))
        }
        "cooling.bindings.save" => serde_json::from_value(params["config"].clone())
            .map_err(invalid_params)
            .and_then(|config| {
                let mut engine = lock(engine);
                let channels = engine.mainboard.channels();
                engine
                    .channel_bindings
                    .save(config, &channels)
                    .map(|()| json!(true))
                    .map_err(|e| match e {
                        crate::channel_bindings::SaveError::Invalid(message) => {
                            invalid_params(message)
                        }
                        crate::channel_bindings::SaveError::Io(message) => (-32000, message),
                    })
            }),
        "cooling.status"
        | "cooling.sensors.sources"
        | "cooling.sensors.values"
        | "cooling.sensors.get"
        | "cooling.sensors.save"
        | "cooling.curves.list"
        | "cooling.curves.save"
        | "cooling.curves.delete"
        | "cooling.preview"
        | "cooling.calibration.preview"
        | "macros.list"
        | "macros.save"
        | "macros.delete"
        | "macros.run"
        | "macros.cancel"
        | "macros.status" => lock(engine)
            .controls
            .call(method, &params)
            .map_err(invalid_params),
        "plugins.list" => Ok(json!(lock(engine).plugins())),
        "plugins.start" | "plugins.stop" => params
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_params("expected plugin id from plugins.list"))
            .and_then(|id| {
                lock(engine)
                    .set_plugin_enabled(id, method == "plugins.start")
                    .map_err(|e| (-32005, e))
            })
            .map(|()| json!(true)),
        "diagnostics.get" => Ok(json!(lock(engine).diagnostics())),
        "diagnostics.export" => Ok(crate::report::create(&lock(engine))),
        "effects.list" => Ok(json!(EffectSpec::NAMES)),
        "effects.default" => params
            .get("name")
            .and_then(Value::as_str)
            .and_then(EffectSpec::default_for)
            .map(|s| json!(s))
            .ok_or_else(|| {
                invalid_params(format!(
                    "expected {{\"name\": one of {:?}}}",
                    EffectSpec::NAMES
                ))
            }),
        "effects.get" => Ok(json!(lock(engine).effect())),
        "effects.set" => serde_json::from_value::<EffectSpec>(params)
            .map_err(invalid_params)
            .and_then(|spec| spec.validate().map(|()| spec).map_err(invalid_params))
            .map(|spec| {
                lock(engine).set_effect(spec);
                json!(true)
            }),
        "custom_effects.list"
        | "custom_effects.save"
        | "custom_effects.get"
        | "custom_effects.delete" => custom_effect_call(engine, method, &params),
        "color.set" => params
            .get("color")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_params("expected {\"color\": \"#rrggbb\"}"))
            .and_then(|s| s.parse::<Rgb>().map_err(invalid_params))
            .map(|color| {
                lock(engine).set_effect(EffectSpec::Solid { color });
                json!(true)
            }),
        "devices.set_region" => {
            let id = params.get("id").and_then(Value::as_str);
            let region = serde_json::from_value::<freergb_core::Region>(params.clone())
                .ok()
                .filter(freergb_core::Region::is_valid);
            match (id, region) {
                (Some(id), Some(r)) => {
                    if lock(engine).set_device_region(id, r) {
                        Ok(json!(true))
                    } else {
                        Err((-32004, format!("no device with id {id:?}")))
                    }
                }
                _ => Err(invalid_params(
                    "expected {\"id\", \"x\", \"y\" in 0..=1, \"w\", \"h\" in 0.01..=1}",
                )),
            }
        }
        "devices.set_brightness" => {
            let id = params.get("id").and_then(Value::as_str);
            let value = params
                .get("value")
                .and_then(Value::as_f64)
                .filter(|v| (0.0..=1.0).contains(v));
            match (id, value) {
                (Some(id), Some(v)) => {
                    if lock(engine).set_device_brightness(id, v as f32) {
                        Ok(json!(true))
                    } else {
                        Err((-32004, format!("no device with id {id:?}")))
                    }
                }
                _ => Err(invalid_params(
                    "expected {\"id\": \"…\", \"value\": 0.0..=1.0}",
                )),
            }
        }
        "brightness.set" => params
            .get("value")
            .and_then(Value::as_f64)
            .filter(|v| (0.0..=1.0).contains(v))
            .ok_or_else(|| invalid_params("expected {\"value\": 0.0..=1.0}"))
            .map(|v| {
                lock(engine).set_brightness(v as f32);
                json!(true)
            }),
        "profiles.list" | "profiles.save" | "profiles.load" | "profiles.delete" => {
            profile_call(engine, method, &params)
        }
        "daemon.shutdown" => {
            shutdown = true;
            Ok(json!(true))
        }
        "daemon.shutdown_if" => {
            if let Some(expected) = params["expected_executable"]
                .as_str()
                .filter(|p| std::path::Path::new(p).is_absolute())
            {
                let expected = std::path::Path::new(expected).canonicalize().ok();
                let actual = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.canonicalize().ok());
                shutdown = match (expected, actual) {
                    (Some(expected), Some(actual)) => {
                        #[cfg(windows)]
                        {
                            expected.to_string_lossy().to_lowercase()
                                == actual.to_string_lossy().to_lowercase()
                        }
                        #[cfg(not(windows))]
                        {
                            expected == actual
                        }
                    }
                    _ => false,
                };
                Ok(json!(shutdown))
            } else {
                Err((
                    -32602,
                    "expected_executable must be an absolute path".into(),
                ))
            }
        }
        other => Err((-32601, format!("method not found: {other}"))),
    };

    let resp = match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err((code, msg)) => error(id, code, msg),
    };
    Reply {
        line: resp.to_string(),
        shutdown,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cooling_sessions_disabled_and_invalid_requests_use_rpc_errors() {
        use super::*;
        use freergb_core::Rgb;
        use freergb_effects::EffectSpec;
        use std::sync::{Arc, Mutex};
        let e: SharedEngine = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::BLACK },
            30,
        )));
        let status = call(&e, r#"{"id":1,"method":"cooling.sessions.status"}"#);
        assert_eq!(status["result"]["state"], "disabled");
        assert_eq!(status["result"]["available"], false);
        for (params, code) in [
            (serde_json::json!({}), -32602),
            (
                serde_json::json!({"mode":"calibration","control_id":"/lpc/nct6701d/0/control/2","rpm_id":"/lpc/nct6701d/0/fan/2","temperature_id":"/lpc/nct6701d/0/temperature/1","confirmed_fan":true}),
                -32006,
            ),
        ] {
            let reply = call(
                &e,
                &serde_json::json!({"id":2,"method":"cooling.sessions.start","params":params})
                    .to_string(),
            );
            assert_eq!(reply["error"]["code"], code);
        }
        assert_eq!(
            call(
                &e,
                r#"{"id":3,"method":"cooling.sessions.stop","params":{}}"#
            )["error"]["code"],
            -32602
        );
        assert_eq!(
            call(
                &e,
                r#"{"id":4,"method":"cooling.sessions.stop","params":{"job_id":1}}"#
            )["error"]["code"],
            -32006
        );
    }
    #[test]
    fn guarded_shutdown_requires_the_actual_executable() {
        let e: SharedEngine = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::BLACK },
            30,
        )));
        let current = std::env::current_exe().unwrap();
        for (expected, stop) in [
            (current.clone(), true),
            (current.parent().unwrap().to_path_buf(), false),
        ] {
            let reply = handle_line(&e, &json!({"id":1,"method":"daemon.shutdown_if","params":{"expected_executable":expected}}).to_string());
            assert_eq!(reply.shutdown, stop);
            assert_eq!(
                serde_json::from_str::<Value>(&reply.line).unwrap()["result"],
                json!(stop)
            );
        }
        let reply = handle_line(
            &e,
            r#"{"id":1,"method":"daemon.shutdown_if","params":{"expected_executable":"relative.exe"}}"#,
        );
        assert!(!reply.shutdown);
        assert!(reply.line.contains("-32602"));
    }
    use super::*;
    use crate::engine::Engine;
    use freergb_core::LedLayout;
    use freergb_device_api::VirtualDevice;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    fn call(e: &SharedEngine, req: &str) -> Value {
        serde_json::from_str(&handle_line(e, req).line).unwrap()
    }

    #[test]
    fn channel_annotations_do_not_enable_hardware_and_invalid_saves_preserve_state() {
        let e = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::BLACK },
            30,
        )));
        let read = r#"{"id":1,"method":"cooling.bindings.get"}"#;
        let initial = call(&e, read)["result"].clone();
        assert_eq!(initial["config"], json!({"version":1,"bindings":[]}));
        let config = json!({"version":1,"bindings":[{
            "provider":"mainboard.lhm","device_id":"board","control_id":"pwm-7",
            "tachometer_id":"rpm-2","header":"CPU_FAN","role":"fan","electrical_mode":"pwm"
        }]});
        let request = |config| {
            json!({"id":2,"method":"cooling.bindings.save","params":{"config":config}}).to_string()
        };
        assert_eq!(call(&e, &request(config.clone()))["result"], true);
        let current = call(&e, read)["result"].clone();
        assert_eq!(current["assignments"][0]["state"], "unavailable");
        assert_eq!(current["assignments"][0]["hardware_verified"], false);
        assert_eq!(current["assignments"][0]["writable"], false);
        let mut invalid = config;
        invalid["bindings"][0]["header"] = json!("");
        assert_eq!(call(&e, &request(invalid))["error"]["code"], -32602);
        assert_eq!(call(&e, read)["result"], current);
        assert!(!lock(&e).mainboard.summary().control_enabled);
    }

    #[test]
    fn calibration_preview_remains_stateless_and_pumps_are_excluded() {
        let e = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::BLACK },
            30,
        )));
        let request = |role| {
            json!({"id":1,"method":"cooling.calibration.preview","params":{"role":role,"critical_c":85,"samples":[{"at_ms":0,"applied_sequence":1,"rpm":0,"rpm_age_ms":0,"temperature_c":40,"temperature_age_ms":0}]}}).to_string()
        };
        let out = call(&e, &request("fan"))["result"].clone();
        assert_eq!(out["simulated"], true);
        assert_eq!(out["hardware_control"], false);
        assert_eq!(out["final_output"]["phase"], "restoring");
        assert_eq!(out["final_output"]["duty_percent"], 100);
        assert_eq!(call(&e, &request("pump"))["error"]["code"], -32602);
        assert!(!lock(&e).mainboard.summary().control_enabled);
    }

    #[test]
    fn keyboard_api_is_additive_and_preview_never_enables_capture() {
        let e = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::BLUE },
            30,
        )));
        let r = call(&e, r#"{"id":1,"method":"keyboard.reactive.get"}"#);
        assert_eq!(r["result"]["config"]["enabled"], false);
        assert_eq!(
            call(
                &e,
                r#"{"id":2,"method":"keyboard.reactive.preview","params":{"usage":4}}"#
            )["result"],
            true
        );
        assert_eq!(
            call(
                &e,
                r#"{"id":3,"method":"keyboard.reactive.preview","params":{"usage":65535}}"#
            )["error"]["code"],
            -32602
        );
        assert_eq!(
            call(
                &e,
                r#"{"id":4,"method":"keyboard.reactive.configure","params":{"config":{}}}"#
            )["error"]["code"],
            -32602
        );
        let r = call(&e, r#"{"id":5,"method":"keyboard.reactive.disable"}"#);
        assert_eq!(
            r["result"],
            json!({"stopped":true,"persisted":false,"error":null})
        );
        assert_eq!(lock(&e).keyboard.status()["state"], "disabled");
        assert_eq!(lock(&e).effect(), &EffectSpec::Solid { color: Rgb::BLUE });
    }

    #[test]
    fn invalid_effects_and_unknown_plugins_do_not_mutate_engine() {
        let initial = EffectSpec::Solid { color: Rgb::GREEN };
        let e = Arc::new(Mutex::new(Engine::new(initial.clone(), 30)));
        for spec in [
            json!({"type":"rainbow_wave", "speed":1e38, "scale":1}),
            json!({"type":"layers", "layers":[{"effect":{"type":"color_cycle", "period_ms":0}}]}),
        ] {
            let req = json!({"id":1,"method":"effects.set","params":spec});
            assert_eq!(call(&e, &req.to_string())["error"]["code"], -32602);
            assert_eq!(*lock(&e).effect(), initial);
        }
        assert_eq!(
            call(&e, r#"{"id":2,"method":"plugins.list"}"#)["result"],
            json!([])
        );
        assert_eq!(
            call(
                &e,
                r#"{"id":3,"method":"plugins.start","params":{"id":"not-trusted"}}"#
            )["error"]["code"],
            -32005
        );
        assert_eq!(
            call(&e, r#"{"id":4,"method":"plugins.stop","params":{}}"#)["error"]["code"],
            -32602
        );
    }

    #[test]
    fn color_brightness_and_devices() {
        let mut engine = Engine::new(EffectSpec::Solid { color: Rgb::BLACK }, 30);
        let dev = VirtualDevice::new("api test", LedLayout::strip(2));
        let log = dev.frame_log();
        engine.add_device(Box::new(dev));
        let e: SharedEngine = Arc::new(Mutex::new(engine));

        let r = call(
            &e,
            r##"{"jsonrpc":"2.0","id":1,"method":"color.set","params":{"color":"#00ff00"}}"##,
        );
        assert_eq!(r["result"], json!(true));
        let r = call(
            &e,
            r#"{"jsonrpc":"2.0","id":2,"method":"brightness.set","params":{"value":0.5}}"#,
        );
        assert_eq!(r["id"], json!(2));

        // Devices run on worker threads: wait for the connect, then the frame.
        let deadline = Instant::now() + std::time::Duration::from_secs(2);
        while lock(&e).devices()[0].status != freergb_device_api::DeviceStatus::Connected {
            assert!(Instant::now() < deadline, "device did not connect");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        lock(&e).tick(Instant::now());
        while log.total() == 0 {
            assert!(Instant::now() < deadline, "frame did not arrive");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(log.last().unwrap().colors()[0], Rgb::new(0, 128, 0));

        let r = call(&e, r#"{"jsonrpc":"2.0","id":3,"method":"devices.list"}"#);
        assert_eq!(r["result"][0]["info"]["id"], json!("virtual:api-test"));
        assert_eq!(r["result"][0]["status"], json!("Connected"));
    }

    #[test]
    fn device_regions_split_a_gradient() {
        let mut engine = Engine::new(
            EffectSpec::Gradient {
                from: Rgb::RED,
                to: Rgb::BLUE,
                angle_deg: 0.0,
            },
            30,
        );
        let left = VirtualDevice::new("left", LedLayout::strip(1));
        let right = VirtualDevice::new("right", LedLayout::strip(1));
        let (ll, rl) = (left.frame_log(), right.frame_log());
        engine.add_device(Box::new(left));
        engine.add_device(Box::new(right));
        let e: SharedEngine = Arc::new(Mutex::new(engine));

        let r = call(
            &e,
            r#"{"id":1,"method":"devices.set_region","params":{"id":"virtual:left","x":0,"y":0,"w":0.1,"h":1}}"#,
        );
        assert_eq!(r["result"], json!(true));
        call(
            &e,
            r#"{"id":2,"method":"devices.set_region","params":{"id":"virtual:right","x":0.9,"y":0,"w":0.1,"h":1}}"#,
        );
        let bad = call(
            &e,
            r#"{"id":3,"method":"devices.set_region","params":{"id":"virtual:left","x":0,"y":0,"w":0,"h":1}}"#,
        );
        assert_eq!(bad["error"]["code"], json!(-32602));
        assert_eq!(
            call(&e, r#"{"id":4,"method":"devices.list"}"#)["result"][1]["region"]["x"],
            json!(0.9f32)
        );

        let deadline = Instant::now() + std::time::Duration::from_secs(2);
        while lock(&e)
            .devices()
            .iter()
            .any(|d| d.status != freergb_device_api::DeviceStatus::Connected)
        {
            assert!(Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        lock(&e).tick(Instant::now());
        while ll.total() == 0 || rl.total() == 0 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let (l, r) = (
            ll.last().unwrap().colors()[0],
            rl.last().unwrap().colors()[0],
        );
        assert!(l.r > 200 && l.b < 60, "left device shows the red end: {l}");
        assert!(
            r.b > 200 && r.r < 60,
            "right device shows the blue end: {r}"
        );
    }

    #[test]
    fn custom_effects_library_and_validation() {
        let root = std::env::temp_dir().join(format!("freergb-api-custom-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let e: SharedEngine = Arc::new(Mutex::new(
            Engine::new(EffectSpec::Solid { color: Rgb::BLACK }, 30)
                .with_profiles_dir(root.join("profiles")),
        ));
        let nested = r#"{"id":1,"method":"effects.set","params":{"type":"layers","layers":[{"effect":{"type":"layers","layers":[]}}]}}"#;
        assert_eq!(call(&e, nested)["error"]["code"], json!(-32602));

        let save = r#"{"id":2,"method":"custom_effects.save","params":{"name":"Party","effect":{"type":"layers","layers":[{"effect":{"type":"solid","color":{"r":9,"g":9,"b":9}},"blend":"add","opacity":0.5}]}}}"#;
        assert_eq!(call(&e, save)["result"], json!(true));
        assert_eq!(
            call(&e, r#"{"id":3,"method":"custom_effects.list"}"#)["result"],
            json!(["Party"])
        );
        let got = call(
            &e,
            r#"{"id":4,"method":"custom_effects.get","params":{"name":"Party"}}"#,
        );
        assert_eq!(got["result"]["layers"][0]["blend"], json!("add"));
        let set = json!({"id": 5, "method": "effects.set", "params": got["result"]}).to_string();
        assert_eq!(call(&e, &set)["result"], json!(true));
        assert_eq!(
            call(&e, r#"{"id":6,"method":"effects.get"}"#)["result"]["type"],
            json!("layers")
        );
        let missing = call(
            &e,
            r#"{"id":7,"method":"custom_effects.get","params":{"name":"Nope"}}"#,
        );
        assert_eq!(missing["error"]["code"], json!(-32004));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn per_device_brightness() {
        let mut engine = Engine::new(EffectSpec::Solid { color: Rgb::WHITE }, 30);
        let dev = VirtualDevice::new("dimmed", LedLayout::strip(1));
        let log = dev.frame_log();
        engine.add_device(Box::new(dev));
        let e: SharedEngine = Arc::new(Mutex::new(engine));

        let r = call(
            &e,
            r#"{"id":1,"method":"devices.set_brightness","params":{"id":"virtual:dimmed","value":0.5}}"#,
        );
        assert_eq!(r["result"], json!(true));
        let r = call(
            &e,
            r#"{"id":2,"method":"devices.set_brightness","params":{"id":"nope","value":0.5}}"#,
        );
        assert_eq!(r["error"]["code"], json!(-32004));
        let r = call(
            &e,
            r#"{"id":3,"method":"devices.set_brightness","params":{"id":"virtual:dimmed","value":2}}"#,
        );
        assert_eq!(r["error"]["code"], json!(-32602));
        assert_eq!(
            call(&e, r#"{"id":4,"method":"devices.list"}"#)["result"][0]["brightness"],
            json!(0.5)
        );

        // Global 0.5 x device 0.5 = 0.25 -> 64.
        call(
            &e,
            r#"{"id":5,"method":"brightness.set","params":{"value":0.5}}"#,
        );
        let deadline = Instant::now() + std::time::Duration::from_secs(2);
        while lock(&e).devices()[0].status != freergb_device_api::DeviceStatus::Connected {
            assert!(Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        lock(&e).tick(Instant::now());
        while log.total() == 0 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(log.last().unwrap().colors()[0], Rgb::new(64, 64, 64));
    }

    #[test]
    fn profiles_round_trip_over_api() {
        let dir = std::env::temp_dir().join(format!("freergb-api-profiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let e: SharedEngine = Arc::new(Mutex::new(
            Engine::new(EffectSpec::Solid { color: Rgb::BLACK }, 30).with_profiles_dir(dir.clone()),
        ));

        call(
            &e,
            r##"{"id":1,"method":"color.set","params":{"color":"#ff8800"}}"##,
        );
        call(
            &e,
            r#"{"id":2,"method":"brightness.set","params":{"value":0.25}}"#,
        );
        let r = call(
            &e,
            r#"{"id":3,"method":"profiles.save","params":{"name":"Abend"}}"#,
        );
        assert_eq!(r["result"], json!(true));

        call(
            &e,
            r##"{"id":4,"method":"color.set","params":{"color":"#0000ff"}}"##,
        );
        call(
            &e,
            r#"{"id":5,"method":"brightness.set","params":{"value":1.0}}"#,
        );
        assert_eq!(
            call(&e, r#"{"id":6,"method":"profiles.list"}"#)["result"],
            json!(["Abend"])
        );

        call(
            &e,
            r#"{"id":7,"method":"profiles.load","params":{"name":"Abend"}}"#,
        );
        let fx = call(&e, r#"{"id":8,"method":"effects.get"}"#);
        assert_eq!(fx["result"]["color"], json!({"r": 255, "g": 136, "b": 0}));
        let d = call(&e, r#"{"id":9,"method":"diagnostics.get"}"#);
        assert_eq!(d["result"]["brightness"], json!(0.25));

        let r = call(
            &e,
            r#"{"id":10,"method":"profiles.load","params":{"name":"../x"}}"#,
        );
        assert_eq!(r["error"]["code"], json!(-32602));
        let r = call(
            &e,
            r#"{"id":11,"method":"profiles.load","params":{"name":"Fehlt"}}"#,
        );
        assert_eq!(r["error"]["code"], json!(-32004));
        let r = call(
            &e,
            r#"{"id":12,"method":"profiles.delete","params":{"name":"Abend"}}"#,
        );
        assert_eq!(r["result"], json!(true));
        let _ = std::fs::remove_dir_all(dir);

        let off: SharedEngine = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::BLACK },
            30,
        )));
        assert_eq!(
            call(&off, r#"{"id":1,"method":"profiles.list"}"#)["error"]["code"],
            json!(-32003)
        );
    }

    #[test]
    fn effects_set_and_errors() {
        let e: SharedEngine = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::BLACK },
            30,
        )));
        let r = call(
            &e,
            r#"{"jsonrpc":"2.0","id":1,"method":"effects.set","params":{"type":"rainbow_wave","speed":0.2,"scale":2.0}}"#,
        );
        assert_eq!(r["result"], json!(true));
        assert_eq!(
            call(&e, r#"{"id":2,"method":"effects.get"}"#)["result"]["type"],
            json!("rainbow_wave")
        );

        assert_eq!(call(&e, "not json")["error"]["code"], json!(-32700));
        assert_eq!(call(&e, r#"{"id":3}"#)["error"]["code"], json!(-32600));
        assert_eq!(
            call(&e, r#"{"id":4,"method":"nope"}"#)["error"]["code"],
            json!(-32601)
        );
        let r = call(
            &e,
            r#"{"id":5,"method":"brightness.set","params":{"value":3}}"#,
        );
        assert_eq!(r["error"]["code"], json!(-32602));
        assert!(handle_line(&e, r#"{"id":6,"method":"daemon.shutdown"}"#).shutdown);
    }
}
