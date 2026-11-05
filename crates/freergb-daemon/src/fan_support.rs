//! Daemon-owned evidence per fan channel: guided channel-test results and
//! (work package 6) unrestricted unlocks. Every record is bound to the board,
//! firmware (BIOS) and channel identity and is ignored as soon as any of them
//! changes. Persisted in the per-user config `cooling-fan-support.json`.
use crate::mainboard::{BoardIdentity, Channel};
use freergb_control::calibration::{CalibrationResult, TestedDuty};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

const MAX_BYTES: u64 = 256 * 1024;
const MAX_RECORDS: usize = 64;

/// Board + firmware + chip + channel. A new BIOS or another board invalidates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelIdentity {
    pub manufacturer: String,
    pub product: String,
    pub bios_version: String,
    pub hardware_id: String,
    pub control_id: String,
}

impl ChannelIdentity {
    pub fn of(board: Option<&BoardIdentity>, channel: &Channel) -> Option<Self> {
        let board = board?;
        let hardware_id = serde_json::from_str::<(String, String)>(&channel.id)
            .ok()?
            .0;
        Some(Self {
            manufacturer: board.manufacturer.clone(),
            product: board.product.clone(),
            bios_version: board.bios_version.clone(),
            hardware_id,
            control_id: channel.sensor_id.clone(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestRecord {
    pub identity: ChannelIdentity,
    pub passed: bool,
    pub tested: Vec<TestedDuty>,
    pub recorded_unix: u64,
    /// Stall and start duty from a stall sweep (unrestricted channels only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stall_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_percent: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnlockRecord {
    pub identity: ChannelIdentity,
    /// Channel name the user typed as confirmation.
    pub confirmed_name: String,
    /// Second, separate confirmation for a channel bound as pump.
    pub pump_confirmed: bool,
    pub unlocked_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanSupportConfig {
    pub version: u32,
    /// Global "unrestricted control" switch (default off).
    pub unrestricted_enabled: bool,
    pub tests: BTreeMap<String, TestRecord>,
    pub unlocks: BTreeMap<String, UnlockRecord>,
    /// Latest completed calibration per channel (duty/RPM pairs for RPM targets).
    #[serde(default)]
    pub calibrations: BTreeMap<String, TestRecord>,
}

impl Default for FanSupportConfig {
    fn default() -> Self {
        Self {
            version: 1,
            unrestricted_enabled: false,
            tests: BTreeMap::new(),
            unlocks: BTreeMap::new(),
            calibrations: BTreeMap::new(),
        }
    }
}

/// A test passes with a plausible, monotonic RPM response at 100/70/50%.
pub fn passed(result: &CalibrationResult) -> bool {
    result.channel_test_passed()
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[derive(Default)]
pub struct FanSupport {
    pub config: FanSupportConfig,
    path: Option<PathBuf>,
    error: Option<String>,
}

impl FanSupport {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let mut bytes = Vec::new();
        let read =
            std::fs::File::open(&path).and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut bytes));
        match read {
            Ok(_) if bytes.len() as u64 <= MAX_BYTES => {
                match serde_json::from_slice::<FanSupportConfig>(&bytes) {
                    Ok(config) if config.version == 1 => self.config = config,
                    _ => self.error = Some("Invalid cooling-fan-support.json ignored".into()),
                }
            }
            Ok(_) => self.error = Some("Oversized cooling-fan-support.json ignored".into()),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => self.error = Some(e.to_string()),
            _ => {}
        }
        self.path = Some(path);
        self
    }

    pub fn save(&mut self) -> Result<(), String> {
        if self.config.tests.len() > MAX_RECORDS
            || self.config.unlocks.len() > MAX_RECORDS
            || self.config.calibrations.len() > MAX_RECORDS
        {
            return Err("Too many fan support records".into());
        }
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec_pretty(&self.config).map_err(|e| e.to_string())?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        }
        self.error = None;
        Ok(())
    }

    pub fn record_test(
        &mut self,
        identity: ChannelIdentity,
        result: &CalibrationResult,
    ) -> Result<(), String> {
        self.config.tests.insert(
            identity.control_id.clone(),
            TestRecord {
                passed: passed(result),
                tested: result.tested.clone(),
                identity,
                recorded_unix: now_unix(),
                stall_percent: result.stall_percent,
                start_percent: result.start_percent,
            },
        );
        self.save()
    }

    /// Stores a completed calibration with an observed, monotonic response.
    pub fn record_calibration(
        &mut self,
        identity: ChannelIdentity,
        result: &CalibrationResult,
    ) -> Result<(), String> {
        if !(result.response_observed && result.monotonic_response) {
            return Ok(());
        }
        self.config.calibrations.insert(
            identity.control_id.clone(),
            TestRecord {
                passed: true,
                tested: result.tested.clone(),
                identity,
                recorded_unix: now_unix(),
                stall_percent: result.stall_percent,
                start_percent: result.start_percent,
            },
        );
        self.save()
    }

    /// Stall/start duties of the current identity's latest calibration.
    pub fn thresholds(&self, identity: &ChannelIdentity) -> (Option<u8>, Option<u8>) {
        self.config
            .calibrations
            .get(&identity.control_id)
            .filter(|c| &c.identity == identity)
            .map_or((None, None), |c| (c.stall_percent, c.start_percent))
    }

    /// Duty/RPM map from the current identity's calibration, else its passed
    /// channel test. Records are not mixed: one measurement run per map.
    pub fn rpm_map(
        &self,
        identity: &ChannelIdentity,
    ) -> Result<freergb_control::rpm::RpmMap, String> {
        let record = self
            .config
            .calibrations
            .get(&identity.control_id)
            .filter(|c| &c.identity == identity)
            .or_else(|| self.passed_test(identity))
            .ok_or("Run the channel test or calibration on this fan first (a new BIOS or board needs a new run)")?;
        freergb_control::rpm::RpmMap::from_tested(&record.tested)
    }

    /// Passed test for exactly this identity (stale records are ignored).
    pub fn passed_test(&self, identity: &ChannelIdentity) -> Option<&TestRecord> {
        self.config
            .tests
            .get(&identity.control_id)
            .filter(|t| t.passed && &t.identity == identity)
    }

    /// Valid unlock for exactly this identity while the global switch is on.
    pub fn unlock(&self, identity: &ChannelIdentity) -> Option<&UnlockRecord> {
        if !self.config.unrestricted_enabled {
            return None;
        }
        self.config
            .unlocks
            .get(&identity.control_id)
            .filter(|u| &u.identity == identity)
    }

    pub fn set_unrestricted(&mut self, enabled: bool) -> Result<(), String> {
        self.config.unrestricted_enabled = enabled;
        self.save()
    }

    /// Per-channel unlock: the global switch must be on and the user must type
    /// the channel's name exactly. Takeover of foreign software mode stays refused
    /// by the caller (inventory) and the protected worker.
    pub fn unlock_channel(
        &mut self,
        identity: ChannelIdentity,
        channel_name: &str,
        typed: &str,
    ) -> Result<(), String> {
        if !self.config.unrestricted_enabled {
            return Err("Enable unrestricted control in the settings first".into());
        }
        if typed.trim() != channel_name.trim() || channel_name.trim().is_empty() {
            return Err("Type the channel name exactly to confirm the unlock".into());
        }
        self.config.unlocks.insert(
            identity.control_id.clone(),
            UnlockRecord {
                identity,
                confirmed_name: channel_name.trim().into(),
                pump_confirmed: false,
                unlocked_unix: now_unix(),
            },
        );
        self.save()
    }

    /// Second, separate confirmation for a pump; requires an existing unlock.
    pub fn confirm_pump(&mut self, identity: &ChannelIdentity, typed: &str) -> Result<(), String> {
        if !self.config.unrestricted_enabled {
            return Err("Enable unrestricted control in the settings first".into());
        }
        let record = self
            .config
            .unlocks
            .get_mut(&identity.control_id)
            .filter(|u| &u.identity == identity)
            .ok_or("Unlock the channel before confirming pump control")?;
        if typed.trim() != format!("PUMP {}", record.confirmed_name) {
            return Err("Type PUMP followed by the channel name to confirm pump control".into());
        }
        record.pump_confirmed = true;
        self.save()
    }

    pub fn revoke(&mut self, control_id: &str) -> Result<bool, String> {
        let removed = self.config.unlocks.remove(control_id).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    fn state(&self, identity: Option<&ChannelIdentity>, control_id: &str) -> Value {
        let test = self.config.tests.get(control_id);
        let rpm_range = identity
            .and_then(|i| self.rpm_map(i).ok())
            .map(|m| json!({"min_rpm": m.min_rpm(), "max_rpm": m.max_rpm()}));
        let unlock = self.config.unlocks.get(control_id);
        let current = |id: &ChannelIdentity| identity.is_some_and(|i| i == id);
        json!({
            "test": match test {
                None => "none",
                Some(t) if !current(&t.identity) => "stale",
                Some(t) if t.passed => "passed",
                Some(_) => "failed",
            },
            "tested": test.filter(|t| current(&t.identity)).map(|t| &t.tested),
            "rpm_range": rpm_range,
            "stall_percent": identity.and_then(|i| self.thresholds(i).0),
            "start_percent": identity.and_then(|i| self.thresholds(i).1),
            "unlock": match unlock {
                None => "none",
                Some(u) if !current(&u.identity) => "stale",
                Some(_) if !self.config.unrestricted_enabled => "disabled",
                Some(u) if u.pump_confirmed => "unlocked_pump",
                Some(_) => "unlocked",
            },
        })
    }

    pub fn view(&self, board: Option<&BoardIdentity>, channels: &[Channel]) -> Value {
        let entries: BTreeMap<_, _> = channels
            .iter()
            .filter(|c| c.kind == "control" && c.control_available)
            .map(|c| {
                let identity = ChannelIdentity::of(board, c);
                let mut state = self.state(identity.as_ref(), &c.sensor_id);
                state["coverage"] = crate::fan_coverage::lookup(&c.sensor_id).into();
                (c.sensor_id.clone(), state)
            })
            .collect();
        json!({
            "unrestricted_enabled": self.config.unrestricted_enabled,
            "board_identity_available": board.is_some(),
            "channels": entries,
            "error": self.error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn identity(bios: &str) -> ChannelIdentity {
        ChannelIdentity {
            manufacturer: "Vendor".into(),
            product: "Board".into(),
            bios_version: bios.into(),
            hardware_id: "/lpc/it8689e/0".into(),
            control_id: "/lpc/it8689e/0/control/1".into(),
        }
    }
    fn result(rpm: [f32; 3]) -> CalibrationResult {
        CalibrationResult {
            response_observed: rpm[2] <= rpm[0] * 0.9,
            lowest_tested_percent: 50,
            suggested_floor_percent: None,
            hardware_verified: false,
            tested: [100, 70, 50]
                .into_iter()
                .zip(rpm)
                .map(|(duty_percent, rpm)| TestedDuty { duty_percent, rpm })
                .collect(),
            monotonic_response: rpm[1] <= rpm[0] * 0.95 && rpm[2] <= rpm[1] * 0.95,
            stall_percent: None,
            start_percent: None,
        }
    }

    #[test]
    fn tests_bind_to_board_firmware_and_channel_and_persist() {
        let path =
            std::env::temp_dir().join(format!("freergb-fan-support-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut store = FanSupport::default().with_file(path.clone());
        store
            .record_test(identity("1.0"), &result([1500.0, 1100.0, 800.0]))
            .unwrap();
        assert!(store.passed_test(&identity("1.0")).is_some());
        assert!(
            store.passed_test(&identity("2.0")).is_none(),
            "new BIOS must invalidate"
        );
        let reloaded = FanSupport::default().with_file(path.clone());
        assert!(reloaded.passed_test(&identity("1.0")).is_some());
        let mut failing = reloaded;
        failing
            .record_test(identity("1.0"), &result([1200.0, 1190.0, 1180.0]))
            .unwrap();
        assert!(failing.passed_test(&identity("1.0")).is_none());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rpm_map_prefers_current_calibration_over_channel_test() {
        let mut store = FanSupport::default();
        assert!(store.rpm_map(&identity("1.0")).is_err());
        store
            .record_test(identity("1.0"), &result([1500.0, 1100.0, 800.0]))
            .unwrap();
        let from_test = store.rpm_map(&identity("1.0")).unwrap();
        assert_eq!(from_test.min_rpm(), 800.0);
        let mut calibration = result([1500.0, 1100.0, 800.0]);
        calibration.tested.push(TestedDuty {
            duty_percent: 40,
            rpm: 600.0,
        });
        store
            .record_calibration(identity("1.0"), &calibration)
            .unwrap();
        assert_eq!(store.rpm_map(&identity("1.0")).unwrap().min_rpm(), 600.0);
        assert!(
            store.rpm_map(&identity("2.0")).is_err(),
            "new BIOS must invalidate"
        );
    }

    #[test]
    fn unlock_requires_switch_typed_name_and_separate_pump_confirmation() {
        let mut store = FanSupport::default();
        assert!(store
            .unlock_channel(identity("1.0"), "Fan Control #2", "Fan Control #2")
            .is_err());
        store.set_unrestricted(true).unwrap();
        assert!(store
            .unlock_channel(identity("1.0"), "Fan Control #2", "fan control #2")
            .is_err());
        assert!(store
            .confirm_pump(&identity("1.0"), "PUMP Fan Control #2")
            .is_err());
        store
            .unlock_channel(identity("1.0"), "Fan Control #2", " Fan Control #2 ")
            .unwrap();
        assert!(!store.unlock(&identity("1.0")).unwrap().pump_confirmed);
        assert!(store
            .confirm_pump(&identity("1.0"), "Fan Control #2")
            .is_err());
        store
            .confirm_pump(&identity("1.0"), "PUMP Fan Control #2")
            .unwrap();
        assert!(store.unlock(&identity("1.0")).unwrap().pump_confirmed);
        assert!(store
            .confirm_pump(&identity("2.0"), "PUMP Fan Control #2")
            .is_err());
        store.set_unrestricted(false).unwrap();
        assert!(
            store.unlock(&identity("1.0")).is_none(),
            "switch off disables unlocks"
        );
        assert!(store.revoke(&identity("1.0").control_id).unwrap());
        assert!(!store.revoke(&identity("1.0").control_id).unwrap());
    }

    #[test]
    fn unlocks_need_the_global_switch_and_matching_identity() {
        let mut store = FanSupport::default();
        store.config.unlocks.insert(
            identity("1.0").control_id,
            UnlockRecord {
                identity: identity("1.0"),
                confirmed_name: "Fan Control #2".into(),
                pump_confirmed: false,
                unlocked_unix: 0,
            },
        );
        assert!(store.unlock(&identity("1.0")).is_none());
        store.config.unrestricted_enabled = true;
        assert!(store.unlock(&identity("1.0")).is_some());
        assert!(store.unlock(&identity("1.1")).is_none());
    }
}
