//! Daemon-owned live cooling jobs. GUI only supplies explicit selection and user
//! intent; the protected service authorizes/binds native sources and checks IO.
use crate::{
    cooling_host::CoolingHost,
    cooling_runtime::{ControlMode, RuntimeLimits, RuntimeReport},
    native_bridge,
};
use freergb_control::channels::ChannelRole;
use freergb_control::fan_support::{self, ChannelEvidence, SupportTier};
use freergb_control::FanCurve;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartRequest {
    pub mode: String,
    pub control_id: String,
    pub temperature_id: String,
    pub rpm_id: String,
    pub confirmed_fan: bool,
    pub curve: Option<FanCurve>,
    pub critical_c: Option<f32>,
    /// Fixed duty for `mode:"manual"` (FanControl "manual percentage").
    #[serde(default)]
    pub duty_percent: Option<u8>,
    /// Composite curve output name for `mode:"composite"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composite_output: Option<String>,
    /// Daemon-owned output feed, attached by the engine (never from JSON).
    #[serde(skip)]
    pub composite_feed: Option<crate::composite_feed::CompositeFeed>,
}

/// Daemon-side facts about the selected control: the protected inventory's
/// capability plus user evidence (bindings, channel test, unlock). Never taken
/// from the request itself.
#[derive(Clone, Copy, Debug)]
pub struct SessionPolicy {
    pub capability: SupportTier,
    pub evidence: ChannelEvidence,
}
impl SessionPolicy {
    /// Behaviour of earlier releases: verified chip, fan confirmed per request.
    pub fn legacy() -> Self {
        Self {
            capability: SupportTier::Verified,
            evidence: ChannelEvidence::default(),
        }
    }
}

/// Fully resolved execution: what is authorized and how the runtime is bounded.
pub struct Plan {
    pub request: StartRequest,
    pub mode: ControlMode,
    pub critical_c: u8,
    /// Floor written into the protected authorization (enforced by the worker).
    pub minimum_percent: u8,
    pub tier: SupportTier,
    pub role: ChannelRole,
    pub limits: RuntimeLimits,
    /// Bounded run time (identification); `None` runs until stopped.
    pub duration: Option<Duration>,
}

/// How long `mode:"identify"` holds a header at 100%.
pub const IDENTIFY_SECONDS: u64 = 8;

impl StartRequest {
    #[cfg(test)]
    fn validate(&self) -> Result<(ControlMode, u8, u8), String> {
        let (mode, critical, minimum, _, _) = self.check(&SessionPolicy::legacy())?;
        Ok((mode, critical, minimum))
    }

    /// Manual percentage is a flat curve: same floors, critical-temperature
    /// override and supervision as any curve.
    fn normalized(mut self) -> Result<Self, String> {
        if self.mode == "identify" {
            // Never lowers a duty: a flat 100% curve, any role, bounded time.
            if self.curve.is_some() || self.duty_percent.is_some() {
                return Err("mode=identify takes no curve or duty".into());
            }
            self.mode = "manual".into();
            self.duty_percent = Some(100);
            self.confirmed_fan = true;
        }
        if self.mode != "manual" {
            if self.duty_percent.is_some() {
                return Err("duty_percent is only valid with mode=manual".into());
            }
            return Ok(self);
        }
        let duty = self
            .duty_percent
            .take()
            .filter(|d| *d <= 100)
            .ok_or("mode=manual requires duty_percent 0..100")?;
        if self.curve.is_some() {
            return Err("mode=manual takes no curve".into());
        }
        let critical = self.critical_c.take().unwrap_or(80.0);
        let duty = f32::from(duty);
        self.mode = "curve".into();
        self.curve = Some(FanCurve {
            points: vec![
                freergb_control::CurvePoint {
                    temperature_c: 0.0,
                    duty_percent: duty,
                },
                freergb_control::CurvePoint {
                    temperature_c: critical,
                    duty_percent: duty,
                },
            ],
            minimum_percent: duty,
            critical_c: critical,
            hysteresis_c: 0.0,
            ..FanCurve::default()
        });
        Ok(self)
    }

