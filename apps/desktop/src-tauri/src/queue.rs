//! Serialized daemon command queue. Every frontend request runs on one worker
//! thread in arrival order, so a burst of slider writes can never reach the
//! daemon out of order. Setter bursts are coalesced like the Classic GUI does.

use serde::Serialize;
use serde_json::Value;
use std::sync::mpsc::{self, Receiver, Sender};

/// Methods the frontend may call. Mirrors `docs/LOCAL_API.md`; daemon shutdown
/// is deliberately absent: closing or using the GUI never stops the service.
pub const ALLOWED: &[&str] = &[
    "marketplace.status",
    "marketplace.ratings.refresh",
    "marketplace.rate",
    "marketplace.report.prepare",
    "marketplace.refresh",
    "marketplace.install",
    "marketplace.uninstall",
    "daemon.version",
    "monitoring.sensors.list",
    "cooling.providers.list",
    "cooling.channels.list",
    "cooling.bindings.get",
    "cooling.bindings.save",
    "cooling.calibration.preview",
    "cooling.sessions.status",
    "cooling.sessions.start",
    "cooling.sessions.stop",
    "cooling.background.get",
    "cooling.background.apply",
    "cooling.support.get",
    "cooling.support.settings",
    "cooling.channels.unlock",
    "cooling.channels.confirm_pump",
    "cooling.channels.revoke",
    "cooling.profiles.list",
    "cooling.profiles.save",
    "cooling.profiles.delete",
    "cooling.profiles.activate",
    "keyboard.reactive.get",
    "keyboard.reactive.configure",
    "keyboard.reactive.disable",
    "keyboard.reactive.preview",
    "devices.list",
    "devices.set_region",
    "devices.set_brightness",
    "diagnostics.get",
    "diagnostics.export",
    "effects.list",
    "effects.default",
    "effects.get",
    "effects.set",
    "color.set",
    "brightness.set",
    "lighting.status",
    "lighting.pause",
    "cooling.fans.pause",
    "custom_effects.list",
    "custom_effects.get",
    "custom_effects.save",
    "custom_effects.delete",
    "profiles.list",
    "profiles.save",
    "profiles.load",
    "profiles.delete",
    "plugins.list",
    "modules.status",
    "onboarding.get",
    "onboarding.complete",
    "onboarding.reset",
    "safety.acknowledge",
    "safety.revoke",
    "modules.catalog.refresh",
    "modules.setup.start",
    "modules.setup.retry",
    "modules.setup.cancel",
    "modules.setup.dismiss",
    "modules.setup.complete",
    "modules.forget",
    "modules.rollback",
    "modules.light_test",
    "plugins.start",
    "plugins.stop",
    "cooling.status",
    "cooling.curves.list",
    "cooling.curves.save",
    "cooling.curves.delete",
    "cooling.preview",
    "cooling.sensors.get",
    "cooling.sensors.save",
    "cooling.sensors.sources",
    "cooling.sensors.values",
    "macros.list",
    "macros.save",
    "macros.delete",
    "macros.run",
    "macros.cancel",
    "macros.status",
];

