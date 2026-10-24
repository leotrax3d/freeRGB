//! Remembered curve/manual assignments per native channel, resumed when the
//! daemon starts (docs/COOLING_RUNTIME.md "Multiple channels"). A channel is
//! resumed only when its hardware identity (chip, native control and the saved
//! `cooling-channel-bindings.json` entry) is unchanged; otherwise it is shown
//! as "needs attention" and nothing is started. Per-user file
//! `cooling-assignments.json`. Every resume passes the normal session policy.
use crate::cooling_sessions::StartRequest;
use crate::mainboard::Channel;
use freergb_control::channels::ChannelBinding;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

pub const MAX_ASSIGNMENTS: usize = crate::cooling_sessions::MAX_ACTIVE;
/// How long to wait for the first channel inventory before giving up on resume.
const INVENTORY_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// Hardware identity recorded when the assignment was made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    /// Chip / device identity (`Channel::device_id`).
    pub device_id: String,
    /// Native control (`Channel::sensor_id`, the session `control_id`).
    pub control_id: String,
    /// Saved binding for the channel, if any.
    pub binding: Option<ChannelBinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub request: StartRequest,
    pub identity: Identity,
}

#[derive(Default)]
pub struct Assignments {
    list: Vec<Assignment>,
    path: Option<PathBuf>,
    /// control_id -> reason for channels that were not resumed.
    attention: BTreeMap<String, String>,
    resume_pending: bool,
    since: Option<std::time::Instant>,
}

/// Only continuous user modes are remembered (never identify/commissioning).
pub fn persistable(request: &StartRequest) -> bool {
    matches!(
        request.mode.as_str(),
        "curve" | "manual" | "composite" | "rpm"
    )
}

/// Current identity of `control_id` from the inventory and saved bindings.
pub fn identity_of(
    control_id: &str,
    channels: &[Channel],
    binding: impl Fn(&str) -> Option<ChannelBinding>,
) -> Option<Identity> {
    let channel = channels
        .iter()
        .find(|c| c.kind == "control" && c.sensor_id == control_id)?;
    Some(Identity {
        device_id: channel.device_id.clone(),
        control_id: control_id.to_owned(),
        binding: binding(&channel.id),
    })
}

