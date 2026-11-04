//! Rust cooling execution over an authenticated native session. No listener,
//! process elevation or automatic hardware activation is performed here.
use crate::native_session::{NativeObservation, NativeSessionClient, Receipt, SessionState};
use freergb_control::calibration::{
    Calibration, CalibrationPhase, CalibrationResult, CalibrationSample, SweepOptions,
};
use freergb_control::channels::ChannelRole;
use freergb_control::cooling_safety::{DutyLease, GuardState, SupervisedControl};
use freergb_control::rpm::{PiGains, RpmMap, RpmPi};
use freergb_control::shaping::{ChannelLimits, OutputShaper, OutputShaping};
use freergb_control::telemetry_guard::{Reading, TelemetryGuard};
use freergb_control::{CurveController, FanCurve};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

#[derive(Clone, Copy, Debug)]
pub struct SamplerFrame {
    pub temperature_c: f32,
    pub temperature_at_ms: u64,
    pub rpm: f32,
    pub rpm_at_ms: u64,
}
impl SamplerFrame {
    pub fn from_native(sample: NativeObservation, epoch: Instant) -> Result<Self, String> {
        let acquired = sample
            .acquired
            .checked_duration_since(epoch)
            .ok_or("Native sample predates cooling host epoch")?
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        Ok(Self {
            temperature_c: sample.temperature_c,
            rpm: sample.rpm,
            temperature_at_ms: acquired,
            rpm_at_ms: acquired,
        })
    }
}
pub enum ControlMode {
    Curve(FanCurve),
    /// Duty from a daemon-evaluated composite curve output
    /// (`crate::composite_feed`); missing/stale/fail-safe outputs mean 100%.
    Composite {
        output: String,
        critical_c: f32,
        feed: crate::composite_feed::CompositeFeed,
    },
    Calibration {
        critical_c: f32,
    },
    /// Guided channel test (100/70/50%), same supervision as calibration.
    ChannelTest {
        critical_c: f32,
    },
    /// Fine calibration sweep; below 40% only with a stall probe, which needs
    /// unrestricted limits (`allow_stop`).
    Sweep {
        critical_c: f32,
        options: SweepOptions,
    },
    /// Closed-loop RPM target: PI correction on the measured duty/RPM map;
    /// bad RPM telemetry runs open loop, bad temperature means 100%.
    Rpm {
        map: RpmMap,
        target_rpm: f32,
        gains: PiGains,
        critical_c: f32,
    },
    /// Any duty mode with per-channel output shaping
    /// (`freergb_control::shaping`) applied before the floor.
    Shaped {
        inner: Box<ControlMode>,
        shaping: OutputShaping,
    },
}
/// Duty limits for this execution, derived from the channel's support tier
/// (`freergb_control::fan_support::limits`). The default is the verified/legacy
/// 40% fan floor; only an unrestricted unlock sets `allow_stop`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeLimits {
    pub floor_percent: u8,
    pub allow_stop: bool,
}
impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            floor_percent: freergb_control::fan_support::SAFE_FAN_FLOOR,
            allow_stop: false,
        }
    }
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct RuntimeReport {
    pub calibration: Option<CalibrationResult>,
    pub native_closed: bool,
    /// Requires authenticated register-return AND durable journal proof. A closed
    /// callback from older workers alone cannot authorize maintenance. Lower tiers
    /// have no register readback: their owned firmware callback plus the cleared
    /// journal is the strongest evidence available (`return_evidence`).
    pub maintenance_authorized: bool,
    pub minimum_applied_percent: u8,
    pub initial_rpm: f32,
    pub last_rpm: f32,
    /// Support tier reported by the protected worker (`None`: legacy verified).
    pub tier: Option<String>,
    /// "register_readback", "firmware_callback" or "none".
    pub return_evidence: &'static str,
    /// Set when supervision or the worker ended the run and the channel was
    /// returned to firmware (lower tiers only); the run did not complete.
    pub fault: Option<String>,
    /// Last non-fatal condition (e.g. unrestricted fan stop).
    pub warning: Option<String>,
}

fn evidence(receipt: &Receipt) -> (bool, &'static str) {
    if receipt.firmware_return_verified && receipt.journal_cleared {
        (true, "register_readback")
    } else if receipt.tier.is_some() && receipt.firmware_returned && receipt.journal_cleared {
        (true, "firmware_callback")
    } else {
        (false, "none")
    }
}
enum Command {
    Duty(u8),
    Close,
}
struct NativeRequest {
    command: Command,
    reply: oneshot::Sender<Result<Receipt, String>>,
}
struct NativeTask(JoinHandle<()>);
impl Drop for NativeTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
struct Pending {
    reply: oneshot::Receiver<Result<Receipt, String>>,
    deadline_ms: u64,
    calibration_sequence: Option<u64>,
    closing: bool,
}

