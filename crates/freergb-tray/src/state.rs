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

impl TrayState {
    /// Builds the state from daemon replies (`None` = call failed).
    pub fn from_replies(
        version: Option<Value>,
        lighting: Option<Value>,
        diagnostics: Option<Value>,
        profiles: Option<Value>,
        onboarding: Option<Value>,
        background: Option<Value>,
        cooling_profiles: Option<Value>,
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
            fans_locked: onboarding.is_some_and(|o| o["safety_acknowledged"] != true),
            fans_running: channels.iter().filter(|c| c["state"] == "running").count(),
            fan_faults: channels.iter().filter(|c| !c["fault"].is_null()).count(),
            cooling_profiles: names(&cooling["profiles"]),
            active_cooling_profile: cooling["active"].as_str().map(str::to_owned),
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
    fn disconnected_service_shows_a_clear_state() {
        let state = TrayState::from_replies(None, None, None, None, None, None, None);
        assert!(!state.connected && state.dimmed());
        assert_eq!(state.status_line(), "freeRGB service is not running");
        assert_eq!(state.fan_line(), "Fan control unavailable");
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