/// High-frequency writes (slider drags) whose older queued values are obsolete.
/// Keep in sync with `is_setter` in `crates/freergb-gui/src/main.rs`.
fn is_setter(method: &str) -> bool {
    matches!(
        method,
        "brightness.set"
            | "color.set"
            | "effects.set"
            | "devices.set_region"
            | "devices.set_brightness"
    )
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CallError {
    /// Method not in [`ALLOWED`].
    Forbidden { method: String },
    /// The daemon answered with a JSON-RPC error.
    Rpc { code: i64, message: String },
    /// The daemon could not be reached or the reply was invalid.
    Transport { message: String },
    /// A newer value for the same setter replaced this one before it was sent.
    Superseded,
}

impl From<freergb_client::ClientError> for CallError {
    fn from(error: freergb_client::ClientError) -> Self {
        match error {
            freergb_client::ClientError::Rpc { code, message } => Self::Rpc { code, message },
            other => Self::Transport {
                message: other.to_string(),
            },
        }
    }
}

pub type Reply = Result<Value, CallError>;

pub struct Job {
    pub method: String,
    pub params: Value,
    pub reply: Sender<Reply>,
}

/// Adds `job` to `pending`; an older queued setter for the same method and
/// device after the last non-setter barrier is answered as superseded.
fn push_job(pending: &mut Vec<Job>, job: Job) {
    if is_setter(&job.method) {
        let start = pending
            .iter()
            .rposition(|p| !is_setter(&p.method))
            .map_or(0, |index| index + 1);
        let mut index = start;
        while index < pending.len() {
            if pending[index].method == job.method
                && pending[index].params.get("id") == job.params.get("id")
            {
                let old = pending.remove(index);
                let _ = old.reply.send(Err(CallError::Superseded));
            } else {
                index += 1;
            }
        }
    }
    pending.push(job);
}

/// Runs queued jobs until every sender is dropped. `after_batch` runs once per
/// drained batch (used to wake the state poller).
pub fn run(
    rx: Receiver<Job>,
    mut call: impl FnMut(&str, Value) -> Reply,
    mut after_batch: impl FnMut(),
) {
    while let Ok(job) = rx.recv() {
        let mut pending = vec![job];
        while let Ok(next) = rx.try_recv() {
            push_job(&mut pending, next);
        }
        for job in pending {
            let _ = job.reply.send(call(&job.method, job.params));
        }
        after_batch();
    }
}

pub fn channel() -> (Sender<Job>, Receiver<Job>) {
    mpsc::channel()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn job(method: &str, params: Value) -> (Job, Receiver<Reply>) {
        let (reply, rx) = mpsc::channel();
        (
            Job {
                method: method.into(),
                params,
                reply,
            },
            rx,
        )
    }

    #[test]
    fn setter_bursts_merge_per_device_after_the_last_barrier() {
        let mut pending = Vec::new();
        let (a, a_rx) = job("devices.set_brightness", json!({"id": "a", "value": 0.1}));
        let (b, _) = job("devices.set_brightness", json!({"id": "b", "value": 0.2}));
        let (barrier, _) = job("profiles.save", json!({"name": "x"}));
        let (a2, _) = job("devices.set_brightness", json!({"id": "a", "value": 0.3}));
        let (a3, _) = job("devices.set_brightness", json!({"id": "a", "value": 0.4}));
        for j in [a, b, barrier, a2, a3] {
            push_job(&mut pending, j);
        }
        let values: Vec<_> = pending.iter().map(|j| j.params["value"].clone()).collect();
        assert_eq!(values, [json!(0.1), json!(0.2), Value::Null, json!(0.4)]);
        // The value before the barrier is kept; it was not superseded.
        assert!(a_rx.try_recv().is_err());
    }

    #[test]
    fn superseded_jobs_are_answered_and_actions_never_merge() {
        let mut pending = Vec::new();
        let (first, first_rx) = job("brightness.set", json!({"value": 0.1}));
        let (second, _) = job("brightness.set", json!({"value": 0.9}));
        let (run1, _) = job("macros.run", json!({"name": "m"}));
        let (run2, _) = job("macros.run", json!({"name": "m"}));
        for j in [first, second, run1, run2] {
            push_job(&mut pending, j);
        }
        assert_eq!(first_rx.try_recv(), Ok(Err(CallError::Superseded)));
        let methods: Vec<_> = pending.iter().map(|j| j.method.as_str()).collect();
        assert_eq!(methods, ["brightness.set", "macros.run", "macros.run"]);
    }

    #[test]
    fn daemon_shutdown_is_not_allowed() {
        assert!(!ALLOWED.contains(&"daemon.shutdown"));
        assert!(!ALLOWED.contains(&"daemon.shutdown_if"));
    }
}
