//! Focus-scoped recording. Only explicit GUI events enter this volatile buffer.
use freergb_control::{macro_key, MacroDefinition, MacroStep};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    KeyDown { key: String },
    KeyUp { key: String },
    Text { text: String },
    Heartbeat,
    PrivacyStop,
}

struct Session {
    id: String,
    started: Instant,
    last_seen: Instant,
    last_event: Instant,
    steps: Vec<MacroStep>,
    held: BTreeSet<String>,
}

#[derive(Default)]
pub struct Recorder {
    session: Option<Session>,
    reason: Option<String>,
}

impl Recorder {
    pub fn active(&self) -> bool {
        self.session.is_some()
    }

    pub fn tick(&mut self, now: Instant) {
        if let Some(s) = &self.session {
            let reason = if now.saturating_duration_since(s.started) >= Duration::from_secs(30) {
                Some("timeout")
            } else if now.saturating_duration_since(s.last_seen) >= Duration::from_secs(3) {
                Some("focus heartbeat expired")
            } else {
                None
            };
            if let Some(reason) = reason {
                self.discard(reason);
            }
        }
    }

    fn discard(&mut self, reason: &str) {
        self.session = None;
        self.reason = Some(reason.into());
    }

    pub fn call(&mut self, method: &str, params: &Value, now: Instant) -> Result<Value, String> {
        self.tick(now);
        if method == "macros.record.status" {
            return Ok(
                json!({"recording":self.active(),"steps":self.session.as_ref().map_or(0, |s|s.steps.len()),"reason":self.reason,"scope":"focused recording pad only"}),
            );
        }
        let id = params["session"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 128)
            .ok_or("expected session (1..128 bytes)")?;
        let allowed: &[&str] = match method {
            "macros.record.start" => &["session"],
            "macros.record.stop" => &["session", "discard"],
            "macros.record.event" => &["session", "event"],
            _ => return Err("unknown recording method".into()),
        };
        if params
            .as_object()
            .is_none_or(|p| p.keys().any(|k| !allowed.contains(&k.as_str())))
        {
            return Err("unknown recording parameter".into());
        }
        if method == "macros.record.stop" && params.get("discard").is_some_and(|v| !v.is_boolean())
        {
            return Err("discard must be boolean".into());
        }
        match method {
            "macros.record.start" => {
                if self.active() {
                    return Err("a recording is already active".into());
                }
                if params.as_object().is_none_or(|p| p.len() != 1) {
                    return Err("start expects only session".into());
                }
                self.session = Some(Session {
                    id: id.into(),
                    started: now,
                    last_seen: now,
                    last_event: now,
                    steps: vec![],
                    held: BTreeSet::new(),
                });
                self.reason = None;
                Ok(json!(true))
            }
            "macros.record.stop" => {
                if self.session.as_ref().is_none_or(|s| s.id != id) {
                    return Err("recording session is not active".into());
                }
                let s = self.session.take().unwrap();
                if params["discard"] == true {
                    self.reason = Some("discarded".into());
                    return Ok(Value::Null);
                }
                let mut steps = s.steps;
                for key in s.held {
                    steps.push(MacroStep::KeyUp { key });
                }
                let definition = MacroDefinition { repeat: 1, steps };
                definition.validate()?;
                self.reason = Some("stopped; draft returned, not saved".into());
                Ok(json!(definition))
            }
            "macros.record.event" => {
                if self.session.as_ref().is_none_or(|s| s.id != id) {
                    return Err("recording session is not active".into());
                }
                let event = serde_json::from_value::<Event>(params["event"].clone())
                    .map_err(|e| e.to_string());
                let result = event.and_then(|event| self.event(event, now));
                if result.is_err() {
                    self.discard("invalid event or recording limit");
                }
                result.map(|()| json!(true))
            }
            _ => Err("unknown recording method".into()),
        }
    }

