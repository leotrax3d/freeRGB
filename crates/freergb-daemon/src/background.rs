//! Background (boot-time) fan control configuration. The daemon only validates
//! and hands the saved per-channel curves to the protected service through an
//! elevated, consented helper (UAC); the service runs them without a GUI or
//! daemon and publishes a status file. See `COOLING_BROAD_SUPPORT_PLAN.md` WP4.
use crate::cooling_sessions::{SessionPolicy, StartRequest};
use freergb_control::FanCurve;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const MAX_CHANNELS: usize = 8;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackgroundChannel {
    pub control_id: String,
    pub temperature_id: String,
    pub rpm_id: String,
    pub curve: FanCurve,
    /// The user physically confirmed a normal fan and its tachometer.
    pub confirmed_fan: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackgroundRequest {
    pub channels: Vec<BackgroundChannel>,
}

/// Tested duty/RPM pairs from the guided channel test (plausibility bands).
pub type Plausibility = Vec<(u8, f32)>;

/// One protected configuration entry; re-validated by the service.
pub fn protected_entry(
    channel: &BackgroundChannel,
    policy: &SessionPolicy,
    plausibility: Option<&Plausibility>,
) -> Result<Value, String> {
    let plan = StartRequest {
        mode: "curve".into(),
        control_id: channel.control_id.clone(),
        temperature_id: channel.temperature_id.clone(),
        rpm_id: channel.rpm_id.clone(),
        confirmed_fan: channel.confirmed_fan,
        curve: Some(channel.curve.clone()),
        critical_c: None,
        duty_percent: None,
    }
    .plan(policy)?;
    let mut entry = json!({
        "control_id": channel.control_id,
        "temperature_id": channel.temperature_id,
        "rpm_id": channel.rpm_id,
        "role": match plan.role {
            freergb_control::channels::ChannelRole::Fan => "fan",
            freergb_control::channels::ChannelRole::Pump => "pump",
        },
        "tier": plan.tier.as_str(),
        "floor_percent": plan.minimum_percent,
        "critical_c": plan.critical_c,
        "curve": channel.curve,
    });
    if plan.role == freergb_control::channels::ChannelRole::Pump && plan.minimum_percent < 100 {
        entry["pump_unlocked"] = json!(true);
    }
    if let Some(bands) = plausibility {
        entry["plausibility"] = bands
            .iter()
            .map(|(duty, rpm)| json!({"duty_percent": duty, "rpm": rpm}))
            .collect();
    }
    Ok(entry)
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplyState {
    pub state: &'static str,
    pub error: Option<String>,
}

pub struct Background {
    helper: Option<PathBuf>,
    path: Option<PathBuf>,
    status_path: Option<PathBuf>,
    saved: BackgroundRequest,
    apply: Arc<Mutex<ApplyState>>,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            helper: None,
            path: None,
            status_path: None,
            saved: BackgroundRequest::default(),
            apply: Arc::new(Mutex::new(ApplyState {
                state: "idle",
                error: None,
            })),
        }
    }
}

fn bounded_read(path: &std::path::Path, limit: u64) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

impl Background {
    /// `helper` is the bundled `freergb-mainboard.exe`; the status file lives
    /// next to the protected broker under `%ProgramFiles%\freeRGB`.
    pub fn enable(mut self, helper: PathBuf) -> Self {
        self.status_path = std::env::var_os("ProgramFiles").map(|root| {
            PathBuf::from(root)
                .join("freeRGB")
                .join("mainboard-control-status.json")
        });
        self.helper = Some(helper);
        self
    }
    pub fn with_file(mut self, path: PathBuf) -> Self {
        if let Some(bytes) = bounded_read(&path, 256 * 1024) {
            if let Ok(saved) = serde_json::from_slice::<BackgroundRequest>(&bytes) {
                self.saved = saved;
            }
        }
        self.path = Some(path);
        self
    }
    pub fn helper(&self) -> Option<PathBuf> {
        self.helper.clone()
    }

