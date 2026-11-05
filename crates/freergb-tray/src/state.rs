//! Platform-independent tray state: what the menu shows, read from the daemon.
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayState {
    pub connected: bool,
    pub lighting_paused: bool,
    pub brightness: Option<f64>,
    pub profiles: Vec<String>,
    pub fans_available: bool,
    pub fans_paused: bool,
    pub fans_locked: bool,
    /// Background channels reported running by the protected service.
    pub fans_running: usize,
    pub fan_faults: usize,
    pub cooling_profiles: Vec<String>,
    pub active_cooling_profile: Option<String>,
    /// Current fan problems, one line each (background channels and sessions).
    pub fan_problems: Vec<String>,
    /// Interactive sessions currently driving a channel (stop-all target).
    pub live_sessions: usize,
}

fn names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().or_else(|| v["name"].as_str()).map(str::to_owned))
        .take(20)
        .collect()
}

fn short(id: &str) -> &str {
    id.rsplit_once("/control/").map_or(id, |(_, index)| index)
}

/// Background faults and interactive sessions that ended faulted or blocked.
fn problems(channels: &[Value], sessions: Option<&Value>) -> Vec<String> {
    let mut list: Vec<String> = channels
        .iter()
        .filter_map(|c| {
            let fault = c["fault"].as_str()?;
            let id = c["control_id"].as_str().unwrap_or("?");
            Some(format!(
                "Fan control {} returned to BIOS ({})",
                short(id),
                fault.replace('_', " ")
            ))
        })
        .collect();
    for job in sessions
        .and_then(|s| s["jobs"].as_array())
        .into_iter()
        .flatten()
    {
        let state = job["state"].as_str().unwrap_or("");
        if state == "faulted" || state == "blocked" {
            let id = job["control_id"].as_str().unwrap_or("?");
            let what = if state == "blocked" {
                "needs attention: return to BIOS not confirmed"
            } else {
                "returned to BIOS after a fault"
            };
            list.push(format!(
                "Fan control {} {} (job {})",
                short(id),
                what,
                job["job_id"]
            ));
        }
    }
    list.truncate(10);
    list
}

/// Sessions that still hold a channel (not idle, stopped or faulted).
fn live(sessions: Option<&Value>) -> usize {
    sessions
        .and_then(|s| s["jobs"].as_array())
        .into_iter()
        .flatten()
        .filter(|j| {
            matches!(
                j["state"].as_str(),
                Some("authorizing" | "executing" | "stopping")
            )
        })
        .count()
}

/// New problems since the previous state, plus a lost service.
pub fn alerts(previous: &TrayState, current: &TrayState) -> Vec<String> {
    let mut alerts: Vec<String> = current
        .fan_problems
        .iter()
        .filter(|p| !previous.fan_problems.contains(p))
        .cloned()
        .collect();
    if previous.connected && !current.connected {
        alerts.push("The freeRGB service stopped. Background fan curves keep running in the protected service.".into());
    }
    alerts
}

impl TrayState {
    /// Builds the state from daemon replies (`None` = call failed).
    #[allow(clippy::too_many_arguments)]
    pub fn from_replies(
        version: Option<Value>,
        lighting: Option<Value>,
        diagnostics: Option<Value>,
        profiles: Option<Value>,
        onboarding: Option<Value>,
        background: Option<Value>,
        cooling_profiles: Option<Value>,
        sessions: Option<Value>,
    ) -> Self {
        if version.is_none() {
            return Self::default();
        }
        let background = background.unwrap_or(Value::Null);
        let service = &background["service"];
        let channels = service["channels"].as_array().cloned().unwrap_or_default();
        let cooling = cooling_profiles.unwrap_or(Value::Null);
        Self {
            connected: true,
            lighting_paused: lighting.is_some_and(|l| l["paused"] == true),
            brightness: diagnostics.and_then(|d| d["brightness"].as_f64()),
            profiles: profiles.as_ref().map(names).unwrap_or_default(),
            fans_available: background["available"] == true,
            fans_paused: service["paused"] == true,
            fans_locked: onboarding.is_none_or(|o| o["safety_acknowledged"] != true),
            fans_running: channels.iter().filter(|c| c["state"] == "running").count(),
            fan_faults: channels.iter().filter(|c| !c["fault"].is_null()).count(),
            cooling_profiles: names(&cooling["profiles"]),
            active_cooling_profile: cooling["active"].as_str().map(str::to_owned),
            fan_problems: problems(&channels, sessions.as_ref()),
            live_sessions: live(sessions.as_ref()),
        }
    }

    pub fn status_line(&self) -> String {
        if !self.connected {
            return "freeRGB service is not running".into();
        }
        if self.lighting_paused {
            "Lighting paused".into()
        } else {
            "Lighting running".into()
        }
    }

