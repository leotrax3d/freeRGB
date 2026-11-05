//! Cooling profiles: named, switchable complete cooling configurations
//! (FanControl-style "configurations"): background (boot) channel curves, live
//! channel assignments with their controllers and limits, the composite curve
//! set and the custom sensor graph. Activating a profile re-validates every
//! channel under the current support policy; nothing in a profile is trusted
//! by the protected service. Electrical channel bindings (fan/pump role) are
//! hardware facts and never part of a profile. Persisted in the per-user
//! config `cooling-profiles.json`; exchanged as a versioned JSON document with
//! identity remapping (docs/COOLING_SERVICE.md).
use crate::background::{BackgroundChannel, BackgroundRequest, MAX_CHANNELS};
use crate::cooling_sessions::StartRequest;
use freergb_control::{curves::CurveSet, sensors::SensorGraph};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::PathBuf;

const MAX_PROFILES: usize = 32;
const MAX_BYTES: u64 = 1024 * 1024;
pub const SCHEMA: &str = "freergb.cooling-profile";
pub const SCHEMA_VERSION: u32 = 2;

/// One profile. A version-1 entry (`{"channels":[...]}`) reads unchanged.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoolingProfile {
    /// Background (boot-time) channel curves.
    #[serde(default)]
    pub channels: Vec<BackgroundChannel>,
    /// Live channel assignments; `None` leaves running channels untouched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignments: Option<Vec<StartRequest>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve_set: Option<CurveSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensors: Option<SensorGraph>,
}

impl From<BackgroundRequest> for CoolingProfile {
    fn from(request: BackgroundRequest) -> Self {
        Self {
            channels: request.channels,
            ..Self::default()
        }
    }
}

impl CoolingProfile {
    pub fn background(&self) -> BackgroundRequest {
        BackgroundRequest {
            channels: self.channels.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.channels.len() > MAX_CHANNELS {
            return Err(format!("At most {MAX_CHANNELS} channels per profile"));
        }
        let mut seen = BTreeSet::new();
        for channel in &self.channels {
            channel.curve.validate()?;
            if !seen.insert(&channel.control_id) {
                return Err("A channel appears twice in the profile".into());
            }
        }
        if let Some(assignments) = &self.assignments {
            if assignments.len() > crate::cooling_assignments::MAX_ASSIGNMENTS {
                return Err("Too many live channels in the profile".into());
            }
            let mut seen = BTreeSet::new();
            for request in assignments {
                if !crate::cooling_assignments::persistable(request) {
                    return Err(format!(
                        "Mode {:?} cannot be part of a profile",
                        request.mode
                    ));
                }
                if let Some(curve) = &request.curve {
                    curve.validate()?;
                }
                if !seen.insert(&request.control_id) {
                    return Err("A live channel appears twice in the profile".into());
                }
            }
        }
        if let Some(set) = &self.curve_set {
            set.validate()?;
        }
        if let Some(graph) = &self.sensors {
            graph.validate()?;
        }
        Ok(())
    }

    /// Every native control the profile drives.
    pub fn controls(&self) -> BTreeSet<String> {
        let mut ids: BTreeSet<_> = self.channels.iter().map(|c| c.control_id.clone()).collect();
        ids.extend(
            self.assignments
                .iter()
                .flatten()
                .map(|a| a.control_id.clone()),
        );
        ids
    }

    fn native_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        for c in &self.channels {
            ids.extend([
                c.control_id.clone(),
                c.temperature_id.clone(),
                c.rpm_id.clone(),
            ]);
        }
        for a in self.assignments.iter().flatten() {
            ids.extend([a.control_id.clone(), a.rpm_id.clone()]);
            if let Ok(sources) = crate::cooling_sessions::temperature_sources(&a.temperature_id) {
                ids.extend(sources.into_iter().map(str::to_owned));
            }
        }
        ids.retain(|id| id.starts_with('/'));
        ids
    }