    pub fn view(&self) -> Value {
        json!({
            "available": self.helper.is_some(),
            "saved": self.saved,
            "apply": *self.apply.lock().unwrap_or_else(|e| e.into_inner()),
            "service": self.service_status(),
        })
    }
    /// Status published by the protected service; never trusted for control.
    pub fn service_status(&self) -> Value {
        self.status_path
            .as_ref()
            .and_then(|path| bounded_read(path, 64 * 1024))
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .filter(|value| value["version"] == 1 && value["channels"].is_array())
            .unwrap_or(Value::Null)
    }
    /// Validate every channel (caller resolves policies), persist the request
    /// locally and start the elevated hand-over in the background.
    pub fn apply(&mut self, request: BackgroundRequest, entries: Vec<Value>) -> Result<(), String> {
        let helper = self
            .helper
            .clone()
            .ok_or("Background control requires the optional mainboard provider on Windows")?;
        if request.channels.len() > MAX_CHANNELS || entries.len() != request.channels.len() {
            return Err(format!("At most {MAX_CHANNELS} background channels"));
        }
        {
            let mut state = self.apply.lock().unwrap_or_else(|e| e.into_inner());
            if state.state == "applying" {
                return Err("A background configuration is already being applied".into());
            }
            state.state = "applying";
            state.error = None;
        }
        let document = json!({"version": 1, "channels": entries});
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec_pretty(&request).map_err(|e| e.to_string())?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        }
        self.saved = request;
        let encoded = base64_encode(document.to_string().as_bytes());
        let state = self.apply.clone();
        std::thread::Builder::new()
            .name("cooling-background-apply".into())
            .spawn(move || {
                let result = run_helper(&helper, &encoded);
                let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => {
                        state.state = "applied";
                        state.error = None;
                    }
                    Err(error) => {
                        state.state = "failed";
                        state.error = Some(error);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Pause/resume all background fan channels in the protected service (no UAC:
/// pausing only returns channels to firmware, resuming restarts the already
/// authorized configuration). Returns the service's paused state.
pub fn set_paused(helper: &std::path::Path, paused: bool) -> Result<bool, String> {
    let mut command = std::process::Command::new(helper);
    command
        .args(["--background", if paused { "pause" } else { "resume" }])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command
        .output()
        .map_err(|error| format!("Fan service unreachable: {error}"))?;
    if !output.status.success() {
        return Err("The protected fan service did not accept the request".into());
    }
    let reply: Value = serde_json::from_slice(output.stdout.trim_ascii())
        .map_err(|_| "Invalid reply from the protected fan service")?;
    reply["background_paused"]
        .as_bool()
        .ok_or_else(|| "Invalid reply from the protected fan service".into())
}

fn run_helper(helper: &std::path::Path, encoded: &str) -> Result<(), String> {
    let mut command = std::process::Command::new(helper);
    command
        .args(["--request-background-config", encoded])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Background configuration failed: {error}"))?;
    let started = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err("Background configuration was refused or cancelled".into())
            };
        }
        if started.elapsed() > std::time::Duration::from_secs(125) {
            // Do not kill: the elevated child may still publish its result.
            return Err("UAC confirmation timed out; check the background status".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_control::channels::ChannelRole;
    use freergb_control::fan_support::{ChannelEvidence, SupportTier};

    fn channel(minimum: f32) -> BackgroundChannel {
        let mut curve = FanCurve {
            minimum_percent: minimum,
            ..FanCurve::default()
        };
        for point in &mut curve.points {
            point.duty_percent = point.duty_percent.max(minimum);
        }
        BackgroundChannel {
            control_id: "/lpc/it8689e/0/control/1".into(),
            temperature_id: "/lpc/it8689e/0/temperature/0".into(),
            rpm_id: "/lpc/it8689e/0/fan/1".into(),
            curve,
            confirmed_fan: true,
        }
    }
    fn policy(capability: SupportTier, test: bool, unlocked: bool) -> SessionPolicy {
        SessionPolicy {
            capability,
            evidence: ChannelEvidence {
                role: Some(ChannelRole::Fan),
                test_passed: test,
                unlocked,
                pump_confirmed: false,
            },
        }
    }

    #[test]
    fn protected_entries_carry_tier_floor_and_test_evidence() {
        let verified = protected_entry(&channel(40.0), &SessionPolicy::legacy(), None).unwrap();
        assert_eq!(
            (
                &verified["tier"],
                &verified["floor_percent"],
                &verified["critical_c"]
            ),
            (&json!("verified"), &json!(40), &json!(85))
        );
        assert!(protected_entry(
            &channel(40.0),
            &policy(SupportTier::LhmSupported, false, false),
            None
        )
        .is_err());
        let bands = vec![(50, 800.0), (100, 1500.0)];
        let tested = protected_entry(
            &channel(40.0),
            &policy(SupportTier::LhmSupported, true, false),
            Some(&bands),
        )
        .unwrap();
        assert_eq!(
            tested["plausibility"][1],
            json!({"duty_percent":100,"rpm":1500.0})
        );
        assert!(protected_entry(
            &channel(0.0),
            &policy(SupportTier::Verified, false, false),
            None
        )
        .is_err());
        let unlocked = protected_entry(
            &channel(0.0),
            &policy(SupportTier::LhmSupported, false, true),
            None,
        )
        .unwrap();
        assert_eq!(
            (&unlocked["tier"], &unlocked["floor_percent"]),
            (&json!("unrestricted"), &json!(0))
        );
        let mut unconfirmed = channel(40.0);
        unconfirmed.confirmed_fan = false;
        assert!(protected_entry(&unconfirmed, &SessionPolicy::legacy(), None).is_err());
    }

    #[test]
    fn base64_matches_the_standard_alphabet() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn apply_without_provider_is_refused() {
        let mut background = Background::default();
        assert!(background
            .apply(BackgroundRequest::default(), Vec::new())
            .is_err());
        assert_eq!(background.view()["available"], false);
    }
}
