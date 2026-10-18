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
        "effects.list" => Ok(json!(["solid", "rainbow_wave", "breathing"])),
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
        "brightness.set" => params
            .get("value")
            .and_then(Value::as_f64)
            .filter(|v| (0.0..=1.0).contains(v))
            .ok_or_else(|| invalid_params("expected {\"value\": 0.0..=1.0}"))
            .map(|v| {
                lock(engine).set_brightness(v as f32);
                json!(true)
            }),
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
