//! Rust cooling execution over an authenticated native session. No listener,
//! process elevation or automatic hardware activation is performed here.
use crate::native_session::{NativeObservation, NativeSessionClient, Receipt, SessionState};
use freergb_control::calibration::{
    Calibration, CalibrationPhase, CalibrationResult, CalibrationSample,
};
use freergb_control::channels::ChannelRole;
use freergb_control::cooling_safety::{DutyLease, GuardState, SupervisedControl};
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
    Calibration { critical_c: f32 },
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct RuntimeReport {
    pub calibration: Option<CalibrationResult>,
    pub native_closed: bool,
    /// Requires authenticated register-return AND durable journal proof. A closed
    /// callback from older workers alone cannot authorize maintenance.
    pub maintenance_authorized: bool,
    pub minimum_applied_percent: u8,
    pub initial_rpm: f32,
    pub last_rpm: f32,
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
    mut client: NativeSessionClient<T>,
    mode: ControlMode,
    initial: SamplerFrame,
    mut samples: mpsc::Receiver<SamplerFrame>,
    mut stop: oneshot::Receiver<()>,
    epoch: Instant,
) -> Result<RuntimeReport, String> {
    let at = now(epoch);
    let critical = match &mode {
        ControlMode::Curve(curve) => {
            curve.validate()?;
            if curve.minimum_percent < 40.0 {
                return Err(
                    "Native fan pilot requires a curve floor of at least 40 percent".into(),
                );
            }
            curve.critical_c
        }
        ControlMode::Calibration { critical_c } => *critical_c,
    };
    let mut telemetry = TelemetryGuard::new("temperature".into(), "rpm".into(), critical)?;
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
    if initial.rpm < 100.0 {
        return Err("A running, explicitly selected fan is required before control".into());
    }
    let mut control = SupervisedControl::new(telemetry, at, 1000, 40)?;
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
    let mut calibration = match mode {
        ControlMode::Calibration { critical_c } => {
            Some(Calibration::new(ChannelRole::Fan, critical_c, at)?)
        }
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
    let mut ticker = tokio::time::interval(Duration::from_millis(25));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            incoming = samples.recv(), if !closing && !native_sampled => {
                let incoming = incoming.ok_or("Trusted sampler disconnected; preserve recovery intent")?;
                observe(&mut control,incoming,Some(frame),now(epoch))?;
                frame = incoming;
            },
            incoming = native_receiver.recv(), if !closing && native_sampled => {
                let incoming = SamplerFrame::from_native(incoming.ok_or("Native sampler transport stopped")?,epoch)?;
                observe(&mut control,incoming,Some(frame),now(epoch))?;
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
                if operation.closing {
                    if receipt.state != SessionState::Closed || receipt.owned { return Err("Native return failed; recovery required".into()); }
                    return Ok(RuntimeReport { calibration: result,native_closed: true,maintenance_authorized: receipt.firmware_return_verified && receipt.journal_cleared,
                        minimum_applied_percent, initial_rpm: initial.rpm,last_rpm: frame.rpm });
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
            return Err(format!(
                "Cooling supervision stopped: {:?}, {:?}; preserve recovery intent",
                control.lease_state(),
                control.telemetry_fault()
            ));
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
        } else if let ControlMode::Curve(curve) = &mode {
            curve_controller
                .update(
                    curve,
                    Some(frame.temperature_c),
                    at.saturating_sub(frame.temperature_at_ms),
                )
                .ceil()
                .clamp(40.0, 100.0) as u8
        } else {
            return Err("Missing cooling mode".into());
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