impl Assignments {
    pub fn with_file(mut self, path: PathBuf) -> Self {
        let mut bytes = Vec::new();
        if std::fs::File::open(&path)
            .and_then(|f| f.take(256 * 1024).read_to_end(&mut bytes))
            .is_ok()
        {
            match serde_json::from_slice::<Vec<Assignment>>(&bytes) {
                Ok(list) if list.len() <= MAX_ASSIGNMENTS => self.list = list,
                _ => {
                    tracing::warn!("invalid cooling-assignments.json ignored");
                }
            }
        }
        self.resume_pending = !self.list.is_empty();
        self.path = Some(path);
        self
    }
    fn persist(&self) {
        let Some(path) = &self.path else { return };
        let result = serde_json::to_vec_pretty(&self.list)
            .map_err(std::io::Error::other)
            .and_then(|bytes| {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let temporary = path.with_extension("json.tmp");
                std::fs::write(&temporary, bytes)?;
                std::fs::rename(&temporary, path)
            });
        if let Err(error) = result {
            tracing::warn!(%error, "could not save cooling assignments");
        }
    }
    /// Remembers (replaces) the assignment of one channel.
    pub fn remember(&mut self, request: StartRequest, identity: Identity) {
        self.list
            .retain(|a| a.request.control_id != request.control_id);
        self.attention.remove(&request.control_id);
        if self.list.len() < MAX_ASSIGNMENTS {
            self.list.push(Assignment { request, identity });
        }
        self.persist();
    }
    /// User stopped a channel: do not resume it.
    pub fn forget(&mut self, control_id: &str) {
        let before = self.list.len();
        self.list.retain(|a| a.request.control_id != control_id);
        let cleared = self.attention.remove(control_id).is_some();
        if before != self.list.len() || cleared {
            self.persist();
        }
    }
    pub fn forget_all(&mut self) {
        self.list.clear();
        self.attention.clear();
        self.resume_pending = false;
        self.persist();
    }
    pub fn control_ids(&self) -> Vec<String> {
        self.list
            .iter()
            .map(|a| a.request.control_id.clone())
            .collect()
    }
    /// Once the inventory is known: requests to resume now. Channels whose
    /// identity changed (or vanished) are flagged and kept for the user.
    pub fn take_resumable(
        &mut self,
        current: impl Fn(&str) -> Option<Identity>,
        inventory_ready: bool,
    ) -> Vec<StartRequest> {
        if !self.resume_pending {
            return Vec::new();
        }
        let since = *self.since.get_or_insert_with(std::time::Instant::now);
        if !inventory_ready && since.elapsed() < INVENTORY_WAIT {
            return Vec::new();
        }
        self.resume_pending = false;
        let mut resumable = Vec::new();
        for assignment in &self.list {
            let id = &assignment.request.control_id;
            match current(id) {
                Some(identity) if identity == assignment.identity => {
                    resumable.push(assignment.request.clone())
                }
                Some(_) => {
                    self.attention.insert(
                        id.clone(),
                        "Hardware identity or channel binding changed since the curve was assigned; check the channel and start it again".into(),
                    );
                }
                None => {
                    self.attention.insert(
                        id.clone(),
                        "Channel not found on this system; check the hardware and bindings".into(),
                    );
                }
            }
        }
        resumable
    }
    /// A resume attempt failed (policy, safety, worker): keep it visible.
    pub fn flag(&mut self, control_id: &str, reason: String) {
        self.attention.insert(control_id.to_owned(), reason);
    }
    pub fn view(&self) -> Value {
        json!({
            "assignments": self.list.iter().map(|a| json!({
                "control_id": a.request.control_id,
                "mode": a.request.mode,
                "temperature_id": a.request.temperature_id,
                "rpm_id": a.request.rpm_id,
                "duty_percent": a.request.duty_percent,
                "needs_attention": self.attention.get(&a.request.control_id),
            })).collect::<Vec<_>>(),
            "resume_pending": self.resume_pending,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_control::channels::{ChannelRole, ElectricalMode};

    fn request(control: &str, mode: &str) -> StartRequest {
        serde_json::from_value(json!({
            "mode": mode, "control_id": control,
            "temperature_id": "/lpc/nct6701d/0/temperature/0",
            "rpm_id": control.replace("control", "fan"), "confirmed_fan": true,
            "curve": null, "critical_c": null, "duty_percent": 50
        }))
        .unwrap()
    }
    fn identity(control: &str, header: &str) -> Identity {
        Identity {
            device_id: "/lpc/nct6701d/0".into(),
            control_id: control.into(),
            binding: Some(ChannelBinding {
                provider: "mainboard".into(),
                device_id: "/lpc/nct6701d/0".into(),
                control_id: format!("mainboard:{control}"),
                tachometer_id: None,
                header: header.into(),
                role: ChannelRole::Fan,
                electrical_mode: ElectricalMode::default(),
            }),
        }
    }

    #[test]
    fn unchanged_channels_resume_changed_ones_need_attention() {
        let dir = std::env::temp_dir().join(format!("freergb-assign-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("cooling-assignments.json");
        let (a, b, c) = (
            "/lpc/nct6701d/0/control/1",
            "/lpc/nct6701d/0/control/2",
            "/lpc/nct6701d/0/control/3",
        );
        let mut store = Assignments::default().with_file(path.clone());
        assert!(persistable(&request(a, "curve")) && !persistable(&request(a, "identify")));
        store.remember(request(a, "curve"), identity(a, "CHA_FAN1"));
        store.remember(request(b, "manual"), identity(b, "CHA_FAN2"));
        store.remember(request(c, "curve"), identity(c, "CPU_FAN"));
        store.remember(request(a, "manual"), identity(a, "CHA_FAN1"));
        assert_eq!(store.control_ids(), vec![b, c, a]);

        let mut restarted = Assignments::default().with_file(path.clone());
        // No inventory yet: wait.
        assert!(restarted.take_resumable(|_| None, false).is_empty());
        let resumed = restarted.take_resumable(
            |id| match id {
                x if x == a => Some(identity(a, "CHA_FAN1")),
                x if x == b => Some(identity(b, "CHA_FAN3")), // rebound header
                _ => None,                                    // vanished
            },
            true,
        );
        assert_eq!(resumed.len(), 1);
        assert_eq!(
            (resumed[0].control_id.as_str(), resumed[0].mode.as_str()),
            (a, "manual")
        );
        let view = restarted.view();
        let flagged: Vec<_> = view["assignments"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| !x["needs_attention"].is_null())
            .map(|x| x["control_id"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(flagged, vec![b.to_owned(), c.to_owned()]);
        // Resume happens once per daemon start.
        assert!(restarted.take_resumable(|_| None, true).is_empty());
        restarted.forget(b);
        restarted.forget_all();
        assert!(Assignments::default()
            .with_file(path)
            .control_ids()
            .is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