    pub fn plan(self, policy: &SessionPolicy) -> Result<Plan, String> {
        let identify = self.mode == "identify";
        let request = self.normalized()?;
        // Identification is 100% only: role and unlock evidence do not matter.
        let identify_policy = SessionPolicy {
            capability: policy.capability,
            evidence: ChannelEvidence::default(),
        };
        let policy = if identify { &identify_policy } else { policy };
        let (mode, critical_c, minimum_percent, tier, limits) = request.check(policy)?;
        let this = request;
        Ok(Plan {
            request: this,
            mode,
            critical_c,
            minimum_percent,
            tier,
            role: if policy.evidence.role == Some(ChannelRole::Pump) {
                ChannelRole::Pump
            } else {
                ChannelRole::Fan
            },
            limits,
            duration: identify.then(|| Duration::from_secs(IDENTIFY_SECONDS)),
        })
    }

    fn check(
        &self,
        policy: &SessionPolicy,
    ) -> Result<(ControlMode, u8, u8, SupportTier, RuntimeLimits), String> {
        // The role assigned during fan identification (bindings) is the
        // confirmation; `confirmed_fan` only stands in for unassigned headers.
        if !self.confirmed_fan && policy.evidence.role.is_none() {
            return Err(
                "Identify this header as a fan first (Cooling → the fan card → Identify)".into(),
            );
        }
        let pump = policy.evidence.role == Some(ChannelRole::Pump);
        // A pump is controlled only after an unrestricted unlock AND its second,
        // separate confirmation, and never tested or calibrated (100% default).
        if pump && !(policy.evidence.unlocked && policy.evidence.pump_confirmed) {
            return Err(
                "This channel is assigned as a pump; pump sessions are not available here".into(),
            );
        }
        if pump && self.mode != "curve" {
            return Err("Pumps are never calibrated or tested".into());
        }
        let evidence = ChannelEvidence {
            role: Some(if pump {
                ChannelRole::Pump
            } else {
                ChannelRole::Fan
            }),
            ..policy.evidence
        };
        let limits = fan_support::limits(policy.capability, evidence)?;
        let sensor = |id: &str, kind: &str| -> Option<String> {
            if id.len() > 128
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'/' || c == b'_')
            {
                return None;
            }
            let (parent, index) = id.rsplit_once('/')?;
            let index = index.parse::<u8>().ok()?;
            let base = parent.strip_suffix(&format!("/{kind}"))?;
            let parts: Vec<_> = base.split('/').collect();
            if parts.len() != 4
                || !parts[0].is_empty()
                || parts[1] != "lpc"
                || parts[2].is_empty()
                || parts[3].parse::<u16>().is_err()
                || (kind != "temperature" && index > 6)
            {
                return None;
            }
            Some(base.into())
        };
        let base = sensor(&self.control_id, "control").ok_or("Invalid native control identity")?;
        if sensor(&self.rpm_id, "fan").as_ref() != Some(&base) {
            return Err("The RPM sensor must be on the same native hardware as the control".into());
        }
        temperature_sources(&self.temperature_id)?;
        match self.mode.as_str() {
            "curve" if self.critical_c.is_none() => {
                let curve = self.curve.clone().ok_or("Curve required")?;
                curve.validate()?;
                if limits.floor_percent == 100
                    && limits.tier == SupportTier::LhmSupported
                    && curve.minimum_percent < 100.0
                {
                    return Err("Pass the guided channel test before running a reduced-duty curve on this LHM-supported channel".into());
                }
                if curve.minimum_percent < f32::from(limits.floor_percent)
                    || curve.critical_c > 100.0
                {
                    return Err(format!(
                        "Native curve floor >={}%, critical <=100 C required",
                        limits.floor_percent
                    ));
                }
                let critical = curve.critical_c.floor() as u8;
                let minimum = curve.minimum_percent.ceil() as u8;
                let runtime = RuntimeLimits {
                    floor_percent: minimum,
                    allow_stop: limits.allow_stop,
                };
                Ok((
                    ControlMode::Curve(curve),
                    critical,
                    minimum,
                    limits.tier,
                    runtime,
                ))
            }
            "composite" if self.curve.is_none() => {
                let output = self
                    .composite_output
                    .clone()
                    .filter(|o| !o.is_empty() && o.len() <= 64)
                    .ok_or("mode=composite requires composite_output")?;
                let critical = self.critical_c.unwrap_or(80.0);
                if !(40.0..=100.0).contains(&critical) {
                    return Err("Critical temperature must be 40..100 C".into());
                }
                if limits.floor_percent == 100 && limits.tier == SupportTier::LhmSupported {
                    return Err("Pass the guided channel test before running a composite curve on this LHM-supported channel".into());
                }
                Ok((
                    ControlMode::Composite {
                        output,
                        critical_c: critical,
                        // Without the daemon's feed every tick reads as 100%.
                        feed: self.composite_feed.clone().unwrap_or_default(),
                    },
                    critical.floor() as u8,
                    limits.floor_percent,
                    limits.tier,
                    RuntimeLimits {
                        floor_percent: limits.floor_percent,
                        allow_stop: limits.allow_stop,
                    },
                ))
            }
            mode @ ("calibration" | "channel_test") if self.curve.is_none() => {
                let critical = self.critical_c.unwrap_or(80.0);
                if !(40.0..=100.0).contains(&critical) {
                    return Err("Calibration critical temperature must be 40..100 C".into());
                }
                // Calibration (100/80/60/40) and the guided channel test (100/70/50)
                // are user-initiated measurements that never go below 40%, so they
                // are offered on untested channels too. The test authorizes 50%.
                let test = mode == "channel_test";
                Ok((
                    if test {
                        ControlMode::ChannelTest {
                            critical_c: critical,
                        }
                    } else {
                        ControlMode::Calibration {
                            critical_c: critical,
                        }
                    },
                    critical.floor() as u8,
                    if test { 50 } else { 40 },
                    limits.tier,
                    RuntimeLimits::default(),
                ))
            }
            _ => Err("Expected curve or calibration with unambiguous parameters".into()),
        }
    }
}
/// Curve temperature sources: one to three SuperIO, CPU or GPU temperatures
/// ("a,b,c"); the protected worker uses the hottest. Mirrors `NativeSources`.
pub fn temperature_sources(list: &str) -> Result<Vec<&str>, String> {
    let parts: Vec<&str> = list.split(',').collect();
    let valid = |id: &str| {
        let Some(rest) = id.strip_prefix('/') else {
            return false;
        };
        let segments: Vec<&str> = rest.split('/').collect();
        let tail_ok = |instance: &str, kind: &str, index: &str| {
            instance.parse::<u16>().is_ok() && kind == "temperature" && index.parse::<u16>().is_ok()
        };
        let name_ok = |name: &str| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        };
        id.len() <= 128
            && match segments.as_slice() {
                ["lpc", chip, instance, kind, index] => {
                    name_ok(chip) && tail_ok(instance, kind, index)
                }
                ["amdcpu" | "intelcpu", instance, kind, index] => tail_ok(instance, kind, index),
                [gpu, instance, kind, index] if gpu.starts_with("gpu-") => {
                    gpu[4..]
                        .split('-')
                        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_lowercase()))
                        && tail_ok(instance, kind, index)
                }
                _ => false,
            }
    };
    let mut seen = std::collections::BTreeSet::new();
    if !(1..=3).contains(&parts.len()) || parts.iter().any(|p| !valid(p) || !seen.insert(*p)) {
        return Err("Choose one to three temperature sources (motherboard, CPU or GPU)".into());
    }
    Ok(parts)
}

