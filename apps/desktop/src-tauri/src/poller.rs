//! Polls daemon state and emits only the slices that changed.
//!
//! The daemon has no push channel yet, so this mirrors the Classic GUI's 1 s
//! poll, but slows down while the window is hidden or minimized and lets the
//! frontend skip slices no visible view needs.

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const VISIBLE_INTERVAL: Duration = Duration::from_secs(1);
pub const HIDDEN_INTERVAL: Duration = Duration::from_secs(5);
pub const EVENT: &str = "state";

/// Slices that are always polled while connected (shell, status, banners).
const CORE: &[(&str, &str)] = &[
    ("devices", "devices.list"),
    ("diagnostics", "diagnostics.get"),
    ("effect", "effects.get"),
    // Device setup: first-run wizard and the new-devices panel.
    ("modules_setup", "modules.status"),
];

/// Slices polled only while a view asks for them.
pub const OPTIONAL: &[(&str, &str)] = &[
    ("monitoring_sensors", "monitoring.sensors.list"),
    ("providers", "cooling.providers.list"),
    ("cooling_channels", "cooling.channels.list"),
    ("cooling_session", "cooling.sessions.status"),
    ("cooling_bindings", "cooling.bindings.get"),
    ("cooling_background", "cooling.background.get"),
    ("cooling_support", "cooling.support.get"),
    ("cooling_profiles", "cooling.profiles.list"),
    ("keyboard_reactive", "keyboard.reactive.get"),
    ("profiles", "profiles.list"),
    ("custom_effects", "custom_effects.list"),
    ("plugins", "plugins.list"),
    ("curves", "cooling.curves.list"),
    ("macros", "macros.list"),
    ("macro_status", "macros.status"),
    ("sensor_values", "cooling.sensors.values"),
    ("sensor_graph", "cooling.sensors.get"),
    ("sensor_sources", "cooling.sensors.sources"),
    ("cooling_status", "cooling.status"),
];

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Connection {
    pub connected: bool,
    pub error: Option<String>,
    pub daemon_version: Option<String>,
    pub api_version: Option<u64>,
}

#[derive(Serialize, Clone)]
pub struct SliceEvent<'a> {
    pub slice: &'a str,
    pub value: &'a Value,
}

#[derive(Default)]
pub struct Shared {
    /// Latest value per slice, also served to a freshly loaded frontend.
    pub slices: BTreeMap<String, Value>,
    /// Optional slices currently requested by the frontend.
    pub wanted: BTreeSet<String>,
}

/// Stores `value` if it differs; returns whether it changed.
pub fn update(slices: &mut BTreeMap<String, Value>, slice: &str, value: Value) -> bool {
    if slices.get(slice) == Some(&value) {
        return false;
    }
    slices.insert(slice.to_owned(), value);
    true
}

fn invalidate(slices: &mut BTreeMap<String, Value>) -> Vec<(String, Value)> {
    slices
        .iter_mut()
        .filter(|(key, _)| key.as_str() != "connection")
        .filter_map(|(key, value)| {
            if value.is_null() {
                return None;
            }
            *value = Value::Null;
            Some((key.clone(), Value::Null))
        })
        .collect()
}

pub struct Deps<C, E, H> {
    pub call: C,
    pub emit: E,
    pub hidden: H,
}