    /// A physical fan confirmation never transfers to other hardware.
    fn clear_confirmations(&mut self) {
        for c in &mut self.channels {
            c.confirmed_fan = false;
        }
        for a in self.assignments.iter_mut().flatten() {
            a.confirmed_fan = false;
        }
    }
}

/// Device part of a native id: `/lpc/nct6701d/0/control/2` -> `/lpc/nct6701d/0`.
pub fn device_prefix(id: &str) -> Option<String> {
    let parts: Vec<_> = id.split('/').collect();
    (id.starts_with('/') && parts.len() >= 4).then(|| parts[..parts.len() - 2].join("/"))
}

fn remap_value(value: &mut Value, map: &BTreeMap<String, String>, count: &mut usize) {
    match value {
        Value::String(text) => {
            if let Some((from, to)) = map.iter().find(|(from, _)| {
                text.strip_prefix(from.as_str())
                    .is_some_and(|r| r.starts_with('/'))
            }) {
                *text = format!("{to}{}", &text[from.len()..]);
                *count += 1;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| remap_value(v, map, count)),
        Value::Object(fields) => fields.values_mut().for_each(|v| remap_value(v, map, count)),
        _ => {}
    }
}

/// Where a profile was exported (hardware identity, no readings).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// `manufacturer product` from SMBIOS, if known.
    pub board: Option<String>,
    /// Device prefixes of the native ids used by the profile.
    pub devices: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema: String,
    pub version: u32,
    pub name: String,
    pub source: Source,
    pub profile: CoolingProfile,
}

/// The local system an import is checked against.
pub struct Target<'a> {
    pub board: Option<String>,
    /// Device prefixes of the current inventory.
    pub devices: &'a [String],
}

#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub name: String,
    pub remapped: usize,
    pub same_board: bool,
    /// Native ids whose device is not present here (activation will refuse them).
    pub unresolved: Vec<String>,
    /// Fan confirmations were cleared; confirm channels again before use.
    pub needs_confirmation: bool,
}

/// Parses, remaps and validates an exported document. `remap` maps source
/// device prefixes to local ones; with exactly one device on both sides and no
/// explicit entry, that pair is mapped automatically.
pub fn import(
    text: &str,
    remap: &BTreeMap<String, String>,
    target: &Target,
) -> Result<(String, CoolingProfile, ImportReport), String> {
    if text.len() as u64 > MAX_BYTES {
        return Err("Profile document too large".into());
    }
    let document: Document =
        serde_json::from_str(text).map_err(|e| format!("Not a cooling profile: {e}"))?;
    if document.schema != SCHEMA {
        return Err("Not a freeRGB cooling profile".into());
    }
    if document.version == 0 || document.version > SCHEMA_VERSION {
        return Err(format!(
            "Profile schema version {} is not supported (max {SCHEMA_VERSION})",
            document.version
        ));
    }
    if !valid_name(&document.name) {
        return Err("Invalid profile name in document".into());
    }
    let mut map = remap.clone();
    for (from, to) in &map {
        if device_prefix(&format!("{from}/x/0")).as_deref() != Some(from.as_str())
            || device_prefix(&format!("{to}/x/0")).as_deref() != Some(to.as_str())
        {
            return Err(format!("Invalid remap entry {from:?} -> {to:?}"));
        }
    }
    if map.is_empty() {
        if let ([from], [to]) = (document.source.devices.as_slice(), target.devices) {
            if from != to {
                map.insert(from.clone(), to.clone());
            }
        }
    }
    let mut value = serde_json::to_value(&document.profile).map_err(|e| e.to_string())?;
    let mut remapped = 0;
    remap_value(&mut value, &map, &mut remapped);
    let mut profile: CoolingProfile = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let same_board = document.source.board.is_some() && document.source.board == target.board;
    let needs_confirmation = !same_board || remapped > 0;
    if needs_confirmation {
        profile.clear_confirmations();
    }
    profile.validate()?;
    let unresolved = profile
        .native_ids()
        .into_iter()
        .filter(|id| device_prefix(id).is_none_or(|prefix| !target.devices.contains(&prefix)))
        .collect();
    let report = ImportReport {
        name: document.name.clone(),
        remapped,
        same_board,
        unresolved,
        needs_confirmation,
    };
    Ok((document.name, profile, report))
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilesConfig {
    pub version: u32,
    pub profiles: BTreeMap<String, CoolingProfile>,
    /// Last activated profile.
    pub active: Option<String>,
}

pub fn valid_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && trimmed == name
        && name.len() <= 64
        && !name.chars().any(char::is_control)
}