fn now(epoch: Instant) -> u64 {
    epoch.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn observe(
    control: &mut SupervisedControl,
    frame: SamplerFrame,
    previous: Option<SamplerFrame>,
    at: u64,
) -> Result<(), String> {
    for (source, acquired_ms, value, unchanged) in [
        (
            "temperature",
            frame.temperature_at_ms,
            frame.temperature_c,
            previous.is_some_and(|p| {
                p.temperature_at_ms == frame.temperature_at_ms
                    && p.temperature_c == frame.temperature_c
            }),
        ),
        (
            "rpm",
            frame.rpm_at_ms,
            frame.rpm,
            previous.is_some_and(|p| p.rpm_at_ms == frame.rpm_at_ms && p.rpm == frame.rpm),
        ),
    ] {
        if !unchanged {
            control
                .observe(
                    Reading {
                        source,
                        acquired_ms,
                        value,
                    },
                    at,
                )
                .map_err(|fault| format!("Unsafe telemetry: {fault:?}"))?;
        }
    }
    Ok(())
}

/// Execute a fan-only curve/calibration. The supervisor must bind the selected
/// sources to this session, translate acquisition times into `epoch`, and verify
/// hardware commissioning before permitting curve mode. GUI data is not a sampler.
/// Native I/O runs in a separate task so the timer/sampler are never blocked by it.
pub async fn run<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    client: NativeSessionClient<T>,
    mode: ControlMode,
    initial: SamplerFrame,
    samples: mpsc::Receiver<SamplerFrame>,
    stop: oneshot::Receiver<()>,
    epoch: Instant,
) -> Result<RuntimeReport, String> {
    run_with_limits(
        client,
        mode,
        RuntimeLimits::default(),
        initial,
        samples,
        stop,
        epoch,
    )
    .await
}

