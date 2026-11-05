//! API 3 cooling plugin host. The plugin is untrusted: every transport call is
//! checked against the signed policy, every sample against the declared
//! sensors, and every duty command passes the same rules as built-in channels.
//!
//! * Lease: each command carries a [`LEASE_MS`] lease enforced here by
//!   [`LeaseGuard`]; when the caller stops renewing, [`CoolingPlugin::poll`]
//!   requests 100 % and then firmware control.
//! * Floor and pumps: fan duty below the declared floor (never below
//!   [`MIN_FAN_FLOOR`]) is refused; pumps and unknown channels only accept 100 %.
//! * Freshness: a channel whose tachometer reading is older than `max_age_ms`
//!   cannot be reduced and an active reduction falls back.
//! * Fail-safe: a crash, hang, malformed answer or policy violation kills the
//!   process, then the host itself writes the policy's signed fail-safe
//!   reports on the daemon-owned handle. The plugin cannot block this.
use crate::host::{broker, Launch, Process, REQUEST_TIMEOUT};
use crate::store::VerifiedCooling;
use freergb_control::cooling_safety::{DutyLease, GuardState, LeaseGuard};
use freergb_device_api::HidTransport;
use freergb_module_api::cooling::{
    Description, Method, Policy, Request, Role, Sample, API, LEASE_MS, MIN_FAN_FLOOR,
};
use freergb_module_api::{
    decode, Call, ErrorCode, ModuleError, ModuleInfo, Op, Response, MAX_CALLS_PER_REQUEST,
    MAX_REPORT,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub const SAMPLE_TIMEOUT: Duration = Duration::from_secs(4);

fn protocol(message: impl Into<String>) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

fn violation(message: impl Into<String>) -> ModuleError {
    ModuleError::new(ErrorCode::Permission, message)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PluginReading {
    pub id: String,
    pub name: String,
    pub unit: freergb_module_api::cooling::Unit,
    /// `None` when absent, never sampled or older than `max_age_ms`.
    pub value: Option<f64>,
    pub age_ms: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct FailsafeOutcome {
    pub reports: usize,
    pub written: usize,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ControlState {
    pub channel: String,
    pub duty_percent: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PluginStatus {
    pub id: String,
    pub running: bool,
    pub fault: Option<String>,
    pub failsafe: Option<FailsafeOutcome>,
    pub controls: Vec<ControlState>,
    pub virtual_io: bool,
}

struct Control {
    guard: LeaseGuard,
    duty: u8,
}

pub struct CoolingPlugin {
    id: String,
    policy: Policy,
    process: Option<Process>,
    io: Option<Box<dyn HidTransport>>,
    next_id: u64,
    last_call: u64,
    sequence: u64,
    epoch: Instant,
    readings: BTreeMap<String, (Option<f64>, Instant)>,
    controls: BTreeMap<String, Control>,
    fault: Option<String>,
    failsafe: Option<FailsafeOutcome>,
}

impl CoolingPlugin {
    /// `io` is the daemon-owned handle for HID plugins (opened only after the
    /// device matched `policy.io.devices`); it must be `None` for virtual ones.
    pub fn start(
        verified: &VerifiedCooling,
        io: Option<Box<dyn HidTransport>>,
    ) -> Result<Self, ModuleError> {
        let manifest = verified.manifest();
        let policy = verified.policy().clone();
        if policy.is_virtual() != io.is_none() {
            return Err(violation("device handle does not match the plugin policy"));
        }
        let launch = Launch {
            module_id: manifest.id.clone(),
            executable: verified.verified.executable.clone(),
            directory: verified.verified.directory.clone(),
        };
        let mut plugin = Self {
            id: manifest.id.clone(),
            policy,
            process: None,
            io,
            next_id: 0,
            last_call: 0,
            sequence: 0,
            epoch: Instant::now(),
            readings: BTreeMap::new(),
            controls: BTreeMap::new(),
            fault: None,
            failsafe: None,
        };
        match Process::spawn(&launch) {
            Ok(process) => plugin.process = Some(process),
            Err(error) => {
                plugin.fail(&error.message);
                return Err(error);
            }
        }
        let checked = (|| {
            let info: ModuleInfo = serde_json::from_value(plugin.exchange(Method::Hello)?)
                .map_err(|_| protocol("invalid plugin hello"))?;
            if info.api != API
                || info.id != manifest.id
                || info.version != manifest.version
                || info.family != manifest.family
            {
                return Err(protocol("plugin identity mismatch"));
            }
            let described: Description = serde_json::from_value(plugin.exchange(Method::Describe)?)
                .map_err(|_| protocol("invalid plugin description"))?;
            if described.sensors != plugin.policy.sensors
                || described.channels != plugin.policy.channels
            {
                return Err(violation(
                    "plugin description differs from its signed policy",
                ));
            }
            Ok(())
        })();
        match checked {
            Ok(()) => Ok(plugin),
            Err(error) => {
                plugin.fail(&error.message);
                Err(error)
            }
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    pub fn is_running(&self) -> bool {
        self.process.is_some() && self.fault.is_none()
    }

    fn ms(&self, now: Instant) -> u64 {
        now.saturating_duration_since(self.epoch).as_millis() as u64
    }

    /// One request; any error faults the plugin (with fail-safe).
    fn request(&mut self, method: Method) -> Result<serde_json::Value, ModuleError> {
        let result = self.exchange(method);
        if let Err(error) = &result {
            self.fail(&error.message);
        }
        result
    }

    fn exchange(&mut self, method: Method) -> Result<serde_json::Value, ModuleError> {
        if let Some(fault) = &self.fault {
            return Err(ModuleError::new(ErrorCode::Disconnected, fault.clone()));
        }
        let timeout: Duration = match method {
            // A HID sample may need a dozen request/response exchanges.
            Method::Sample if !self.policy.is_virtual() => SAMPLE_TIMEOUT,
            _ => REQUEST_TIMEOUT,
        };
        let deadline = Instant::now() + timeout;
        let allows_io = method.allows_io();
        self.next_id += 1;
        let id = self.next_id;
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| protocol("plugin is not running"))?;
        process.send(&Request { v: API, id, method }, deadline)?;
        let mut calls = 0u32;
        loop {
            let message = process.receive(deadline)?;
            if message.get("call").is_some() {
                let call: Call =
                    serde_json::from_value(message).map_err(|_| protocol("invalid call"))?;
                calls += 1;
                if call.v != API || call.call <= self.last_call || calls > MAX_CALLS_PER_REQUEST {
                    return Err(protocol("invalid, replayed or excessive transport call"));
                }
                self.last_call = call.call;
                if !allows_io {
                    return Err(violation("policy violation: I/O outside a device request"));
                }
                let data = decode(&call.data)?;
                if data.len() > MAX_REPORT
                    || self.policy.authorize(call.op, &data, call.len).is_err()
                {
                    return Err(violation(format!(
                        "policy violation: {:?} of {} bytes is not allowed",
                        call.op,
                        if matches!(call.op, Op::Write | Op::SendFeature) {
                            data.len()
                        } else {
                            call.len
                        }
                    )));
                }
                let mut reply = broker(&call, &mut self.io);
                reply.v = API;
                process.send(&reply, deadline)?;
                continue;
            }
            let response: Response =
                serde_json::from_value(message).map_err(|_| protocol("invalid plugin response"))?;
            if response.v != API || response.id != id {
                return Err(protocol("plugin reply version/id mismatch"));
            }
            if let Some(error) = response.error {
                return Err(error);
            }
            return response
                .result
                .ok_or_else(|| protocol("plugin reply has no result"));
        }
    }

    /// Reads all sensors. Missing, duplicate, undeclared, non-finite or
    /// out-of-range values are malformed output and fault the plugin.
    pub fn sample(&mut self, now: Instant) -> Result<(), ModuleError> {
        if !self.policy.capability.sensors() {
            return Ok(());
        }
        let value = self.request(Method::Sample)?;
        let checked = serde_json::from_value::<Sample>(value)
            .map_err(|_| protocol("malformed sample"))
            .and_then(|sample| {
                let mut seen = BTreeMap::new();
                for reading in sample.sensors {
                    let decl = self
                        .policy
                        .sensor(&reading.id)
                        .ok_or_else(|| protocol("sample has an undeclared sensor"))?;
                    if let Some(v) = reading.value {
                        if !v.is_finite() || v < decl.min || v > decl.max {
                            return Err(protocol(format!(
                                "sensor {} value {v} outside its declared range",
                                decl.id
                            )));
                        }
                    }
                    if seen.insert(reading.id, reading.value).is_some() {
                        return Err(protocol("sample has a duplicate sensor"));
                    }
                }
                if seen.len() != self.policy.sensors.len() {
                    return Err(protocol("sample lacks declared sensors"));
                }
                Ok(seen)
            });
        match checked {
            Ok(values) => {
                for (id, value) in values {
                    self.readings.insert(id, (value, now));
                }
                Ok(())
            }
            Err(error) => {
                self.fail(&error.message);
                Err(error)
            }
        }
    }

    fn fresh(&self, id: &str, now: Instant) -> Option<(f64, u64)> {
        let (value, at) = self.readings.get(id)?;
        let age = now.saturating_duration_since(*at).as_millis() as u64;
        (age <= self.policy.max_age_ms && self.fault.is_none())
            .then_some(())
            .and(value.map(|v| (v, age)))
    }

    pub fn readings(&self, now: Instant) -> Vec<PluginReading> {
        self.policy
            .sensors
            .iter()
            .map(|s| {
                let fresh = self.fresh(&s.id, now);
                PluginReading {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    unit: s.unit,
                    value: fresh.map(|f| f.0),
                    age_ms: fresh.map(|f| f.1),
                }
            })
            .collect()
    }

    fn tachometer_fresh(&self, channel: &str, now: Instant) -> bool {
        self.policy
            .channel(channel)
            .and_then(|c| c.tachometer.as_deref())
            .is_none_or(|t| self.fresh(t, now).is_some())
    }

    /// Floor that applies to a channel: 100 for pumps/unknown, else the
    /// declared floor (validated >= [`MIN_FAN_FLOOR`]).
    pub fn floor(&self, channel: &str) -> u8 {
        match self.policy.channel(channel) {
            Some(c) if c.role == Role::Fan => c.floor_percent.max(MIN_FAN_FLOOR),
            _ => 100,
        }
    }

    /// Sets (or renews) a duty. Renew at least every [`LEASE_MS`].
    pub fn set_duty(&mut self, channel: &str, duty: u8, now: Instant) -> Result<(), String> {
        if let Some(fault) = &self.fault {
            return Err(format!("plugin faulted: {fault}"));
        }
        if !self.policy.capability.controls() {
            return Err("plugin declares no controls".into());
        }
        let decl = self
            .policy
            .channel(channel)
            .ok_or("unknown plugin channel")?
            .clone();
        if decl.role != Role::Fan && duty < 100 {
            return Err("pumps and unknown channels are never reduced".into());
        }
        let floor = self.floor(channel);
        if !(floor..=100).contains(&duty) {
            return Err(format!("duty must be {floor}..100"));
        }
        if duty < 100 && !self.tachometer_fresh(channel, now) {
            return Err("tachometer reading is missing or stale; only 100 % allowed".into());
        }
        let now_ms = self.ms(now);
        if !self.controls.contains_key(channel) {
            let guard = LeaseGuard::new(now_ms, LEASE_MS, floor)?;
            self.controls
                .insert(channel.to_owned(), Control { guard, duty: 100 });
        }
        self.sequence += 1;
        let sequence = self.sequence;
        let control = self.controls.get_mut(channel).expect("inserted above");
        control.guard.accept(
            DutyLease {
                sequence,
                issued_at_ms: now_ms,
                duty_percent: duty,
            },
            now_ms,
        )?;
        control.duty = duty;
        self.request(Method::SetDuty {
            channel: channel.to_owned(),
            duty_percent: duty,
            sequence,
            lease_ms: LEASE_MS,
        })
        .map(|_| ())
        .map_err(|e| e.message)
    }

    /// Hands one channel back to firmware control.
    pub fn release(&mut self, channel: &str) -> Result<(), String> {
        if self.controls.remove(channel).is_none() {
            return Ok(());
        }
        self.request(Method::Release {
            channel: channel.to_owned(),
        })
        .map(|_| ())
        .map_err(|e| e.message)
    }

    /// Supervisor tick: an expired lease or a stale tachometer requests 100 %
    /// and then firmware control. Returns the channels that fell back.
    pub fn poll(&mut self, now: Instant) -> Vec<String> {
        let now_ms = self.ms(now);
        let mut fallen = Vec::new();
        let ids: Vec<String> = self.controls.keys().cloned().collect();
        for id in ids {
            let stale = !self.tachometer_fresh(&id, now);
            let control = self.controls.get_mut(&id).expect("listed above");
            control.guard.poll(now_ms);
            if control.guard.state() != GuardState::Armed || (stale && control.duty < 100) {
                control.guard.stop();
                self.sequence += 1;
                let sequence = self.sequence;
                let full = self.request(Method::SetDuty {
                    channel: id.clone(),
                    duty_percent: 100,
                    sequence,
                    lease_ms: LEASE_MS,
                });
                if full.is_ok() {
                    let _ = self.release(&id);
                }
                self.controls.remove(&id);
                fallen.push(id);
            }
        }
        fallen
    }

    /// Releases every channel and stops the plugin cleanly.
    pub fn shutdown(&mut self) -> Result<(), ModuleError> {
        if self.fault.is_some() || self.process.is_none() {
            return Ok(());
        }
        let ids: Vec<String> = self.controls.keys().cloned().collect();
        for id in ids {
            self.release(&id).map_err(protocol)?;
        }
        self.request(Method::Shutdown)?;
        self.process = None;
        Ok(())
    }

    /// Kills the process first, then writes the signed fail-safe reports.
    fn fail(&mut self, reason: &str) {
        if self.fault.is_some() {
            return;
        }
        tracing::warn!(plugin = %self.id, %reason, "cooling plugin faulted; running fail-safe");
        self.fault = Some(reason.to_owned());
        self.controls.clear();
        self.process = None;
        let mut outcome = FailsafeOutcome {
            reports: self.policy.failsafe().len(),
            ..FailsafeOutcome::default()
        };
        if let Some(io) = self.io.as_mut() {
            for report in self.policy.failsafe() {
                let result = match report.op {
                    Op::SendFeature => io.send_feature(&report.data),
                    _ => io.write(&report.data),
                };
                match result {
                    Ok(_) => outcome.written += 1,
                    Err(e) => outcome.errors.push(e.to_string()),
                }
            }
        }
        self.failsafe = Some(outcome);
    }

    pub fn status(&self) -> PluginStatus {
        PluginStatus {
            id: self.id.clone(),
            running: self.is_running(),
            fault: self.fault.clone(),
            failsafe: self.failsafe.clone(),
            controls: self
                .controls
                .iter()
                .map(|(channel, c)| ControlState {
                    channel: channel.clone(),
                    duty_percent: c.duty,
                })
                .collect(),
            virtual_io: self.policy.is_virtual(),
        }
    }
}

impl Drop for CoolingPlugin {
    fn drop(&mut self) {
        // On error `shutdown` has already faulted and run the fail-safe.
        let _ = self.shutdown();
    }
}