#[derive(Clone, Serialize)]
pub struct Status {
    pub available: bool,
    pub job_id: Option<u64>,
    pub state: String,
    pub report: Option<RuntimeReport>,
    pub error: Option<String>,
    /// Support tier of the current/last job ("verified", "lhm_supported", ...).
    pub tier: Option<&'static str>,
    /// Mode and native control of the current/last job.
    pub mode: Option<String>,
    pub control_id: Option<String>,
    /// All recent jobs (one per channel may run at the same time); the fields
    /// above describe the most recently started one.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub jobs: Vec<Status>,
}

const ACTIVE: [&str; 3] = ["authorizing", "executing", "stopping"];
/// Concurrent interactive sessions (one per channel, each its own task on the
/// shared worker); recent jobs kept for display. The command queue is bounded.
pub const MAX_ACTIVE: usize = 16;
const MAX_KEPT: usize = 32;
const QUEUE: usize = 2 * MAX_ACTIVE;

struct Board {
    available: bool,
    latest: Option<u64>,
    jobs: std::collections::BTreeMap<u64, Status>,
}
type View = Arc<Mutex<Board>>;
enum Command {
    Start(u64, Box<Plan>),
    Stop(u64),
}
pub struct CoolingSessions {
    sender: Option<mpsc::Sender<Command>>,
    view: View,
    thread: Option<std::thread::JoinHandle<()>>,
    next: u64,
}
impl Default for CoolingSessions {
    fn default() -> Self {
        Self {
            sender: None,
            view: Arc::new(Mutex::new(Board {
                available: false,
                latest: None,
                jobs: Default::default(),
            })),
            thread: None,
            next: 0,
        }
    }
}
fn idle(available: bool) -> Status {
    Status {
        available,
        job_id: None,
        state: if available { "idle" } else { "disabled" }.into(),
        report: None,
        error: None,
        tier: None,
        mode: None,
        control_id: None,
        jobs: Vec::new(),
    }
}
fn update(view: &View, id: u64, state: &str, report: Option<RuntimeReport>, error: Option<String>) {
    let mut board = view.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(job) = board.jobs.get_mut(&id) {
        job.state = state.into();
        job.report = report;
        job.error = error;
    }
}
impl CoolingSessions {
    pub fn enable(helper: PathBuf) -> Result<Self, String> {
        if !cfg!(windows) {
            return Err("Native cooling requires Windows".into());
        }
        let mut result = Self::default();
        let (tx, rx) = mpsc::channel(QUEUE);
        let view = result.view.clone();
        result
            .view
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .available = true;
        result.thread = Some(
            std::thread::Builder::new()
                .name("mainboard-cooling".into())
                .spawn(move || {
                    match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(runtime) => runtime.block_on(worker(helper, view, rx)),
                        Err(error) => tracing::error!(%error, "cooling runtime unavailable"),
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        result.sender = Some(tx);
        Ok(result)
    }
    pub fn status(&self) -> Status {
        let board = self.view.lock().unwrap_or_else(|e| e.into_inner());
        let mut status = board
            .latest
            .and_then(|id| board.jobs.get(&id).cloned())
            .unwrap_or_else(|| idle(board.available));
        status.available = board.available;
        status.jobs = board.jobs.values().rev().cloned().collect();
        status
    }
    pub fn start(&mut self, request: StartRequest, policy: &SessionPolicy) -> Result<u64, String> {
        let plan = request.plan(policy)?;
        let sender = self
            .sender
            .as_ref()
            .ok_or("Mainboard provider/control not enabled")?;
        {
            let board = self.view.lock().unwrap_or_else(|e| e.into_inner());
            if board.jobs.values().any(|j| j.state == "blocked") {
                return Err("Native recovery inspection required before daemon restart".into());
            }
            let active: Vec<_> = board
                .jobs
                .values()
                .filter(|j| ACTIVE.contains(&j.state.as_str()))
                .collect();
            if active
                .iter()
                .any(|j| j.control_id.as_deref() == Some(plan.request.control_id.as_str()))
            {
                return Err("A cooling job is already active on this channel".into());
            }
            if active.len() >= MAX_ACTIVE {
                return Err("Too many cooling jobs are active".into());
            }
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Cooling job identity exhausted")?;
        let id = self.next;
        {
            let mut board = self.view.lock().unwrap_or_else(|e| e.into_inner());
            let available = board.available;
            board.jobs.insert(
                id,
                Status {
                    available,
                    job_id: Some(id),
                    state: "authorizing".into(),
                    report: None,
                    error: None,
                    tier: Some(plan.tier.as_str()),
                    mode: Some(plan.request.mode.clone()),
                    control_id: Some(plan.request.control_id.clone()),
                    jobs: Vec::new(),
                },
            );
            board.latest = Some(id);
            // Forget the oldest finished jobs.
            while board.jobs.len() > MAX_KEPT {
                let Some(oldest) = board
                    .jobs
                    .iter()
                    .find(|(_, j)| !ACTIVE.contains(&j.state.as_str()) && j.state != "blocked")
                    .map(|(k, _)| *k)
                else {
                    break;
                };
                board.jobs.remove(&oldest);
            }
        }
        if sender.try_send(Command::Start(id, Box::new(plan))).is_err() {
            update(
                &self.view,
                id,
                "failed",
                None,
                Some("Cooling worker unavailable".into()),
            );
            return Err("Cooling worker unavailable".into());
        }
        Ok(id)
    }
    pub fn stop(&self, id: u64) -> Result<(), String> {
        let active = self
            .view
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .jobs
            .get(&id)
            .is_some_and(|j| ACTIVE.contains(&j.state.as_str()));
        if !active {
            return Err("Cooling job is not active".into());
        }
        self.sender
            .as_ref()
            .ok_or("Cooling disabled")?
            .try_send(Command::Stop(id))
            .map_err(|_| "Cooling queue unavailable".to_owned())
    }
}
impl CoolingSessions {
    /// Control of an active job, if it is active.
    pub fn active_control(&self, id: u64) -> Option<String> {
        let board = self.view.lock().unwrap_or_else(|e| e.into_inner());
        board
            .jobs
            .get(&id)
            .filter(|j| ACTIVE.contains(&j.state.as_str()))
            .and_then(|j| j.control_id.clone())
    }
    /// Active jobs, newest first.
    pub fn active(&self) -> Vec<Status> {
        let board = self.view.lock().unwrap_or_else(|e| e.into_inner());
        board
            .jobs
            .values()
            .rev()
            .filter(|j| ACTIVE.contains(&j.state.as_str()))
            .cloned()
            .collect()
    }
    /// Stops every active job (each returns its own channel). Returns how many
    /// stop requests were queued; a full queue is reported as an error.
    pub fn stop_all(&self) -> Result<usize, String> {
        let ids: Vec<u64> = self.active().iter().filter_map(|j| j.job_id).collect();
        let mut failed = None;
        for id in &ids {
            if let Err(error) = self.stop(*id) {
                failed = Some(error);
            }
        }
        match failed {
            Some(error) => Err(error),
            None => Ok(ids.len()),
        }
    }
}
impl Drop for CoolingSessions {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Dispatches commands; every job runs as its own task with its own stop line.
async fn worker(helper: PathBuf, view: View, mut commands: mpsc::Receiver<Command>) {
    let mut stops: std::collections::HashMap<u64, mpsc::Sender<()>> = Default::default();
    let mut tasks = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(Command::Start(id, plan)) => {
                    let (stop, stopped) = mpsc::channel(1);
                    stops.insert(id, stop);
                    tasks.spawn(job(helper.clone(), view.clone(), id, *plan, stopped));
                }
                Some(Command::Stop(id)) => {
                    if let Some(stop) = stops.get(&id) {
                        let _ = stop.try_send(());
                    }
                }
                None => break,
            },
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
        }
        stops.retain(|_, stop| !stop.is_closed());
    }
    // Shutdown: every job returns its channel (explicit close) before exit.
    stops.clear();
    while tasks.join_next().await.is_some() {}
}

async fn job(helper: PathBuf, view: View, id: u64, plan: Plan, mut stop: mpsc::Receiver<()>) {
    let Plan {
        request,
        mode,
        critical_c: critical,
        minimum_percent: minimum,
        tier,
        role,
        limits,
        duration,
    } = plan;
    // Do not cancel an elevated authorization child: it may still publish
    // authorization after the requesting helper is dropped. Check a queued stop
    // only after authorization finishes, before native acquisition.
    let authorized = native_bridge::authorize(
        &helper,
        &request.control_id,
        &request.temperature_id,
        &request.rpm_id,
        critical,
        minimum,
        tier.protected_name().map(|name| (name, role)),
    )
    .await;
    match stop.try_recv() {
        Ok(()) | Err(mpsc::error::TryRecvError::Disconnected) => {
            update(&view, id, "stopped", None, None);
            return;
        }
        Err(mpsc::error::TryRecvError::Empty) => {}
    }
    if let Err(error) = authorized {
        update(&view, id, "failed", None, Some(error));
        return;
    }
    let mut host = CoolingHost::new(true, Instant::now());
    if let Err(error) = host.start_native(
        native_bridge::connect(helper.clone(), request.control_id.clone()),
        mode,
        limits,
    ) {
        update(&view, id, "failed", None, Some(error));
        return;
    }
    update(&view, id, "executing", None, None);
    let mut ticks = tokio::time::interval(Duration::from_millis(100));
    let deadline = duration.map(|d| tokio::time::Instant::now() + d);
    loop {
        tokio::select! {
            _ = async { match deadline { Some(at) => tokio::time::sleep_until(at).await, None => std::future::pending().await } } => {
                // Bounded identification: explicit close (firmware return).
                update(&view,id,"stopping",None,None);
                match host.prepare_maintenance(20000).await {
                    Ok(())=>update(&view,id,"completed",host.last_report().cloned(),None),
                    Err(error)=>update(&view,id,"blocked",None,Some(error)),
                }
                return;
            },
            _=ticks.tick()=>{
                match host.collect_finished().await {
                    Ok(Some(report))=>{
                        let state = if report.fault.is_some() { "faulted" } else { "completed" };
                        update(&view,id,state,Some(report),None);return;
                    },
                    Ok(None)=>{},
                    Err(error)=>{update(&view,id,"blocked",None,Some(error));return;},
                }
            },
            _ = stop.recv() => {
                // Explicit stop or daemon shutdown (sender dropped).
                update(&view,id,"stopping",None,None);
                match host.prepare_maintenance(20000).await {
                    Ok(())=>update(&view,id,"stopped",host.last_report().cloned(),None),
                    Err(error)=>update(&view,id,"blocked",None,Some(error)),
                }
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> StartRequest {
        serde_json::from_value(json!({
            "mode":"calibration", "control_id":"/lpc/nct6701d/0/control/2",
            "rpm_id":"/lpc/nct6701d/0/fan/2", "temperature_id":"/lpc/nct6701d/0/temperature/1",
            "confirmed_fan":true
        }))
        .unwrap()
    }

    #[test]
    fn composite_requests_need_an_output_and_sane_critical() {
        let mut r = request();
        r.mode = "composite".into();
        assert!(r.validate().is_err());
        r.composite_output = Some("case".into());
        let (mode, critical, minimum) = r.validate().unwrap();
        assert!(matches!(mode, ControlMode::Composite { .. }));
        assert_eq!((critical, minimum), (80, 40));
        r.critical_c = Some(120.0);
        assert!(r.validate().is_err());
        r.critical_c = None;
        r.curve = Some(FanCurve::default());
        assert!(r.validate().is_err());
    }

    #[test]
    fn unsafe_or_ambiguous_native_requests_are_rejected() {
        assert!(request().validate().is_ok());
        let mut r = request();
        r.confirmed_fan = false;
        assert!(r.validate().is_err());
        r = request();
        r.rpm_id = "/lpc/nct6701d/1/fan/2".into();
        assert!(r.validate().is_err());
        r = request();
        r.control_id = "/lpc/nct6701d/0/control/7".into();
        assert!(r.validate().is_err());
        r = request();
        r.critical_c = Some(f32::NAN);
        assert!(r.validate().is_err());
        r = request();
        r.curve = Some(FanCurve::default());
        assert!(r.validate().is_err());
        r.mode = "curve".into();
        assert!(r.validate().is_err()); // default 25% floor cannot reach native IO
        let curve = r.curve.as_mut().unwrap();
        curve.minimum_percent = 40.0;
        curve.points[0].duty_percent = 40.0;
        assert!(r.validate().is_ok());
    }

    fn policy(
        capability: SupportTier,
        role: Option<ChannelRole>,
        test: bool,
        unlocked: bool,
    ) -> SessionPolicy {
        SessionPolicy {
            capability,
            evidence: ChannelEvidence {
                role,
                test_passed: test,
                unlocked,
                pump_confirmed: false,
            },
        }
    }

    fn curve(minimum: f32) -> FanCurve {
        let mut curve = FanCurve {
            minimum_percent: minimum,
            ..FanCurve::default()
        };
        for point in &mut curve.points {
            point.duty_percent = point.duty_percent.max(minimum);
        }
        curve
    }

    #[test]
    fn tiers_bound_curve_floors_and_protected_authorization() {
        let mut r = request();
        r.mode = "curve".into();
        r.curve = Some(curve(40.0));
        let legacy = r.plan(&SessionPolicy::legacy()).unwrap();
        assert_eq!(
            (
                legacy.tier,
                legacy.minimum_percent,
                legacy.tier.protected_name()
            ),
            (SupportTier::Verified, 40, None)
        );
        let untested =
            |r: StartRequest| r.plan(&policy(SupportTier::LhmSupported, None, false, false));
        let mut r = request();
        r.mode = "curve".into();
        r.curve = Some(curve(40.0));
        assert!(untested(r).err().unwrap().contains("guided channel test"));
        let mut r = request();
        r.mode = "curve".into();
        r.curve = Some(curve(40.0));
        let tested = r
            .plan(&policy(SupportTier::LhmSupported, None, true, false))
            .unwrap();
        assert_eq!(
            (tested.tier.protected_name(), tested.limits.allow_stop),
            (Some("lhm_supported"), false)
        );
        let mut r = request();
        r.mode = "curve".into();
        r.curve = Some(curve(0.0));
        assert!(r
            .plan(&policy(SupportTier::LhmSupported, None, true, false))
            .is_err());
        let mut r = request();
        r.mode = "curve".into();
        r.curve = Some(curve(0.0));
        let unlocked = r
            .plan(&policy(SupportTier::LhmSupported, None, false, true))
            .unwrap();
        assert_eq!(
            (
                unlocked.tier,
                unlocked.minimum_percent,
                unlocked.limits.allow_stop
            ),
            (SupportTier::Unrestricted, 0, true)
        );
        // Calibration stays >=40% and is offered for untested LHM channels.
        let calibration = request()
            .plan(&policy(SupportTier::LhmSupported, None, false, false))
            .unwrap();
        assert_eq!(
            (
                calibration.minimum_percent,
                calibration.limits,
                calibration.tier
            ),
            (40, RuntimeLimits::default(), SupportTier::LhmSupported)
        );
        assert!(request()
            .plan(&policy(
                SupportTier::Verified,
                Some(ChannelRole::Pump),
                true,
                false
            ))
            .is_err());
        assert!(request()
            .plan(&policy(SupportTier::ReadOnly, None, true, true))
            .is_err());
    }

    #[test]
    fn identify_holds_any_channel_at_full_speed_for_a_bounded_time() {
        let mut r = request();
        r.mode = "identify".into();
        r.critical_c = None;
        r.confirmed_fan = false;
        let pump = SessionPolicy {
            capability: SupportTier::LhmSupported,
            evidence: ChannelEvidence {
                role: Some(ChannelRole::Pump),
                ..ChannelEvidence::default()
            },
        };
        let plan = r.plan(&pump).unwrap();
        assert_eq!(plan.minimum_percent, 100);
        assert_eq!(plan.duration, Some(Duration::from_secs(IDENTIFY_SECONDS)));
        assert_eq!(plan.limits.floor_percent, 100);
        let mut curve = request();
        curve.mode = "identify".into();
        curve.critical_c = None;
        curve.curve = Some(FanCurve::default());
        assert!(curve.plan(&SessionPolicy::legacy()).is_err());
    }

    #[test]
    fn temperature_sources_accept_cpu_gpu_and_hottest_of_lists() {
        assert!(temperature_sources("/amdcpu/0/temperature/2").is_ok());
        assert_eq!(
            temperature_sources("/lpc/nct6701d/0/temperature/1,/gpu-nvidia/0/temperature/0")
                .unwrap()
                .len(),
            2
        );
        assert!(temperature_sources("/gpu-intel-integrated/0/temperature/0").is_ok());
        for bad in [
            "",
            "/amdcpu/0/load/1",
            "/x/0/temperature/1",
            "/amdcpu/0/temperature/1,/amdcpu/0/temperature/1",
            "/amdcpu/0/temperature/1,/amdcpu/0/temperature/2,/amdcpu/0/temperature/3,/amdcpu/0/temperature/4",
        ] {
            assert!(temperature_sources(bad).is_err(), "{bad}");
        }
        let mut r = request();
        r.temperature_id = "/intelcpu/0/temperature/0".into();
        assert!(r.validate().is_ok(), "CPU temperature with a SuperIO fan");
        let mut r = request();
        r.rpm_id = "/lpc/nct6701d/1/fan/2".into();
        assert!(r.validate().is_err(), "RPM must stay on the control chip");
    }

    #[test]
    fn manual_percentage_is_a_flat_supervised_curve() {
        let manual = |duty: u8| {
            let mut r = request();
            r.mode = "manual".into();
            r.critical_c = None;
            r.duty_percent = Some(duty);
            r
        };
        let plan = manual(60).plan(&SessionPolicy::legacy()).unwrap();
        assert_eq!((plan.minimum_percent, plan.critical_c), (60, 80));
        assert!(
            manual(30).plan(&SessionPolicy::legacy()).is_err(),
            "40% floor applies"
        );
        assert!(manual(101).plan(&SessionPolicy::legacy()).is_err());
        let unlocked = SessionPolicy {
            capability: SupportTier::LhmSupported,
            evidence: ChannelEvidence {
                role: None,
                test_passed: false,
                unlocked: true,
                pump_confirmed: false,
            },
        };
        assert_eq!(manual(0).plan(&unlocked).unwrap().minimum_percent, 0);
        let mut stray = request();
        stray.duty_percent = Some(50);
        assert!(stray.plan(&SessionPolicy::legacy()).is_err());
    }

    #[test]
    fn pumps_need_unlock_and_second_confirmation_and_only_run_curves() {
        let pump = |unlocked: bool, confirmed: bool| SessionPolicy {
            capability: SupportTier::LhmSupported,
            evidence: ChannelEvidence {
                role: Some(ChannelRole::Pump),
                test_passed: false,
                unlocked,
                pump_confirmed: confirmed,
            },
        };
        let curve_request = || {
            let mut r = request();
            r.mode = "curve".into();
            r.curve = Some(curve(20.0));
            r
        };
        assert!(curve_request().plan(&pump(false, false)).is_err());
        assert!(curve_request().plan(&pump(true, false)).is_err());
        let plan = curve_request().plan(&pump(true, true)).unwrap();
        assert_eq!(
            (plan.role, plan.tier, plan.minimum_percent),
            (ChannelRole::Pump, SupportTier::Unrestricted, 20)
        );
        assert!(
            request().plan(&pump(true, true)).is_err(),
            "no pump calibration"
        );
    }

    #[test]
    fn disabled_and_blocked_jobs_never_queue_acquisition() {
        let mut sessions = CoolingSessions::default();
        assert!(!sessions.status().available);
        assert!(sessions.start(request(), &SessionPolicy::legacy()).is_err());
        assert!(sessions.stop(1).is_err());
        let (sender, mut receiver) = mpsc::channel(8);
        sessions.sender = Some(sender);
        {
            let mut board = sessions.view.lock().unwrap();
            let mut blocked = idle(true);
            blocked.job_id = Some(7);
            blocked.state = "blocked".into();
            blocked.error = Some("return unverified".into());
            board.jobs.insert(7, blocked);
        }
        assert!(sessions.start(request(), &SessionPolicy::legacy()).is_err());
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn one_job_per_channel_and_several_channels_at_once() {
        let mut sessions = CoolingSessions::default();
        let (sender, mut receiver) = mpsc::channel(8);
        sessions.sender = Some(sender);
        sessions.view.lock().unwrap().available = true;
        let first = sessions.start(request(), &SessionPolicy::legacy()).unwrap();
        assert!(
            sessions.start(request(), &SessionPolicy::legacy()).is_err(),
            "same channel twice"
        );
        let mut other = request();
        other.control_id = "/lpc/nct6701d/0/control/3".into();
        other.rpm_id = "/lpc/nct6701d/0/fan/3".into();
        let second = sessions.start(other, &SessionPolicy::legacy()).unwrap();
        assert_ne!(first, second);
        assert!(matches!(receiver.try_recv(), Ok(Command::Start(id, _)) if id == first));
        assert!(matches!(receiver.try_recv(), Ok(Command::Start(id, _)) if id == second));
        let status = sessions.status();
        assert_eq!(status.job_id, Some(second));
        assert_eq!(status.jobs.len(), 2);
        assert!(sessions.stop(first).is_ok() && sessions.stop(99).is_err());
        assert!(matches!(receiver.try_recv(), Ok(Command::Stop(id)) if id == first));
    }

    #[test]
    fn sixteen_channels_at_most_and_stop_all_reaches_every_job() {
        let mut sessions = CoolingSessions::default();
        let (sender, mut receiver) = mpsc::channel(QUEUE);
        sessions.sender = Some(sender);
        sessions.view.lock().unwrap().available = true;
        let mut ids = Vec::new();
        for index in 0..MAX_ACTIVE {
            let mut channel = request();
            let (chip, header) = (index / 7, index % 7);
            channel.control_id = format!("/lpc/nct6701d/{chip}/control/{header}");
            channel.rpm_id = format!("/lpc/nct6701d/{chip}/fan/{header}");
            channel.temperature_id = format!("/lpc/nct6701d/{chip}/temperature/1");
            ids.push(sessions.start(channel, &SessionPolicy::legacy()).unwrap());
        }
        let mut extra = request();
        extra.control_id = "/lpc/nct6701d/2/control/5".into();
        extra.rpm_id = "/lpc/nct6701d/2/fan/5".into();
        extra.temperature_id = "/lpc/nct6701d/2/temperature/1".into();
        assert!(sessions.start(extra, &SessionPolicy::legacy()).is_err());
        assert_eq!(sessions.active().len(), MAX_ACTIVE);
        assert_eq!(
            sessions.active_control(ids[3]).as_deref(),
            Some("/lpc/nct6701d/0/control/3")
        );
        while receiver.try_recv().is_ok() {}
        assert_eq!(sessions.stop_all().unwrap(), MAX_ACTIVE);
        let mut stopped = Vec::new();
        while let Ok(Command::Stop(id)) = receiver.try_recv() {
            stopped.push(id);
        }
        stopped.sort_unstable();
        assert_eq!(stopped, ids);
        // A finished job on one channel leaves the others running.
        update(
            &sessions.view,
            ids[0],
            "faulted",
            None,
            Some("sensor lost".into()),
        );
        assert_eq!(sessions.active().len(), MAX_ACTIVE - 1);
        assert!(sessions.active_control(ids[0]).is_none());
    }
}