    fn event(&mut self, event: Event, now: Instant) -> Result<(), String> {
        if matches!(event, Event::PrivacyStop) {
            self.discard("focus lost or sensitive field");
            return Ok(());
        }
        let s = self.session.as_mut().unwrap();
        s.last_seen = now;
        let step = match event {
            Event::Heartbeat => return Ok(()),
            Event::KeyDown { key } => {
                if !macro_key(&key) {
                    return Err("unsupported recording key".into());
                }
                if !s.held.insert(key.clone()) {
                    return Ok(());
                } // Ignore autorepeat.
                MacroStep::KeyDown { key }
            }
            Event::KeyUp { key } => {
                if !s.held.remove(&key) {
                    return Err("release without press".into());
                }
                MacroStep::KeyUp { key }
            }
            Event::Text { text } => {
                if text.is_empty() || text.len() > 1024 || !s.held.is_empty() {
                    return Err("invalid text event".into());
                }
                MacroStep::Text { text }
            }
            Event::PrivacyStop => unreachable!(),
        };
        let delay = now
            .saturating_duration_since(s.last_event)
            .as_millis()
            .min(30_000) as u32;
        // Coalesce adjacent text arriving within one input burst.
        if delay < 20 {
            if let (Some(MacroStep::Text { text: previous }), MacroStep::Text { text }) =
                (s.steps.last_mut(), &step)
            {
                if previous.len() + text.len() > 1024 {
                    return Err("text limit".into());
                }
                previous.push_str(text);
                s.last_event = now;
                return Ok(());
            }
        }
        if !s.steps.is_empty() {
            let mut remaining = delay;
            while remaining > 0 {
                let ms = remaining.min(10_000);
                s.steps.push(MacroStep::Delay { ms });
                remaining -= ms;
            }
        }
        s.steps.push(step);
        s.last_event = now;
        if s.steps.len() + s.held.len() > 128 {
            return Err("recording is limited to 128 steps".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delay_chunks_and_held_keys_close_into_a_valid_draft() {
        let now = Instant::now();
        let mut r = Recorder::default();
        r.call("macros.record.start", &json!({"session":"s"}), now)
            .unwrap();
        r.call(
            "macros.record.event",
            &json!({"session":"s","event":{"type":"key_down","key":"ctrl"}}),
            now,
        )
        .unwrap();
        for seconds in 1..=11 {
            r.call(
                "macros.record.event",
                &json!({"session":"s","event":{"type":"heartbeat"}}),
                now + Duration::from_secs(seconds),
            )
            .unwrap();
        }
        r.call(
            "macros.record.event",
            &json!({"session":"s","event":{"type":"key_down","key":"a"}}),
            now + Duration::from_secs(11),
        )
        .unwrap();
        let draft = r
            .call(
                "macros.record.stop",
                &json!({"session":"s"}),
                now + Duration::from_secs(11),
            )
            .unwrap();
        assert_eq!(draft["steps"][1], json!({"type":"delay","ms":10000}));
        assert_eq!(draft["steps"][2], json!({"type":"delay","ms":1000}));
        serde_json::from_value::<MacroDefinition>(draft)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn pairs_repeat_coalescing_and_unsaved_draft() {
        let now = Instant::now();
        let mut r = Recorder::default();
        r.call("macros.record.start", &json!({"session":"test"}), now)
            .unwrap();
        for event in [
            json!({"type":"key_down","key":"a"}),
            json!({"type":"key_down","key":"a"}),
            json!({"type":"key_up","key":"a"}),
            json!({"type":"text","text":"hi"}),
            json!({"type":"text","text":"!"}),
        ] {
            r.call(
                "macros.record.event",
                &json!({"session":"test","event":event}),
                now,
            )
            .unwrap();
        }
        let result = r
            .call("macros.record.stop", &json!({"session":"test"}), now)
            .unwrap();
        assert_eq!(result["steps"].as_array().unwrap().len(), 3);
        assert_eq!(result["steps"][2]["text"], "hi!");
        assert!(!r.active());
    }
    #[test]
    fn privacy_timeout_invalid_and_foreign_sessions_discard() {
        for cause in ["privacy", "timeout", "heartbeat", "invalid", "limit"] {
            let now = Instant::now();
            let mut r = Recorder::default();
            r.call("macros.record.start", &json!({"session":"test"}), now)
                .unwrap();
            assert!(r
                .call("macros.record.stop", &json!({"session":"other"}), now)
                .is_err());
            match cause {
                "privacy" => {
                    r.call(
                        "macros.record.event",
                        &json!({"session":"test","event":{"type":"privacy_stop"}}),
                        now,
                    )
                    .unwrap();
                }
                "timeout" => r.tick(now + Duration::from_secs(30)),
                "heartbeat" => r.tick(now + Duration::from_secs(3)),
                "invalid" => {
                    assert!(r
                        .call(
                            "macros.record.event",
                            &json!({"session":"test","event":{"type":"key_up","key":"a"}}),
                            now
                        )
                        .is_err());
                }
                _ => {
                    for i in 0..129 {
                        let _ = r.call(
                            "macros.record.event",
                            &json!({"session":"test","event":{"type":if i % 2 == 0 {"key_down"} else {"key_up"},"key":"a"}}),
                            now,
                        );
                    }
                }
            }
            assert!(!r.active(), "{cause}");
            assert!(r
                .call("macros.record.status", &Value::Null, now)
                .unwrap()
                .get("steps")
                .is_some());
        }
    }
}