/// Fetches one round. Returns `None` if the daemon is unreachable.
fn fetch<C: FnMut(&str) -> Result<Value, freergb_client::ClientError>>(
    call: &mut C,
    wanted: &BTreeSet<String>,
    version: &mut Option<Value>,
) -> Result<Vec<(&'static str, Value)>, String> {
    if version.is_none() {
        *version = Some(call("daemon.version").map_err(|e| e.to_string())?);
    }
    let mut out = Vec::new();
    for (slice, method) in CORE {
        out.push((*slice, call(method).map_err(|e| e.to_string())?));
    }
    for (slice, method) in OPTIONAL.iter().filter(|(s, _)| wanted.contains(*s)) {
        match call(method) {
            Ok(value) => out.push((*slice, value)),
            // e.g. profiles without storage (`--no-state`): an explicit null.
            Err(freergb_client::ClientError::Rpc { .. }) => out.push((*slice, Value::Null)),
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(out)
}

pub fn run<C, E, H>(shared: Arc<Mutex<Shared>>, wake: Receiver<()>, mut deps: Deps<C, E, H>)
where
    C: FnMut(&str) -> Result<Value, freergb_client::ClientError>,
    E: FnMut(&str, &Value),
    H: FnMut() -> bool,
{
    let mut version: Option<Value> = None;
    loop {
        let wanted = shared
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .wanted
            .clone();
        let result = fetch(&mut deps.call, &wanted, &mut version);
        let connection = match &result {
            Ok(_) => Connection {
                connected: true,
                error: None,
                daemon_version: version
                    .as_ref()
                    .and_then(|v| v["daemon_version"].as_str())
                    .map(str::to_owned),
                api_version: version.as_ref().and_then(|v| v["api_version"].as_u64()),
            },
            Err(error) => {
                // The daemon may come back as a different build.
                version = None;
                Connection {
                    connected: false,
                    error: Some(error.clone()),
                    daemon_version: None,
                    api_version: None,
                }
            }
        };
        let mut changed = Vec::new();
        {
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            let connection = json!(connection);
            if update(&mut s.slices, "connection", connection.clone()) {
                changed.push(("connection".to_owned(), connection));
            }
            // A disconnected daemon's cached readings must never look live.
            if result.is_err() {
                changed.extend(invalidate(&mut s.slices));
            }
            for (slice, value) in result.unwrap_or_default() {
                // A view may have unmounted while this blocking fetch ran.
                if OPTIONAL.iter().any(|(name, _)| *name == slice) && !s.wanted.contains(slice) {
                    continue;
                }
                if update(&mut s.slices, slice, value.clone()) {
                    changed.push((slice.to_owned(), value));
                }
            }
        }
        for (slice, value) in &changed {
            (deps.emit)(slice, value);
        }
        let interval = if (deps.hidden)() {
            HIDDEN_INTERVAL
        } else {
            VISIBLE_INTERVAL
        };
        match wake.recv_timeout(interval) {
            Ok(()) => while wake.try_recv().is_ok() {},
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn lost_connection_clears_all_cached_readings_once_then_repopulates() {
        let mut slices = BTreeMap::from([
            ("connection".into(), json!({"connected":false})),
            ("monitoring_sensors".into(), json!([{"value":42}])),
            ("cooling_session".into(), json!({"state":"executing"})),
        ]);
        assert_eq!(invalidate(&mut slices).len(), 2);
        assert_eq!(slices["connection"], json!({"connected":false}));
        assert!(slices["monitoring_sensors"].is_null());
        assert!(slices["cooling_session"].is_null());
        assert!(invalidate(&mut slices).is_empty());
        assert!(update(&mut slices, "monitoring_sensors", json!([])));
        assert_eq!(slices["monitoring_sensors"], json!([]));
    }

    #[test]
    fn optional_rpc_errors_are_unavailable_not_a_disconnection() {
        let mut version = None;
        let wanted = BTreeSet::from(["cooling_session".to_string()]);
        let result = fetch(
            &mut |method| {
                if method == "cooling.sessions.status" {
                    Err(freergb_client::ClientError::Rpc {
                        code: -32601,
                        message: "older daemon".into(),
                    })
                } else {
                    Ok(json!({}))
                }
            },
            &wanted,
            &mut version,
        )
        .unwrap();
        assert!(result.contains(&("cooling_session", Value::Null)));
        assert!(!result.iter().any(|(slice, _)| *slice == "cooling_channels"));
    }

    #[test]
    fn unchanged_slices_are_not_reported() {
        let mut slices = BTreeMap::new();
        assert!(update(&mut slices, "effect", json!({"type": "solid"})));
        assert!(!update(&mut slices, "effect", json!({"type": "solid"})));
        assert!(update(&mut slices, "effect", json!({"type": "breathing"})));
    }

    #[test]
    fn rounds_emit_changes_only_and_optional_slices_on_demand() {
        let shared = Arc::new(Mutex::new(Shared::default()));
        shared.lock().unwrap().wanted.insert("profiles".into());
        let (wake_tx, wake_rx) = mpsc::channel();
        let emitted = Arc::new(Mutex::new(Vec::<String>::new()));
        let rounds = Arc::new(Mutex::new(0));
        let (e, r) = (emitted.clone(), rounds.clone());
        let handle = std::thread::spawn({
            let shared = shared.clone();
            move || {
                run(
                    shared,
                    wake_rx,
                    Deps {
                        call: move |method: &str| {
                            Ok(match method {
                                "daemon.version" => {
                                    json!({"api_version": 1, "daemon_version": "0.1.0"})
                                }
                                "profiles.list" => json!(["a"]),
                                _ => json!([]),
                            })
                        },
                        emit: move |slice: &str, _: &Value| e.lock().unwrap().push(slice.into()),
                        hidden: move || {
                            *r.lock().unwrap() += 1;
                            false
                        },
                    },
                )
            }
        });
        wake_tx.send(()).unwrap();
        while *rounds.lock().unwrap() < 2 {
            std::thread::yield_now();
        }
        drop(wake_tx);
        handle.join().unwrap();
        let emitted = emitted.lock().unwrap().clone();
        assert_eq!(
            emitted,
            [
                "connection",
                "devices",
                "diagnostics",
                "effect",
                "modules_setup",
                "profiles"
            ]
        );
        assert!(!shared.lock().unwrap().slices.contains_key("plugins"));
    }
}