#[derive(Default)]
pub struct CoolingProfiles {
    config: ProfilesConfig,
    path: Option<PathBuf>,
    error: Option<String>,
}

impl CoolingProfiles {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let mut bytes = Vec::new();
        let read =
            std::fs::File::open(&path).and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut bytes));
        match read {
            Ok(_) if bytes.len() as u64 <= MAX_BYTES => {
                match serde_json::from_slice::<ProfilesConfig>(&bytes) {
                    Ok(config) if matches!(config.version, 1 | 2) => self.config = config,
                    _ => self.error = Some("Invalid cooling-profiles.json ignored".into()),
                }
            }
            Ok(_) => self.error = Some("Oversized cooling-profiles.json ignored".into()),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => self.error = Some(e.to_string()),
            _ => {}
        }
        self.config.version = SCHEMA_VERSION;
        self.path = Some(path);
        self
    }

    fn persist(&mut self) -> Result<(), String> {
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

    pub fn view(&self) -> Value {
        let profiles: Vec<_> = self
            .config
            .profiles
            .iter()
            .map(|(name, p)| {
                json!({
                    "name": name,
                    "channels": p.channels.len(),
                    "live_channels": p.assignments.as_ref().map(Vec::len),
                    "curve_set": p.curve_set.is_some(),
                    "sensors": p.sensors.is_some(),
                })
            })
            .collect();
        json!({"profiles": profiles, "active": self.config.active, "error": self.error})
    }

    pub fn names(&self) -> Vec<String> {
        self.config.profiles.keys().cloned().collect()
    }

    pub fn active(&self) -> Option<&str> {
        self.config.active.as_deref()
    }

    pub fn get(&self, name: &str) -> Option<&CoolingProfile> {
        self.config.profiles.get(name)
    }

    pub fn save(&mut self, name: &str, profile: CoolingProfile) -> Result<(), String> {
        if !valid_name(name) {
            return Err(
                "Profile names need 1..64 characters without leading/trailing spaces".into(),
            );
        }
        profile.validate()?;
        if !self.config.profiles.contains_key(name) && self.config.profiles.len() >= MAX_PROFILES {
            return Err(format!("At most {MAX_PROFILES} cooling profiles"));
        }
        self.config.profiles.insert(name.to_owned(), profile);
        self.persist()
    }

    pub fn delete(&mut self, name: &str) -> Result<bool, String> {
        let removed = self.config.profiles.remove(name).is_some();
        if removed {
            if self.config.active.as_deref() == Some(name) {
                self.config.active = None;
            }
            self.persist()?;
        }
        Ok(removed)
    }

    pub fn mark_active(&mut self, name: &str) -> Result<(), String> {
        self.config.active = Some(name.to_owned());
        self.persist()
    }

    /// Versioned exchange document with the source hardware identity.
    pub fn export(&self, name: &str, board: Option<String>) -> Result<Document, String> {
        let profile = self.get(name).ok_or("Unknown cooling profile")?.clone();
        let devices: BTreeSet<_> = profile
            .native_ids()
            .iter()
            .filter_map(|id| device_prefix(id))
            .collect();
        Ok(Document {
            schema: SCHEMA.into(),
            version: SCHEMA_VERSION,
            name: name.to_owned(),
            source: Source {
                board,
                devices: devices.into_iter().collect(),
            },
            profile,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_control::FanCurve;

    fn curve() -> FanCurve {
        let mut curve = FanCurve {
            minimum_percent: 40.0,
            ..FanCurve::default()
        };
        for point in &mut curve.points {
            point.duty_percent = point.duty_percent.max(40.0);
        }
        curve
    }

    pub(crate) fn channel(index: u8) -> BackgroundChannel {
        BackgroundChannel {
            control_id: format!("/lpc/it8689e/0/control/{index}"),
            temperature_id: "/lpc/it8689e/0/temperature/0".into(),
            rpm_id: format!("/lpc/it8689e/0/fan/{index}"),
            curve: curve(),
            confirmed_fan: true,
        }
    }

    pub(crate) fn live(index: u8, mode: &str) -> StartRequest {
        serde_json::from_value(json!({
            "mode": mode,
            "control_id": format!("/lpc/it8689e/0/control/{index}"),
            "temperature_id": "/lpc/it8689e/0/temperature/0,/amdcpu/0/temperature/2",
            "rpm_id": format!("/lpc/it8689e/0/fan/{index}"),
            "confirmed_fan": true,
            "curve": if mode == "curve" { serde_json::to_value(curve()).unwrap() } else { Value::Null },
            "critical_c": null,
            "duty_percent": if mode == "manual" { json!(55) } else { Value::Null },
            "composite_output": if mode == "composite" { json!("case") } else { Value::Null },
        }))
        .unwrap()
    }

    fn temp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("freergb-{name}-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn profiles_validate_persist_and_track_the_active_one() {
        let path = temp("profiles");
        let mut store = CoolingProfiles::default().with_file(path.clone());
        let quiet = CoolingProfile {
            channels: vec![channel(1), channel(2)],
            assignments: Some(vec![live(3, "curve"), live(4, "manual")]),
            ..CoolingProfile::default()
        };
        store.save("Quiet", quiet).unwrap();
        store.save("Off", CoolingProfile::default()).unwrap();
        for bad in ["", " Quiet", "Quiet ", "a\nb", &"x".repeat(65)] {
            assert!(
                store.save(bad, CoolingProfile::default()).is_err(),
                "{bad:?}"
            );
        }
        let twice = CoolingProfile {
            channels: vec![channel(1), channel(1)],
            ..CoolingProfile::default()
        };
        assert!(store.save("Twice", twice).is_err());
        let live_twice = CoolingProfile {
            assignments: Some(vec![live(3, "curve"), live(3, "manual")]),
            ..CoolingProfile::default()
        };
        assert!(store.save("LiveTwice", live_twice).is_err());
        let commissioning = CoolingProfile {
            assignments: Some(vec![live(3, "calibration")]),
            ..CoolingProfile::default()
        };
        assert!(store.save("Calibrate", commissioning).is_err());
        let mut invalid = channel(3);
        invalid.curve.points.clear();
        assert!(store
            .save(
                "Broken",
                CoolingProfile {
                    channels: vec![invalid],
                    ..CoolingProfile::default()
                }
            )
            .is_err());
        store.mark_active("Quiet").unwrap();
        let reloaded = CoolingProfiles::default().with_file(path.clone());
        let quiet = reloaded.get("Quiet").unwrap();
        assert_eq!(quiet.channels.len(), 2);
        assert_eq!(quiet.assignments.as_ref().unwrap().len(), 2);
        assert_eq!(reloaded.view()["active"], "Quiet");
        assert_eq!(reloaded.view()["profiles"][1]["live_channels"], 2);
        let mut reloaded = reloaded;
        assert!(reloaded.delete("Quiet").unwrap());
        assert!(reloaded.view()["active"].is_null());
        assert!(!reloaded.delete("Quiet").unwrap());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn version_one_files_still_load() {
        let path = temp("profiles-v1");
        let v1 = json!({"version":1,"profiles":{"Old":{"channels":[serde_json::to_value(channel(1)).unwrap()]}},"active":"Old"});
        std::fs::write(&path, v1.to_string()).unwrap();
        let store = CoolingProfiles::default().with_file(path.clone());
        let old = store.get("Old").unwrap();
        assert_eq!(old.channels.len(), 1);
        assert!(old.assignments.is_none(), "v1 leaves live channels alone");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn export_import_round_trip_on_the_same_board_keeps_confirmations() {
        let mut store = CoolingProfiles::default();
        store
            .save(
                "Quiet",
                CoolingProfile {
                    channels: vec![channel(1)],
                    assignments: Some(vec![live(2, "curve")]),
                    ..CoolingProfile::default()
                },
            )
            .unwrap();
        let board = Some("ASUS PRIME".to_string());
        let document = store.export("Quiet", board.clone()).unwrap();
        assert_eq!(document.schema, SCHEMA);
        assert_eq!(document.source.devices, ["/amdcpu/0", "/lpc/it8689e/0"]);
        let text = serde_json::to_string(&document).unwrap();
        let devices = ["/amdcpu/0".to_string(), "/lpc/it8689e/0".to_string()];
        let (name, profile, report) = import(
            &text,
            &BTreeMap::new(),
            &Target {
                board,
                devices: &devices,
            },
        )
        .unwrap();
        assert_eq!(name, "Quiet");
        assert!(report.same_board && !report.needs_confirmation);
        assert!(report.unresolved.is_empty());
        assert!(profile.channels[0].confirmed_fan);
    }

    #[test]
    fn import_on_another_board_remaps_identities_and_clears_confirmations() {
        let mut store = CoolingProfiles::default();
        store
            .save(
                "Quiet",
                CoolingProfile {
                    channels: vec![channel(1)],
                    ..CoolingProfile::default()
                },
            )
            .unwrap();
        let text = serde_json::to_string(&store.export("Quiet", Some("Gigabyte".into())).unwrap())
            .unwrap();
        let devices = ["/lpc/nct6701d/0".to_string()];
        let target = Target {
            board: Some("ASUS".into()),
            devices: &devices,
        };
        // One device on each side: mapped automatically.
        let (_, profile, report) = import(&text, &BTreeMap::new(), &target).unwrap();
        assert_eq!(profile.channels[0].control_id, "/lpc/nct6701d/0/control/1");
        assert_eq!(profile.channels[0].rpm_id, "/lpc/nct6701d/0/fan/1");
        assert_eq!(report.remapped, 3);
        assert!(report.needs_confirmation && !profile.channels[0].confirmed_fan);
        assert!(report.unresolved.is_empty());
        // Explicit mapping to a device that is missing here stays unresolved.
        let explicit = BTreeMap::from([("/lpc/it8689e/0".into(), "/lpc/it8689e/1".into())]);
        let (_, _, report) = import(&text, &explicit, &target).unwrap();
        assert_eq!(report.unresolved.len(), 3);
        // Bad documents.
        assert!(import("{}", &BTreeMap::new(), &target).is_err());
        let future = text.replace("\"version\":2", "\"version\":9");
        assert!(import(&future, &BTreeMap::new(), &target).is_err());
        let bad_map = BTreeMap::from([("lpc".into(), "/x".into())]);
        assert!(import(&text, &bad_map, &target).is_err());
    }

    #[test]
    fn device_prefix_drops_kind_and_index() {
        assert_eq!(
            device_prefix("/lpc/nct6701d/0/control/2").as_deref(),
            Some("/lpc/nct6701d/0")
        );
        assert_eq!(
            device_prefix("/gpu-nvidia/0/temperature/0").as_deref(),
            Some("/gpu-nvidia/0")
        );
        assert_eq!(device_prefix("custom-node"), None);
    }
}