    pub fn fan_line(&self) -> String {
        if !self.connected || !self.fans_available {
            "Fan control unavailable".into()
        } else if self.fans_locked {
            "Fans: locked until the safety notice is accepted".into()
        } else if self.fans_paused {
            "Fans: paused — BIOS controls all fans".into()
        } else if self.fan_faults > 0 {
            format!(
                "Fans: {} running, {} faulted",
                self.fans_running, self.fan_faults
            )
        } else if self.fans_running > 0 {
            format!(
                "Fans: {} curve{} running",
                self.fans_running,
                if self.fans_running == 1 { "" } else { "s" }
            )
        } else {
            "Fans: BIOS control".into()
        }
    }

    pub fn tooltip(&self) -> String {
        format!("freeRGB — {}\n{}", self.status_line(), self.fan_line())
    }

    /// Dimmed icon when nothing is actively controlled.
    pub fn dimmed(&self) -> bool {
        !self.connected || (self.lighting_paused && (self.fans_paused || self.fans_running == 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn counts_only_sessions_that_still_hold_a_channel() {
        let sessions = json!({"jobs":[{"state":"executing"},{"state":"authorizing"},{"state":"faulted","control_id":"x","job_id":1},{"state":"idle"}]});
        assert_eq!(live(Some(&sessions)), 2);
        assert_eq!(live(None), 0);
    }

    #[test]
    fn disconnected_service_shows_a_clear_state() {
        let state = TrayState::from_replies(None, None, None, None, None, None, None, None);
        assert!(!state.connected && state.dimmed());
        assert_eq!(state.status_line(), "freeRGB service is not running");
        assert_eq!(state.fan_line(), "Fan control unavailable");
        assert_eq!(
            state.tooltip(),
            "freeRGB — freeRGB service is not running\nFan control unavailable"
        );
    }

    #[test]
    fn missing_safety_reply_never_unlocks_fan_actions() {
        let state = TrayState::from_replies(
            Some(json!({"api_version":1})),
            None,
            None,
            None,
            None,
            Some(json!({"available":true,"service":{"channels":[]}})),
            None,
            None,
        );
        assert!(state.connected && state.fans_available && state.fans_locked);
        assert!(state.fan_line().contains("locked"));
    }

    #[test]
    fn replies_map_to_menu_state() {
        let state = TrayState::from_replies(
            Some(json!({"version":"0.1.0"})),
            Some(json!({"paused":true})),
            Some(json!({"brightness":0.5})),
            Some(json!(["Evening", "Gaming"])),
            Some(json!({"safety_acknowledged":true})),
            Some(
                json!({"available":true,"service":{"paused":false,"channels":[{"state":"running","fault":null},{"state":"faulted","fault":"rpm_implausible"}]}}),
            ),
            Some(json!({"profiles":[{"name":"Quiet","channels":2}],"active":"Quiet"})),
            Some(
                json!({"jobs":[{"job_id":3,"state":"faulted","control_id":"/lpc/nct/0/control/2"}]}),
            ),
        );
        assert_eq!(state.fan_problems.len(), 2);
        assert!(state.fan_problems[0].contains("rpm implausible"));
        let before = TrayState {
            connected: true,
            ..TrayState::default()
        };
        assert_eq!(alerts(&before, &state).len(), 2);
        assert!(alerts(&state, &state).is_empty(), "no repeats");
        assert_eq!(
            alerts(&state, &TrayState::default()).len(),
            1,
            "service lost"
        );
        assert_eq!(state.profiles, ["Evening", "Gaming"]);
        assert_eq!(state.brightness, Some(0.5));
        assert_eq!(state.cooling_profiles, ["Quiet"]);
        assert_eq!(state.active_cooling_profile.as_deref(), Some("Quiet"));
        assert_eq!(state.fan_line(), "Fans: 1 running, 1 faulted");
        assert_eq!(state.status_line(), "Lighting paused");
        assert!(!state.dimmed(), "a running fan curve is active control");
    }

    #[test]
    fn locked_and_paused_fans_are_explained() {
        let base = |onboarding: Value, paused: bool| {
            TrayState::from_replies(
                Some(json!({})),
                None,
                None,
                None,
                Some(onboarding),
                Some(json!({"available":true,"service":{"paused":paused,"channels":[]}})),
                None,
                None,
            )
        };
        assert!(base(json!({"safety_acknowledged":false}), false)
            .fan_line()
            .contains("locked"));
        assert!(base(json!({"safety_acknowledged":true}), true)
            .fan_line()
            .contains("paused"));
        assert_eq!(
            base(json!({"safety_acknowledged":true}), false).fan_line(),
            "Fans: BIOS control"
        );
    }
}
