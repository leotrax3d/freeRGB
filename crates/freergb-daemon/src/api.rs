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
        ProfileError::InvalidName(_) => (-32602, e.to_string()),
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
        "diagnostics.get" => Ok(json!(lock(engine).diagnostics())),
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
            .map(|spec| {
                lock(engine).set_effect(spec);
                json!(true)
            }),
        "color.set" => params
            .get("color")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_params("expected {\"color\": \"#rrggbb\"}"))
            .and_then(|s| s.parse::<Rgb>().map_err(invalid_params))
            .map(|color| {
                lock(engine).set_effect(EffectSpec::Solid { color });
                json!(true)
            }),
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
