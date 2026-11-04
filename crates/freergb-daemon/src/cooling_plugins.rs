//! API 3 cooling plugins in the daemon: enable/disable, supervision and the
//! control loop. Contract and safety rules: `docs/COOLING_PLUGINS.md`.
//!
//! Plugins run on one supervisor thread, never on the engine tick: a hung
//! plugin delays only its own requests (bounded by the host deadlines) and
//! status reads return the last snapshot. Plugin sensors and channels appear
//! in the shared lists with `provider: "plugin"` (the GUI's source badge).
//!
//! Invariants, mirroring the built-in channels:
//! * nothing is written before `cooling.plugins.set` (after safety onboarding);
//! * every control tick renews the host lease; stopping the tick lets it expire
//!   (100 %, then firmware control);
//! * a missing composite output, a rejected reduction or a stale tachometer
//!   means 100 %; a faulted plugin drops its assignments after the host's
//!   fail-safe ran;
//! * restarts back off 2..60 s and stop after
//!   [`MAX_CONSECUTIVE_FAILURES`] until the plugin is re-enabled.
use crate::composite_feed::CompositeFeed;
use crate::mainboard::{Channel, SensorEntry};
use freergb_device_api::{HidTransport, SensorReading};
use freergb_hid::{HidInterfaceInfo, HidapiTransport};
use freergb_module_api::cooling::{Io, Policy, Unit};
use freergb_modules::cooling_host::{CoolingPlugin, PluginReading, PluginStatus};
use freergb_modules::host::MAX_CONSECUTIVE_FAILURES;
use freergb_modules::store::Store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub const PROVIDER: &str = "plugin";
const STATE_FILE: &str = "cooling-plugins.json";
const TICK: Duration = Duration::from_millis(250);
/// Renewal period; well inside the host lease.
const CONTROL_EVERY: Duration = Duration::from_millis(500);
const MAX_ASSIGNMENTS: usize = 32;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum PluginMode {
    Manual {
        percent: f32,
    },
    /// Follows a composite curve output (`cooling.curve_set`).
    Composite {
        output: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PluginAssignment {
    pub plugin: String,
    pub channel: String,
    #[serde(flatten)]
    pub mode: PluginMode,
}

#[derive(Default, Deserialize, Serialize)]
struct Persisted {
    version: u32,
    enabled: BTreeSet<String>,
}

#[derive(Default)]
struct Slot {
    plugin: Option<CoolingPlugin>,
    /// The supervisor thread holds the plugin outside the lock.
    busy: bool,
    policy: Option<Policy>,
    status: Option<PluginStatus>,
    readings: Vec<PluginReading>,
    failures: u32,
    retry_at: Option<Instant>,
    error: Option<String>,
    last_sample: Option<Instant>,
    last_control: Option<Instant>,
    /// Matching unclaimed HID interface from the last hardware scan.
    device: Option<HidInterfaceInfo>,
    /// Outcome of the last `cooling.plugins.install`.
    install: Option<String>,
}

#[derive(Default)]
struct Inner {
    store: Option<PathBuf>,
    state_dir: Option<PathBuf>,
    enabled: BTreeSet<String>,
    slots: BTreeMap<String, Slot>,
    assignments: BTreeMap<String, PluginAssignment>,
    faults: BTreeMap<String, String>,
    /// Composite outputs referenced by assignments, from the engine tick.
    feed: BTreeMap<String, f32>,
    stop: bool,
}

fn key(plugin: &str, channel: &str) -> String {
    format!("{plugin}/{channel}")
}

#[derive(Default)]
pub struct CoolingPlugins {
    inner: Arc<Mutex<Inner>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

fn lock(inner: &Mutex<Inner>) -> MutexGuard<'_, Inner> {
    inner.lock().unwrap_or_else(|e| e.into_inner())
}

impl Inner {
    fn save(&self) {
        let Some(dir) = &self.state_dir else {
            return;
        };
        let persisted = Persisted {
            version: 1,
            enabled: self.enabled.clone(),
        };
        let path = dir.join(STATE_FILE);
        let tmp = path.with_extension("json.tmp");
        let result = serde_json::to_vec_pretty(&persisted)
            .map_err(std::io::Error::other)
            .and_then(|bytes| std::fs::write(&tmp, bytes))
            .and_then(|()| std::fs::rename(&tmp, &path));
        if let Err(error) = result {
            tracing::warn!(%error, "cooling plugin state not written");
        }
    }

    /// Installed cooling plugins (cheap manifest read, no executable hashing).
    fn installed(&self) -> Vec<freergb_modules::package::PackageManifest> {
        self.store
            .as_ref()
            .and_then(|root| Store::open(root.clone()).ok())
            .map(|store| {
                store
                    .manifests()
                    .into_iter()
                    .filter(|m| m.cooling.is_some())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl CoolingPlugins {
    /// Starts the supervisor. `store` is the hardware module store root.
    pub fn start(&mut self, store: PathBuf, state_dir: Option<PathBuf>) {
        {
            let mut inner = lock(&self.inner);
            inner.enabled = state_dir
                .as_ref()
                .and_then(|d| std::fs::read(d.join(STATE_FILE)).ok())
                .filter(|b| b.len() <= 64 * 1024)
                .and_then(|b| serde_json::from_slice::<Persisted>(&b).ok())
                .map(|p| p.enabled)
                .unwrap_or_default();
            inner.store = Some(store);
            inner.state_dir = state_dir;
        }
        let inner = self.inner.clone();
        self.worker = std::thread::Builder::new()
            .name("cooling-plugins".into())
            .spawn(move || supervise(&inner))
            .ok();
    }

    /// Records the outcome of a background install for `view`.
    pub fn reporter(&self) -> impl Fn(&str, Result<(), String>) + Send + 'static {
        let inner = self.inner.clone();
        move |id, result| {
            let mut inner = lock(&inner);
            let slot = inner.slots.entry(id.to_owned()).or_default();
            slot.install = Some(match result {
                Ok(()) => "installed".to_owned(),
                Err(e) => format!("install failed: {e}"),
            });
        }
    }

    pub fn started(&self) -> bool {
        self.worker.is_some()
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<bool, String> {
        let mut inner = lock(&self.inner);
        if enabled && !inner.installed().iter().any(|m| m.id == id) {
            return Err("cooling plugin is not installed".into());
        }
        if enabled {
            inner.enabled.insert(id.to_owned());
            // Re-enabling clears the restart limit.
            let slot = inner.slots.entry(id.to_owned()).or_default();
            slot.failures = 0;
            slot.retry_at = None;
            slot.error = None;
        } else {
            inner.enabled.remove(id);
            inner.assignments.retain(|_, a| a.plugin != id);
        }
        inner.save();
        Ok(enabled)
    }

    pub fn assign(&self, assignment: PluginAssignment) -> Result<(), String> {
        let mut inner = lock(&self.inner);
        let slot = inner
            .slots
            .get(&assignment.plugin)
            .filter(|s| s.status.as_ref().is_some_and(|st| st.running))
            .ok_or("cooling plugin is not running")?;
        let policy = slot
            .policy
            .as_ref()
            .ok_or("cooling plugin is not running")?;
        let decl = policy
            .channel(&assignment.channel)
            .ok_or("unknown plugin channel; see cooling.plugins.list")?;
        let floor = f32::from(decl.floor_percent);
        match &assignment.mode {
            PluginMode::Manual { percent }
                if !percent.is_finite() || *percent < floor || *percent > 100.0 =>
            {
                return Err(format!("percent must be {floor}..100"));
            }
            PluginMode::Composite { output } if output.is_empty() || output.len() > 64 => {
                return Err("composite output name required".into());
            }
            _ => {}
        }
        let id = key(&assignment.plugin, &assignment.channel);
        if !inner.assignments.contains_key(&id) && inner.assignments.len() >= MAX_ASSIGNMENTS {
            return Err("too many plugin channel assignments".into());
        }
        inner.faults.remove(&id);
        inner.assignments.insert(id, assignment);
        Ok(())
    }

    /// Stops one assignment (or all); the supervisor releases the channel.
    pub fn release(&self, plugin: Option<&str>, channel: Option<&str>) -> usize {
        let mut inner = lock(&self.inner);
        let before = inner.assignments.len();
        inner.assignments.retain(|_, a| {
            !(plugin.is_none_or(|p| p == a.plugin) && channel.is_none_or(|c| c == a.channel))
        });
        before - inner.assignments.len()
    }

    /// Called every engine sensor tick with the shared composite feed.
    pub fn feed(&self, feed: &CompositeFeed, now: Instant) {
        let mut inner = lock(&self.inner);
        let outputs: BTreeSet<String> = inner
            .assignments
            .values()
            .filter_map(|a| match &a.mode {
                PluginMode::Composite { output } => Some(output.clone()),
                PluginMode::Manual { .. } => None,
            })
            .collect();
        inner.feed = outputs
            .into_iter()
            .map(|o| {
                let duty = feed.duty(&o, now);
                (o, duty)
            })
            .collect();
    }

    /// Hardware scan hook: remembers the first unclaimed interface matching
    /// each enabled HID plugin. Interfaces of built-in adapters and hardware
    /// modules are never handed to a plugin.
    pub fn scan(&self, list: &[HidInterfaceInfo], claimed: &dyn Fn(&HidInterfaceInfo) -> bool) {
        let mut inner = lock(&self.inner);
        let enabled = inner.enabled.clone();
        let installed = inner.installed();
        for manifest in installed.iter().filter(|m| enabled.contains(&m.id)) {
            let Some(Io::Hid { devices, .. }) = manifest.cooling.as_ref().map(|p| &p.io) else {
                continue;
            };
            let found = list
                .iter()
                .find(|i| {
                    !claimed(i)
                        && devices.iter().any(|d| {
                            d.vid == i.vid
                                && d.pid == i.pid
                                && d.interface.is_none_or(|n| n == i.interface)
                                && d.usage_page.is_none_or(|u| u == i.usage_page)
                        })
                })
                .cloned();
            inner.slots.entry(manifest.id.clone()).or_default().device = found;
        }
    }

    pub fn sensors(&self) -> Vec<SensorEntry> {
        let inner = lock(&self.inner);
        let mut out = Vec::new();
        for (id, slot) in &inner.slots {
            for r in &slot.readings {
                out.push(SensorEntry {
                    provider: PROVIDER,
                    device_id: format!("{PROVIDER}:{id}"),
                    device_name: id.clone(),
                    sensor: SensorReading {
                        id: format!("{PROVIDER}:{id}/{}", r.id),
                        name: r.name.clone(),
                        unit: unit(r.unit).into(),
                        value: r.value,
                        age_ms: r.age_ms.unwrap_or(u64::MAX),
                    },
                });
            }
        }
        out
    }

    pub fn channels(&self) -> Vec<Channel> {
        let inner = lock(&self.inner);
        let mut out = Vec::new();
        for (id, slot) in &inner.slots {
            let (Some(policy), Some(status)) = (&slot.policy, &slot.status) else {
                continue;
            };
            for c in &policy.channels {
                let tach = c
                    .tachometer
                    .as_ref()
                    .and_then(|t| slot.readings.iter().find(|r| &r.id == t));
                let channel_id = format!("{PROVIDER}:{id}/{}", c.id);
                out.push(Channel {
                    provider: PROVIDER,
                    id: channel_id.clone(),
                    device_id: format!("{PROVIDER}:{id}"),
                    name: c.name.clone(),
                    kind: "control".into(),
                    reading: SensorReading {
                        id: channel_id.clone(),
                        name: c.name.clone(),
                        unit: "RPM".into(),
                        value: tach.and_then(|r| r.value),
                        age_ms: tach.and_then(|r| r.age_ms).unwrap_or(u64::MAX),
                    },
                    control_available: status.running,
                    writable: status.running,
                    physical_mapping: "plugin_declared",
                    electrical_mode: "unknown",
                    sensor_id: channel_id,
                    hardware_name: id.clone(),
                    support: None,
                });
            }
        }
        out
    }

    /// `catalog` lists API 3 entries of the verified module catalog.
    pub fn view(&self, catalog: &[freergb_modules::catalog::ModuleEntry]) -> Value {
        let inner = lock(&self.inner);
        let installed = inner.installed();
        let mut plugins: Vec<Value> = installed
            .iter()
            .map(|m| {
                let slot = inner.slots.get(&m.id);
                let policy = m.cooling.as_ref().expect("filtered");
                let channels: Vec<Value> = policy
                    .channels
                    .iter()
                    .map(|c| {
                        let k = key(&m.id, &c.id);
                        json!({
                            "channel": c,
                            "assignment": inner.assignments.get(&k),
                            "applied_percent": slot
                                .and_then(|s| s.status.as_ref())
                                .and_then(|s| s.controls.iter().find(|x| x.channel == c.id))
                                .map(|x| x.duty_percent),
                            "fault": inner.faults.get(&k),
                        })
                    })
                    .collect();
                json!({
                    "id": m.id,
                    "version": m.version,
                    "family": m.family,
                    "license": m.license,
                    "installed": true,
                    "enabled": inner.enabled.contains(&m.id),
                    "capability": policy.capability,
                    "virtual": policy.is_virtual(),
                    "device_found": policy.is_virtual() || slot.is_some_and(|s| s.device.is_some()),
                    "status": slot.and_then(|s| s.status.as_ref()),
                    "error": slot.and_then(|s| s.error.as_ref()),
                    "install": slot.and_then(|s| s.install.as_ref()),
                    "failures": slot.map_or(0, |s| s.failures),
                    "sensors": slot.map(|s| s.readings.clone()).unwrap_or_default(),
                    "channels": channels,
                })
            })
            .collect();
        for entry in catalog {
            if !installed.iter().any(|m| m.id == entry.id) {
                plugins.push(json!({
                    "id": entry.id,
                    "name": entry.name,
                    "version": entry.version,
                    "family": entry.family,
                    "license": entry.license,
                    "installed": false,
                    "enabled": false,
                }));
            }
        }
        json!({ "started": self.started(), "plugins": plugins })
    }
}

fn unit(unit: Unit) -> &'static str {
    match unit {
        Unit::Celsius => "°C",
        Unit::Rpm => "RPM",
        Unit::Percent => "%",
    }
}

fn backoff(failures: u32) -> Duration {
    Duration::from_secs((2u64 << failures.min(5)).min(60))
}

/// Starts, samples, renews and stops plugins until `stop`.
fn supervise(shared: &Mutex<Inner>) {
    loop {
        let now = Instant::now();
        let work: Vec<String> = {
            let inner = lock(shared);
            if inner.stop {
                break;
            }
            inner
                .enabled
                .iter()
                .chain(inner.slots.keys())
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        };
        for id in work {
            step(shared, &id, now);
        }
        std::thread::sleep(TICK);
    }
    // Shutdown: every plugin releases its channels (or runs its fail-safe).
    let plugins: Vec<CoolingPlugin> = lock(shared)
        .slots
        .values_mut()
        .filter_map(|s| s.plugin.take())
        .collect();
    drop(plugins);
}

fn step(shared: &Mutex<Inner>, id: &str, now: Instant) {
    // Phase 1 (locked): decide and take the plugin out.
    let (mut plugin, assignments, feed, start, store) = {
        let mut inner = lock(shared);
        let enabled = inner.enabled.contains(id);
        let store = inner.store.clone();
        let assignments: Vec<PluginAssignment> = inner
            .assignments
            .values()
            .filter(|a| a.plugin == id)
            .cloned()
            .collect();
        let feed = inner.feed.clone();
        let slot = inner.slots.entry(id.to_owned()).or_default();
        if slot.busy {
            return;
        }
        if !enabled {
            if slot.plugin.is_none() {
                return;
            }
            slot.busy = true;
            let plugin = slot.plugin.take();
            drop(inner);
            drop(plugin); // releases every channel, then exits
            let mut inner = lock(shared);
            let slot = inner.slots.entry(id.to_owned()).or_default();
            slot.busy = false;
            if let Some(status) = &mut slot.status {
                status.running = false;
                status.controls.clear();
            }
            return;
        }
        let start = slot.plugin.is_none()
            && slot.failures < MAX_CONSECUTIVE_FAILURES
            && slot.retry_at.is_none_or(|at| now >= at);
        if slot.plugin.is_none() && !start {
            return;
        }
        slot.busy = true;
        let device = slot.device.clone();
        (
            slot.plugin.take(),
            assignments,
            feed,
            start.then_some(device),
            store,
        )
    };
    // Phase 2 (unlocked): blocking plugin I/O.
    let mut started_error = None;
    let launch_failed;
    let starting = start.is_some();
    if let Some(device) = start {
        match launch(id, store, device) {
            Ok(p) => plugin = Some(p),
            Err(e) => started_error = Some(e),
        }
    }
    launch_failed = started_error.is_some();
    let (due_sample, due_control) = {
        let inner = lock(shared);
        let slot = &inner.slots[id];
        let interval = slot
            .policy
            .as_ref()
            .or(plugin.as_ref().map(|p| p.policy()))
            .map_or(1000, |p| p.sample_interval_ms);
        (
            slot.last_sample
                .is_none_or(|at| now.saturating_duration_since(at).as_millis() as u64 >= interval),
            slot.last_control
                .is_none_or(|at| now.saturating_duration_since(at) >= CONTROL_EVERY),
        )
    };
    let mut faults = BTreeMap::new();
    if let Some(p) = plugin.as_mut() {
        if due_sample {
            let _ = p.sample(now);
        }
        for channel in p.poll(now) {
            faults.insert(
                key(id, &channel),
                "lease or tachometer expired; returned to firmware control".to_owned(),
            );
        }
        if due_control && p.is_running() {
            let active: BTreeSet<&str> = assignments.iter().map(|a| a.channel.as_str()).collect();
            for c in p.status().controls {
                if !active.contains(c.channel.as_str()) {
                    let _ = p.release(&c.channel);
                }
            }
            for a in &assignments {
                let wanted = match &a.mode {
                    PluginMode::Manual { percent } => *percent,
                    PluginMode::Composite { output } => feed.get(output).copied().unwrap_or(100.0),
                };
                let floor = p.floor(&a.channel);
                let duty = (wanted.ceil().clamp(0.0, 100.0) as u8).clamp(floor, 100);
                if let Err(error) = p.set_duty(&a.channel, duty, now) {
                    // A refused reduction is not a reason to lose control: full speed.
                    if !p.is_running() || p.set_duty(&a.channel, 100, now).is_err() {
                        faults.insert(key(id, &a.channel), error);
                    }
                }
            }
        }
    }
    // Phase 3 (locked): publish and put back.
    let mut inner = lock(shared);
    let running = plugin.as_ref().is_some_and(CoolingPlugin::is_running);
    if !running {
        inner.assignments.retain(|_, a| a.plugin != id);
    }
    inner.faults.extend(faults);
    let slot = inner.slots.entry(id.to_owned()).or_default();
    slot.busy = false;
    if due_sample {
        slot.last_sample = Some(now);
    }
    if due_control {
        slot.last_control = Some(now);
    }
    if let Some(p) = &plugin {
        slot.policy = Some(p.policy().clone());
        slot.status = Some(p.status());
        slot.readings = p.readings(now);
    }
    if let Some(error) = started_error {
        slot.error = Some(error);
    }
    if running {
        if starting {
            slot.error = None;
        }
        slot.failures = 0;
        slot.plugin = plugin;
    } else if plugin.is_some() || launch_failed {
        if let Some(fault) = plugin.as_ref().and_then(|p| p.status().fault) {
            slot.error = Some(fault);
        }
        slot.failures += 1;
        slot.retry_at = Some(now + backoff(slot.failures));
        slot.readings.iter_mut().for_each(|r| r.value = None);
        tracing::warn!(plugin = %id, failures = slot.failures, error = ?slot.error, "cooling plugin stopped");
    }
}

fn launch(
    id: &str,
    store: Option<PathBuf>,
    device: Option<HidInterfaceInfo>,
) -> Result<CoolingPlugin, String> {
    let store = Store::open(store.ok_or("no module store")?).map_err(|e| e.to_string())?;
    let verified = store.verified_cooling(id).map_err(|e| e.to_string())?;
    let io: Option<Box<dyn HidTransport>> = if verified.policy().is_virtual() {
        None
    } else {
        let device = device.ok_or("plugin device not found (or driven by a built-in adapter)")?;
        // The daemon owns the handle; the plugin only receives allowed reports.
        Some(Box::new(
            HidapiTransport::open(&device).map_err(|e| format!("cannot open the device: {e}"))?,
        ))
    };
    tracing::info!(plugin = %id, "starting cooling plugin");
    CoolingPlugin::start(&verified, io).map_err(|e| e.message)
}

impl Drop for CoolingPlugins {
    fn drop(&mut self) {
        lock(&self.inner).stop = true;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