/// `run` with tier-specific limits. The worker independently enforces the same
/// floor (protected authorization), so a wrong limit here cannot widen it.
pub async fn run_with_limits<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    mut client: NativeSessionClient<T>,
    mode: ControlMode,
    limits: RuntimeLimits,
    initial: SamplerFrame,
    mut samples: mpsc::Receiver<SamplerFrame>,
    mut stop: oneshot::Receiver<()>,
    epoch: Instant,
) -> Result<RuntimeReport, String> {
    let at = now(epoch);
    if limits.floor_percent > 100
        || (limits.floor_percent < freergb_control::fan_support::SAFE_FAN_FLOOR
            && !limits.allow_stop)
        || (limits.allow_stop && client.tier() != Some("unrestricted"))
    {
        return Err("Runtime limits do not match the admitted support tier".into());
    }
    let tier = client.tier().map(str::to_owned);
    // Lower tiers return to their captured firmware mode on any supervision fault
    // (explicit close); legacy verified workers keep holding 100% for recovery.
    let graceful = tier.is_some();
    let (mode, shaping) = match mode {
        ControlMode::Shaped { inner, shaping } => {
            if matches!(
                *inner,
                ControlMode::Shaped { .. }
                    | ControlMode::Calibration { .. }
                    | ControlMode::ChannelTest { .. }
                    | ControlMode::Sweep { .. }
            ) {
                return Err("Output shaping applies to curve, composite and RPM modes only".into());
            }
            shaping.validate_for(limits.allow_stop, false)?;
            (*inner, Some(shaping))
        }
        mode => (mode, None),
    };
    if let ControlMode::Sweep { options, .. } = &mode {
        options.validate()?;
        if options.probe_stall && !limits.allow_stop {
            return Err("Stall sweeps need an unrestricted channel".into());
        }
    }
    if let ControlMode::Rpm {
        gains, target_rpm, ..
    } = &mode
    {
        gains.validate()?;
        if !target_rpm.is_finite() || *target_rpm <= 0.0 {
            return Err("RPM target must be positive".into());
        }
    }
    let critical = match &mode {
        ControlMode::Curve(curve) => {
            curve.validate()?;
            if curve.minimum_percent < f32::from(limits.floor_percent) {
                return Err(format!(
                    "Native curve floor must be at least {} percent for this channel",
                    limits.floor_percent
                ));
            }
            curve.critical_c
        }
        ControlMode::Calibration { critical_c }
        | ControlMode::ChannelTest { critical_c }
        | ControlMode::Sweep { critical_c, .. }
        | ControlMode::Rpm { critical_c, .. }
        | ControlMode::Composite { critical_c, .. } => {
            if !(40.0..=100.0).contains(critical_c) {
                return Err("Critical temperature must be 40..100 C".into());
            }
            *critical_c
        }
        ControlMode::Shaped { .. } => unreachable!("unwrapped above"),
    };
    let mut telemetry = TelemetryGuard::new("temperature".into(), "rpm".into(), critical)?;
    if limits.allow_stop {
        telemetry = telemetry.allow_fan_stop();
    }
    telemetry
        .observe(
            Reading {
                source: "temperature",
                acquired_ms: initial.temperature_at_ms,
                value: initial.temperature_c,
            },
            at,
        )
        .map_err(|fault| format!("Unsafe initial temperature: {fault:?}"))?;
    telemetry
        .observe(
            Reading {
                source: "rpm",
                acquired_ms: initial.rpm_at_ms,
                value: initial.rpm,
            },
            at,
        )
        .map_err(|fault| format!("Unsafe initial RPM: {fault:?}"))?;
    if initial.rpm < 100.0 && !limits.allow_stop {
        return Err("A running, explicitly selected fan is required before control".into());
    }
    let mut control = SupervisedControl::with_limits(
        telemetry,
        at,
        1000,
        limits.floor_percent,
        limits.allow_stop,
    )?;
    let native_sampled = client.observation().is_some();
    let (native_samples, mut native_receiver) = mpsc::channel(4);
    let (tx, mut rx) = mpsc::channel::<NativeRequest>(1);
    let _native_task = NativeTask(tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            let closing = matches!(request.command, Command::Close);
            let result = match request.command {
                Command::Duty(duty) => client.set_duty(duty).await,
                Command::Close => client.close().await,
            };
            let terminal = result
                .as_ref()
                .map_or(true, |receipt| receipt.state != SessionState::Active);
            if !terminal {
                if let Some(sample) = client.observation() {
                    if native_samples.send(sample).await.is_err() {
                        break;
                    }
                }
            }
            let _ = request.reply.send(result);
            if closing || terminal {
                break;
            }
        }
    }));
    let mut curve_controller = CurveController::default();
    let mut shaper = OutputShaper::default();
    let mut rpm_loop = RpmPi::default();
    let mut calibration = match mode {
        ControlMode::Calibration { critical_c } => {
            Some(Calibration::new(ChannelRole::Fan, critical_c, at)?)
        }
        ControlMode::ChannelTest { critical_c } => {
            Some(Calibration::channel_test(ChannelRole::Fan, critical_c, at)?)
        }
        ControlMode::Sweep {
            critical_c,
            options,
        } => Some(Calibration::sweep(
            ChannelRole::Fan,
            critical_c,
            at,
            options,
        )?),
        _ => None,
    };
    let mut frame = initial;
    let mut minimum_applied_percent = 100;
    let mut pending: Option<Pending> = None;
    let mut sequence = 0;
    let mut applied_calibration_sequence = None;
    let mut next_send = at;
    let mut closing = false;
    let mut result = None;
    let mut fault: Option<String> = None;
    let mut warning: Option<String> = None;
    let mut ticker = tokio::time::interval(Duration::from_millis(25));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            incoming = samples.recv(), if !closing && !native_sampled => {
                let incoming = incoming.ok_or("Trusted sampler disconnected; preserve recovery intent")?;
                if let Err(error) = observe(&mut control,incoming,Some(frame),now(epoch)) {
                    if !graceful { return Err(error); }
                    fault = Some(error); closing = true;
                }
                frame = incoming;
            },
            incoming = native_receiver.recv(), if !closing && native_sampled => {
                let incoming = SamplerFrame::from_native(incoming.ok_or("Native sampler transport stopped")?,epoch)?;
                if let Err(error) = observe(&mut control,incoming,Some(frame),now(epoch)) {
                    if !graceful { return Err(error); }
                    fault = Some(error); closing = true;
                }
                frame = incoming;
            },
            _ = &mut stop, if !closing => { closing = true; },
            receipt = async {
                match pending.as_mut() {
                    Some(pending) => (&mut pending.reply).await,
                    None => std::future::pending().await,
                }
            } => {
                let receipt = receipt.map_err(|_| "Native I/O task stopped")??;
                let operation = pending.take().ok_or("Unmatched native receipt")?;
                if receipt.warning.is_some() { warning = receipt.warning.clone(); }
                let returned = receipt.fault_returned();
                if operation.closing || returned {
                    if !returned && (receipt.state != SessionState::Closed || receipt.owned) { return Err("Native return failed; recovery required".into()); }
                    let (maintenance_authorized, return_evidence) = evidence(&receipt);
                    if returned { fault = fault.or_else(|| Some(format!("Worker returned the channel to firmware: {}", receipt.reason.as_deref().unwrap_or("fault")))); }
                    return Ok(RuntimeReport { calibration: result,native_closed: true,maintenance_authorized,
                        minimum_applied_percent, initial_rpm: initial.rpm,last_rpm: frame.rpm, tier, return_evidence, fault, warning });
                }
                if receipt.state != SessionState::Active || !receipt.owned { return Err("Native session entered emergency; recovery required".into()); }
                minimum_applied_percent = minimum_applied_percent.min(receipt.duty_percent);
                applied_calibration_sequence = operation.calibration_sequence;
                next_send = now(epoch).saturating_add(100);
            }
        }
        let at = now(epoch);
        control.poll(at);
        if !closing && control.lease_state() != GuardState::Armed {
            let message = format!(
                "Cooling supervision stopped: {:?}, {:?}",
                control.lease_state(),
                control.telemetry_fault()
            );
            if !graceful {
                return Err(format!("{message}; preserve recovery intent"));
            }
            // Explicit close: 100%, then the worker's owned firmware return.
            fault = Some(message);
            closing = true;
        }
        if let Some(condition) = control.telemetry_warning() {
            warning = Some(format!("{condition:?}"));
        }
        if pending
            .as_ref()
            .is_some_and(|operation| at >= operation.deadline_ms)
        {
            return Err(
                "Native acknowledgement deadline exceeded; preserve recovery intent".into(),
            );
        }
        if pending.is_some() {
            continue;
        }
        if !closing && at < next_send {
            continue;
        }
        let mut calibration_sequence = None;
        let duty = if closing {
            100
        } else if let Some(calibration) = calibration.as_mut() {
            let output = calibration.update(CalibrationSample {
                at_ms: at,
                applied_sequence: applied_calibration_sequence,
                rpm: Some(frame.rpm),
                rpm_age_ms: at.saturating_sub(frame.rpm_at_ms),
                temperature_c: Some(frame.temperature_c),
                temperature_age_ms: at.saturating_sub(frame.temperature_at_ms),
            });
            if matches!(
                output.phase,
                CalibrationPhase::Completed | CalibrationPhase::Aborted
            ) {
                result = output.result;
                closing = true;
            } else if output.phase == CalibrationPhase::RestorationFailed {
                return Err("Calibration restoration failed; preserve recovery intent".into());
            }
            calibration_sequence = Some(output.sequence);
            output.duty_percent
        } else {
            let temperature_age = at.saturating_sub(frame.temperature_at_ms);
            // (duty, fail-safe): fail-safe outputs bypass every shaping step.
            let (raw, failsafe) = match &mode {
                ControlMode::Curve(curve) => {
                    let duty = curve_controller.update_at(
                        curve,
                        Some(frame.temperature_c),
                        temperature_age,
                        at,
                    );
                    let hot = frame.temperature_c >= curve.critical_c || temperature_age > 3000;
                    (duty, hot)
                }
                ControlMode::Composite {
                    output,
                    critical_c,
                    feed,
                } => {
                    // The session's own temperature still forces 100% at critical.
                    let hot = frame.temperature_c >= *critical_c || temperature_age > 3000;
                    let duty = if hot {
                        100.0
                    } else {
                        feed.duty(output, Instant::now())
                    };
                    // The feed reports untrusted outputs as 100%.
                    (duty, hot || duty >= 100.0)
                }
                ControlMode::Rpm {
                    map,
                    target_rpm,
                    gains,
                    critical_c,
                } => {
                    let hot = frame.temperature_c >= *critical_c || temperature_age > 3000;
                    let out = rpm_loop.update(
                        map,
                        *gains,
                        *target_rpm,
                        Some(frame.rpm),
                        at.saturating_sub(frame.rpm_at_ms),
                        at,
                    );
                    if hot {
                        (100.0, true)
                    } else {
                        (out.duty_percent, false)
                    }
                }
                _ => return Err("Missing cooling mode".into()),
            };
            let duty = match &shaping {
                Some(shaping) => shaper.apply(
                    shaping,
                    raw,
                    failsafe,
                    ChannelLimits {
                        floor_percent: f32::from(limits.floor_percent),
                        allow_stop: limits.allow_stop,
                    },
                    at,
                ),
                None => raw,
            };
            duty.ceil().clamp(f32::from(limits.floor_percent), 100.0) as u8
        };
        sequence += 1;
        if !closing {
            control.accept(
                DutyLease {
                    sequence,
                    issued_at_ms: at,
                    duty_percent: duty,
                },
                at,
            )?;
        }
        let (reply, receiver) = oneshot::channel();
        tx.try_send(NativeRequest {
            command: if closing {
                Command::Close
            } else {
                Command::Duty(duty)
            },
            reply,
        })
        .map_err(|_| "Native I/O queue unavailable; preserve recovery intent")?;
        pending = Some(Pending {
            reply: receiver,
            deadline_ms: at.saturating_add(150),
            calibration_sequence,
            closing,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    #[tokio::test]
    async fn healthy_commands_cannot_mask_stale_temperature_or_stalled_fan() {
        healthy_commands_check().await;
    }

    /// Tiered worker peer: echoes duties with fresh tickets; a close returns
    /// firmware via the owned callback and reports the cleared journal.
    fn tiered_peer(
        worker: tokio::io::DuplexStream,
        tier: &'static str,
    ) -> tokio::task::JoinHandle<Vec<serde_json::Value>> {
        tokio::spawn(async move {
            let mut worker = BufReader::new(worker);
            let mut reply = json!({"version":1,"session":"a".repeat(32),"ticket":"b".repeat(32),
                "sequence":0,"state":"active","reason":null,"duty_percent":100,"ticket_ms":250,
                "lease_ms":1000,"owned":true,"physical_write_verified":false,"tier":tier,
                "firmware_returned":false,"warning":null});
            worker
                .get_mut()
                .write_all(format!("{reply}\n").as_bytes())
                .await
                .unwrap();
            let mut seen = Vec::new();
            loop {
                let mut line = String::new();
                if worker.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                let request: serde_json::Value = serde_json::from_str(&line).unwrap();
                reply["sequence"] = request["sequence"].clone();
                reply["ticket"] = json!(format!("{:032x}", seen.len() + 1));
                let close = request["action"] == "close";
                if close {
                    reply["state"] = json!("closed");
                    reply["owned"] = json!(false);
                    reply["duty_percent"] = json!(100);
                    reply["firmware_returned"] = json!(true);
                    reply["journal_cleared"] = json!(true);
                } else {
                    reply["duty_percent"] = request["duty_percent"].clone();
                    reply["warning"] = if request["duty_percent"] == 0 {
                        json!("fan_stalled")
                    } else {
                        serde_json::Value::Null
                    };
                }
                seen.push(request);
                worker
                    .get_mut()
                    .write_all(format!("{reply}\n").as_bytes())
                    .await
                    .unwrap();
                if close {
                    break;
                }
            }
            seen
        })
    }

    fn feed(
        samples: mpsc::Sender<SamplerFrame>,
        epoch: Instant,
        rpm: f32,
        stale_temperature: bool,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(25)).await;
                let at = now(epoch);
                let frame = SamplerFrame {
                    temperature_c: 30.0,
                    temperature_at_ms: if stale_temperature { 0 } else { at },
                    rpm,
                    rpm_at_ms: at,
                };
                if samples.send(frame).await.is_err() {
                    break;
                }
            }
        })
    }

    #[tokio::test]
    async fn unrestricted_curve_may_stop_the_fan_and_reports_callback_return() {
        let (transport, worker) = tokio::io::duplex(4096);
        let peer = tiered_peer(worker, "unrestricted");
        let client = NativeSessionClient::from_authenticated_transport(transport)
            .await
            .unwrap();
        let epoch = Instant::now();
        let (samples, rx) = mpsc::channel(4);
        let (stop, stop_rx) = oneshot::channel();
        let mut curve = FanCurve {
            minimum_percent: 0.0,
            ..FanCurve::default()
        };
        curve.points[0].duty_percent = 0.0;
        let sampler = feed(samples, epoch, 0.0, false);
        let runtime = tokio::spawn(run_with_limits(
            client,
            ControlMode::Curve(curve),
            RuntimeLimits {
                floor_percent: 0,
                allow_stop: true,
            },
            SamplerFrame {
                temperature_c: 20.0,
                temperature_at_ms: 0,
                rpm: 0.0,
                rpm_at_ms: 0,
            },
            rx,
            stop_rx,
            epoch,
        ));
        tokio::time::sleep(Duration::from_millis(1600)).await;
        stop.send(()).unwrap();
        let report = tokio::time::timeout(Duration::from_secs(2), runtime)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(report.tier.as_deref(), Some("unrestricted"));
        assert_eq!(report.minimum_applied_percent, 0);
        assert_eq!(
            (report.return_evidence, report.maintenance_authorized),
            ("firmware_callback", true)
        );
        assert!(report.fault.is_none(), "{:?}", report.fault);
        assert!(report.warning.is_some(), "fan stop must surface a warning");
        let seen = peer.await.unwrap();
        assert!(seen.iter().any(|r| r["duty_percent"] == 0));
        sampler.abort();
    }

    #[tokio::test]
    async fn lower_tier_supervision_fault_returns_to_firmware_instead_of_blocking() {
        let (transport, worker) = tokio::io::duplex(4096);
        let peer = tiered_peer(worker, "lhm_supported");
        let client = NativeSessionClient::from_authenticated_transport(transport)
            .await
            .unwrap();
        let epoch = Instant::now();
        let (samples, rx) = mpsc::channel(4);
        let (_stop, stop_rx) = oneshot::channel();
        let sampler = feed(samples, epoch, 900.0, true);
        let curve = FanCurve {
            minimum_percent: 40.0,
            points: FanCurve::default()
                .points
                .into_iter()
                .map(|mut p| {
                    p.duty_percent = p.duty_percent.max(40.0);
                    p
                })
                .collect(),
            ..FanCurve::default()
        };
        let report = tokio::time::timeout(
            Duration::from_secs(4),
            run_with_limits(
                client,
                ControlMode::Curve(curve),
                RuntimeLimits::default(),
                SamplerFrame {
                    temperature_c: 30.0,
                    temperature_at_ms: 0,
                    rpm: 900.0,
                    rpm_at_ms: 0,
                },
                rx,
                stop_rx,
                epoch,
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            report.fault.as_deref().unwrap().contains("Stale"),
            "{:?}",
            report.fault
        );
        assert_eq!(report.return_evidence, "firmware_callback");
        let seen = peer.await.unwrap();
        assert_eq!(seen.last().unwrap()["action"], "close");
        assert!(seen
            .iter()
            .all(|r| r["action"] == "close" || r["duty_percent"].as_u64().unwrap() >= 40));
        sampler.abort();
    }

    /// Simulated boot path: the service-mode boot profile resolves to the same
    /// controllers a live session runs (curve and composite here), each against
    /// its own simulated protected worker, and every channel returns on stop.
    #[tokio::test]
    async fn simulated_boot_runs_the_profile_controllers_headless() {
        use crate::cooling_service::{boot_action, BootAction, RunMode, ServiceConfig};
        let mut curve = FanCurve {
            minimum_percent: 40.0,
            ..FanCurve::default()
        };
        for point in &mut curve.points {
            point.duty_percent = point.duty_percent.max(40.0);
        }
        let channel = |index: u8, mode: &str| {
            json!({
                "mode": mode,
                "control_id": format!("/lpc/nct6701d/0/control/{index}"),
                "temperature_id": "/lpc/nct6701d/0/temperature/0",
                "rpm_id": format!("/lpc/nct6701d/0/fan/{index}"),
                "confirmed_fan": true,
                "curve": if mode == "curve" { json!(curve) } else { serde_json::Value::Null },
                "critical_c": null,
                "composite_output": if mode == "composite" { json!("boot_case") } else { serde_json::Value::Null },
            })
        };
        let profile: crate::cooling_profiles::CoolingProfile = serde_json::from_value(
            json!({"assignments": [channel(1, "curve"), channel(2, "composite")]}),
        )
        .unwrap();
        profile.validate().unwrap();
        let config = ServiceConfig {
            version: 1,
            mode: RunMode::Service,
            boot_profile: Some("Boot".into()),
        };
        assert_eq!(
            boot_action(&config, &["Boot".to_string()]),
            BootAction::ActivateProfile("Boot".into())
        );
        let feed_out = crate::composite_feed::CompositeFeed::default();
        let publisher = {
            let feed_out = feed_out.clone();
            tokio::spawn(async move {
                loop {
                    let ok = freergb_control::curves::CurveOutput {
                        percent: 65.0,
                        failsafe: false,
                    };
                    feed_out.publish([("boot_case".to_string(), ok)].into(), Instant::now());
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            })
        };
        let epoch = Instant::now();
        let mut runs = Vec::new();
        for mut request in profile.assignments.unwrap() {
            request.composite_feed = Some(feed_out.clone());
            let plan = request
                .plan(&crate::cooling_sessions::SessionPolicy::legacy())
                .unwrap();
            let (transport, worker) = tokio::io::duplex(4096);
            let peer = tiered_peer(worker, "lhm_supported");
            let client = NativeSessionClient::from_authenticated_transport(transport)
                .await
                .unwrap();
            let (samples, rx) = mpsc::channel(4);
            let (stop, stop_rx) = oneshot::channel();
            let sampler = feed(samples, epoch, 900.0, false);
            let runtime = tokio::spawn(run_with_limits(
                client,
                plan.mode,
                plan.limits,
                SamplerFrame {
                    temperature_c: 30.0,
                    temperature_at_ms: now(epoch),
                    rpm: 900.0,
                    rpm_at_ms: now(epoch),
                },
                rx,
                stop_rx,
                epoch,
            ));
            runs.push((peer, runtime, stop, sampler));
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
        let mut duties = Vec::new();
        for (peer, runtime, stop, sampler) in runs {
            stop.send(()).unwrap();
            let report = tokio::time::timeout(Duration::from_secs(2), runtime)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(report.fault.is_none(), "{:?}", report.fault);
            let seen = peer.await.unwrap();
            assert_eq!(seen.last().unwrap()["action"], "close", "channel returned");
            duties.push(
                seen.iter()
                    .filter_map(|r| r["duty_percent"].as_u64())
                    .collect::<Vec<_>>(),
            );
            sampler.abort();
        }
        publisher.abort();
        assert!(!duties[0].is_empty() && duties[0].iter().all(|d| *d >= 40));
        assert!(
            duties[1].contains(&65),
            "composite drove the duty: {:?}",
            duties[1]
        );
    }

    #[tokio::test]
    async fn composite_mode_follows_the_feed_and_fails_safe() {
        let (transport, worker) = tokio::io::duplex(4096);
        let peer = tiered_peer(worker, "lhm_supported");
        let client = NativeSessionClient::from_authenticated_transport(transport)
            .await
            .unwrap();
        let epoch = Instant::now();
        let (samples, rx) = mpsc::channel(4);
        let (stop, stop_rx) = oneshot::channel();
        let sampler = feed(samples, epoch, 900.0, false);
        let output = "runtime_composite_test";
        let ok = freergb_control::curves::CurveOutput {
            percent: 55.0,
            failsafe: false,
        };
        let composite = crate::composite_feed::CompositeFeed::default();
        composite.publish([(output.to_string(), ok)].into(), Instant::now());
        let runtime = tokio::spawn(run_with_limits(
            client,
            ControlMode::Composite {
                output: output.into(),
                critical_c: 80.0,
                feed: composite.clone(),
            },
            RuntimeLimits::default(),
            SamplerFrame {
                temperature_c: 30.0,
                temperature_at_ms: 0,
                rpm: 900.0,
                rpm_at_ms: 0,
            },
            rx,
            stop_rx,
            epoch,
        ));
        tokio::time::sleep(Duration::from_millis(600)).await;
        let failed = freergb_control::curves::CurveOutput {
            percent: 100.0,
            failsafe: true,
        };
        composite.publish([(output.to_string(), failed)].into(), Instant::now());
        tokio::time::sleep(Duration::from_millis(600)).await;
        stop.send(()).unwrap();
        let report = tokio::time::timeout(Duration::from_secs(2), runtime)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(report.fault.is_none(), "{:?}", report.fault);
        let duties: Vec<u64> = peer
            .await
            .unwrap()
            .iter()
            .filter_map(|r| r["duty_percent"].as_u64())
            .collect();
        let first_full = duties.iter().position(|d| *d == 100).unwrap();
        assert!(first_full > 0 && duties[..first_full].iter().all(|d| *d == 55));
        sampler.abort();
    }

    #[tokio::test]
    async fn shaped_closed_loop_rpm_corrects_skips_resonance_and_caps() {
        use freergb_control::calibration::TestedDuty;
        use freergb_control::shaping::{AvoidRange, AvoidUnit};
        let (transport, worker) = tokio::io::duplex(4096);
        let peer = tiered_peer(worker, "lhm_supported");
        let client = NativeSessionClient::from_authenticated_transport(transport)
            .await
            .unwrap();
        let epoch = Instant::now();
        let (samples, rx) = mpsc::channel(4);
        let (stop, stop_rx) = oneshot::channel();
        // The fan stays at 900 RPM: the PI term keeps pushing above open loop.
        let sampler = feed(samples, epoch, 900.0, false);
        let map = RpmMap::from_tested(&[
            TestedDuty {
                duty_percent: 100,
                rpm: 1800.0,
            },
            TestedDuty {
                duty_percent: 40,
                rpm: 900.0,
            },
        ])
        .unwrap();
        let runtime = tokio::spawn(run_with_limits(
            client,
            ControlMode::Shaped {
                inner: Box::new(ControlMode::Rpm {
                    map,
                    target_rpm: 1300.0,
                    gains: PiGains::default(),
                    critical_c: 80.0,
                }),
                shaping: OutputShaping {
                    max_percent: 80.0,
                    avoid: vec![AvoidRange {
                        from: 60.0,
                        to: 70.0,
                        unit: AvoidUnit::Percent,
                    }],
                    ..OutputShaping::default()
                },
            },
            RuntimeLimits::default(),
            SamplerFrame {
                temperature_c: 30.0,
                temperature_at_ms: 0,
                rpm: 900.0,
                rpm_at_ms: 0,
            },
            rx,
            stop_rx,
            epoch,
        ));
        tokio::time::sleep(Duration::from_millis(1500)).await;
        stop.send(()).unwrap();
        let report = tokio::time::timeout(Duration::from_secs(2), runtime)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(report.fault.is_none(), "{:?}", report.fault);
        let duties: Vec<u64> = peer
            .await
            .unwrap()
            .iter()
            .filter_map(|r| r["duty_percent"].as_u64())
            .collect();
        assert!(duties.len() > 3, "{duties:?}");
        assert!(
            duties.iter().all(|d| (70..=80).contains(d)),
            "open loop 66.7% snaps to 70, PI raises it, max caps at 80: {duties:?}"
        );
        assert!(duties.iter().any(|d| *d > 70), "{duties:?}");
        sampler.abort();
    }

    #[tokio::test]
    async fn runtime_limits_must_match_the_admitted_tier() {
        for (tier, limits) in [
            (
                Some("lhm_supported"),
                RuntimeLimits {
                    floor_percent: 0,
                    allow_stop: true,
                },
            ),
            (
                None,
                RuntimeLimits {
                    floor_percent: 0,
                    allow_stop: true,
                },
            ),
            (
                Some("unrestricted"),
                RuntimeLimits {
                    floor_percent: 20,
                    allow_stop: false,
                },
            ),
        ] {
            let (transport, mut worker) = tokio::io::duplex(4096);
            let mut reply = json!({"version":1,"session":"a".repeat(32),"ticket":"b".repeat(32),
                "sequence":0,"state":"active","reason":null,"duty_percent":100,"ticket_ms":250,
                "lease_ms":1000,"owned":true,"physical_write_verified":false});
            if let Some(tier) = tier {
                reply["tier"] = json!(tier);
            }
            worker
                .write_all(format!("{reply}\n").as_bytes())
                .await
                .unwrap();
            let client = NativeSessionClient::from_authenticated_transport(transport)
                .await
                .unwrap();
            let (_samples, rx) = mpsc::channel(1);
            let (_stop, stop_rx) = oneshot::channel();
            let result = run_with_limits(
                client,
                ControlMode::Calibration { critical_c: 80.0 },
                limits,
                SamplerFrame {
                    temperature_c: 30.0,
                    temperature_at_ms: 0,
                    rpm: 900.0,
                    rpm_at_ms: 0,
                },
                rx,
                stop_rx,
                Instant::now(),
            )
            .await;
            assert!(result.is_err(), "{tier:?} {limits:?}");
        }
    }

    async fn healthy_commands_check() {
        for stalled in [false, true] {
            let (transport, worker) = tokio::io::duplex(4096);
            let peer = tokio::spawn(async move {
                let mut worker = BufReader::new(worker);
                let mut reply = json!({"version":1,"session":"a".repeat(32),"ticket":"b".repeat(32),
                    "sequence":0,"state":"active","reason":null,"duty_percent":100,"ticket_ms":250,
                    "lease_ms":1000,"owned":true,"physical_write_verified":false});
                worker
                    .get_mut()
                    .write_all(format!("{reply}\n").as_bytes())
                    .await
                    .unwrap();
                let mut commands = 0;
                loop {
                    let mut line = String::new();
                    if worker.read_line(&mut line).await.unwrap() == 0 {
                        break;
                    }
                    let request: serde_json::Value = serde_json::from_str(&line).unwrap();
                    commands += 1;
                    reply["sequence"] = request["sequence"].clone();
                    reply["duty_percent"] = request["duty_percent"].clone();
                    // New ticket on every successful command; command path stays healthy.
                    reply["ticket"] = json!(format!("{commands:032x}"));
                    worker
                        .get_mut()
                        .write_all(format!("{reply}\n").as_bytes())
                        .await
                        .unwrap();
                }
                commands
            });
            let client = NativeSessionClient::from_authenticated_transport(transport)
                .await
                .unwrap();
            let epoch = Instant::now();
            let (samples, rx) = mpsc::channel(4);
            let (_stop, stop_rx) = oneshot::channel();
            let mut curve = FanCurve {
                minimum_percent: 40.0,
                ..FanCurve::default()
            };
            for point in &mut curve.points {
                point.duty_percent = point.duty_percent.max(40.0);
            }
            let runtime = tokio::spawn(run(
                client,
                ControlMode::Curve(curve),
                SamplerFrame {
                    temperature_c: 40.0,
                    temperature_at_ms: 0,
                    rpm: 900.0,
                    rpm_at_ms: 0,
                },
                rx,
                stop_rx,
                epoch,
            ));
            let sampler = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    let at = now(epoch);
                    if samples
                        .send(SamplerFrame {
                            temperature_c: 40.0,
                            // Repeated cached temperature cannot be refreshed by fresh RPM.
                            temperature_at_ms: if stalled { at } else { 0 },
                            rpm: if stalled { 0.0 } else { 900.0 },
                            rpm_at_ms: at,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });
            let error = tokio::time::timeout(Duration::from_secs(4), runtime)
                .await
                .unwrap()
                .unwrap()
                .err()
                .unwrap();
            assert!(
                error.contains(if stalled { "Stalled" } else { "Stale" }),
                "{error}"
            );
            assert!(
                peer.await.unwrap() >= 3,
                "commands must remain healthy during telemetry failure"
            );
            sampler.await.unwrap();
        }
    }
    #[tokio::test]
    async fn sensor_emergency_does_not_wait_for_stuck_native_receipt() {
        let (transport, mut worker) = tokio::io::duplex(4096);
        worker
            .write_all(
                format!(
                    "{}\n",
                    json!({"version":1,"session":"a".repeat(32),"ticket":"b".repeat(32),
            "sequence":0,"state":"active","reason":null,"duty_percent":100,"ticket_ms":250,
            "lease_ms":1000,"owned":true,"physical_write_verified":false})
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let client = NativeSessionClient::from_authenticated_transport(transport)
            .await
            .unwrap();
        let (sent, seen) = oneshot::channel();
        let native = tokio::spawn(async move {
            let mut worker = BufReader::new(worker);
            let mut line = String::new();
            worker.read_line(&mut line).await.unwrap();
            sent.send(()).unwrap();
            // No receipt: supervisor must still process the critical sample.
            line.clear();
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(2), worker.read_line(&mut line))
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );
        });
        let epoch = Instant::now();
        let initial = SamplerFrame {
            temperature_c: 40.0,
            temperature_at_ms: 0,
            rpm: 900.0,
            rpm_at_ms: 0,
        };
        let (samples, rx) = mpsc::channel(4);
        let (_stop, stop_rx) = oneshot::channel();
        let runtime = tokio::spawn(run(
            client,
            ControlMode::Calibration { critical_c: 80.0 },
            initial,
            rx,
            stop_rx,
            epoch,
        ));
        seen.await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let at = now(epoch);
        samples
            .send(SamplerFrame {
                temperature_c: 90.0,
                temperature_at_ms: at,
                rpm_at_ms: at,
                ..initial
            })
            .await
            .unwrap();
        let error = tokio::time::timeout(Duration::from_secs(1), runtime)
            .await
            .unwrap()
            .unwrap()
            .err()
            .unwrap();
        assert!(error.contains("Critical"), "{error}");
        native.await.unwrap();
    }
}
