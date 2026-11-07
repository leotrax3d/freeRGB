//! Owns cooling execution lifetimes and drains them before maintenance.
//! The Windows service transport must supply an authenticated factory and trusted
//! sampler. This module does not authorize hardware or expose a public listener.
use crate::cooling_runtime::{self, ControlMode, RuntimeLimits, RuntimeReport, SamplerFrame};
use crate::native_session::{NativeSessionClient, NATIVE_STARTUP_BUDGET};
use freergb_control::maintenance::{
    MaintenanceGate, MaintenanceState, ReleaseEvidence, SessionToken,
};
use std::future::Future;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

struct Execution {
    token: SessionToken,
    stop: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<RuntimeReport, String>>,
}

impl Drop for Execution {
    fn drop(&mut self) {
        // Aborting drops the native client. Never interpret EOF as release proof.
        self.task.abort();
    }
}

pub struct CoolingHost {
    gate: MaintenanceGate,
    epoch: Instant,
    execution: Option<Execution>,
    last_report: Option<RuntimeReport>,
}

impl CoolingHost {
    /// `recovery_clear` must come from the authenticated protected backend.
    pub fn new(recovery_clear: bool, epoch: Instant) -> Self {
        Self {
            gate: MaintenanceGate::new(recovery_clear, 0),
            epoch,
            execution: None,
            last_report: None,
        }
    }

    fn now(&self) -> u64 {
        self.epoch.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }

    /// Reserve admission BEFORE invoking the native acquisition factory. The
    /// current durable native journal supports one channel, so a second is refused.
    /// Acquisition and runtime execute in an owned task: caller cancellation does
    /// not orphan an acquired channel. The factory must not detach child work.
    pub fn start<T, F>(
        &mut self,
        acquire: F,
        mode: ControlMode,
        initial: SamplerFrame,
        samples: mpsc::Receiver<SamplerFrame>,
    ) -> Result<(), String>
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        F: Future<Output = Result<NativeSessionClient<T>, String>> + Send + 'static,
    {
        if self.execution.is_some() || self.gate.unresolved_count() != 0 {
            return Err("Cooling channel already active or unresolved".into());
        }
        let token = self.gate.admit(self.now())?;
        let (stop, stopped) = oneshot::channel();
        let epoch = self.epoch;
        let task = tokio::spawn(async move {
            let client = tokio::time::timeout(Duration::from_millis(500), acquire)
                .await
                .map_err(|_| "Native acquisition timed out; recovery inspection required")??;
            cooling_runtime::run(client, mode, initial, samples, stopped, epoch).await
        });
        self.execution = Some(Execution {
            token,
            stop: Some(stop),
            task,
        });
        Ok(())
    }

    /// Production native sampler variant: initial values come only from the
    /// authenticated worker receipt, never from GUI/API arguments.
    pub fn start_native<T, F>(
        &mut self,
        acquire: F,
        mode: ControlMode,
        limits: RuntimeLimits,
    ) -> Result<(), String>
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        F: Future<Output = Result<NativeSessionClient<T>, String>> + Send + 'static,
    {
        if self.execution.is_some() || self.gate.unresolved_count() != 0 {
            return Err("Cooling channel already active or unresolved".into());
        }
        let token = self.gate.admit(self.now())?;
        let (stop, stopped) = oneshot::channel();
        let epoch = self.epoch;
        let task = tokio::spawn(async move {
            let client = tokio::time::timeout(NATIVE_STARTUP_BUDGET, acquire)
                .await
                .map_err(|_| "Protected native startup timed out; inspect recovery state")??;
            let observation = client
                .observation()
                .ok_or("Protected worker supplied no trusted telemetry")?;
            // Startup is bounded by acquisition above, before supervision exists.
            // A trusted cached temperature can predate this host: include that
            // acquisition in the runtime clock without resetting its real age.
            let epoch = epoch.min(observation.acquired);
            let initial = SamplerFrame::from_native(observation, epoch)?;
            let (_sender, samples) = mpsc::channel(1);
            cooling_runtime::run_with_limits(client, mode, limits, initial, samples, stopped, epoch)
                .await
        });
        self.execution = Some(Execution {
            token,
            stop: Some(stop),
            task,
        });
        Ok(())
    }

    fn record(
        &mut self,
        token: SessionToken,
        result: Result<Result<RuntimeReport, String>, tokio::task::JoinError>,
    ) -> Result<RuntimeReport, String> {
        match result {
            Ok(Ok(report)) => {
                self.gate.released(
                    token,
                    ReleaseEvidence {
                        native_released: report.native_closed,
                        firmware_return_verified: report.maintenance_authorized,
                        journal_cleared: report.maintenance_authorized,
                    },
                    self.now(),
                )?;
                self.last_report = Some(report.clone());
                Ok(report)
            }
            failure => {
                self.gate.worker_lost(token);
                Err(match failure {
                    Ok(Err(error)) => error,
                    Err(error) => {
                        format!("Cooling execution lost: {error}; preserve recovery state")
                    }
                    _ => unreachable!(),
                })
            }
        }
    }

    /// Collect completed calibration/curve execution. Errors close admission and
    /// preserve the unresolved token, including acquisition errors with uncertain IO.
    pub async fn collect_finished(&mut self) -> Result<Option<RuntimeReport>, String> {
        if !self
            .execution
            .as_ref()
            .is_some_and(|e| e.task.is_finished())
        {
            return Ok(None);
        }
        let mut execution = self.execution.take().expect("checked execution");
        let result = (&mut execution.task).await;
        self.record(execution.token, result).map(Some)
    }

    /// Close admission, request native return, then require register AND durable
    /// journal evidence. Ready is terminal; only a fresh validated host can reopen.
    /// The installer/service must additionally hold its existing OS maintenance lock.
    pub async fn prepare_maintenance(&mut self, timeout_ms: u64) -> Result<(), String> {
        self.gate.begin(self.now(), timeout_ms)?;
        if let Some(mut execution) = self.execution.take() {
            if let Some(stop) = execution.stop.take() {
                let _ = stop.send(());
            }
            match tokio::time::timeout(Duration::from_millis(timeout_ms), &mut execution.task).await
            {
                Ok(result) => {
                    self.record(execution.token, result)?;
                }
                Err(_) => {
                    self.gate.worker_lost(execution.token);
                    return Err(
                        "Cooling return timed out; keep binaries and recovery journal".into(),
                    );
                }
            }
        }
        if self.gate.poll(self.now()) != MaintenanceState::Ready {
            return Err("Cooling maintenance blocked; return evidence incomplete".into());
        }
        Ok(())
    }

    pub fn state(&mut self) -> MaintenanceState {
        self.gate.poll(self.now())
    }
    pub fn last_report(&self) -> Option<&RuntimeReport> {
        self.last_report.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    async fn native_start_then_freeze(startup: Duration, initial_age_ms: u64) {
        let (transport, worker) = tokio::io::duplex(4096);
        let (started, ready) = oneshot::channel();
        let peer = tokio::spawn(async move {
            ready.await.unwrap();
            let acquired = Instant::now();
            let mut worker = BufReader::new(worker);
            let mut reply = json!({"version":1,"session":"a".repeat(32),"ticket":"b".repeat(32),
                "sequence":0,"state":"active","reason":null,"duty_percent":100,"ticket_ms":250,
                "lease_ms":1000,"owned":true,"physical_write_verified":false,"tier":"lhm_supported",
                "telemetry":{"sequence":1,"temperature_c":45,"rpm":900,"age_ms":initial_age_ms,
                    "sources":{"temperature_sequence":1,"rpm_sequence":1,"temperature_age_ms":initial_age_ms,"rpm_age_ms":0}}});
            worker
                .get_mut()
                .write_all(format!("{reply}\n").as_bytes())
                .await
                .unwrap();
            let mut commands = 0;
            loop {
                let mut line = String::new();
                assert_ne!(
                    worker.read_line(&mut line).await.unwrap(),
                    0,
                    "lost session without firmware return"
                );
                let request: Value = serde_json::from_str(&line).unwrap();
                reply["sequence"] = request["sequence"].clone();
                commands += 1;
                reply["ticket"] = json!(format!("{commands:032x}"));
                let close = request["action"] == "close";
                if close {
                    assert!(
                        acquired.elapsed().as_millis() + u128::from(initial_age_ms) >= 6900,
                        "startup time was charged as sample age"
                    );
                    reply["state"] = json!("closed");
                    reply["owned"] = json!(false);
                    reply["duty_percent"] = json!(100);
                    reply["firmware_returned"] = json!(true);
                    reply["journal_cleared"] = json!(true);
                    reply["telemetry"] = Value::Null;
                } else {
                    reply["duty_percent"] = request["duty_percent"].clone();
                    reply["telemetry"]["sequence"] = json!(commands + 1);
                    reply["telemetry"]["sources"]["rpm_sequence"] = json!(commands + 1);
                    reply["telemetry"]["rpm"] = json!(900 + commands % 3);
                    // Only temperature stays frozen, even if a cached receipt
                    // understates its age. Fresh RPM and tickets cannot renew it.
                }
                worker
                    .get_mut()
                    .write_all(format!("{reply}\n").as_bytes())
                    .await
                    .unwrap();
                if close {
                    return commands;
                }
            }
        });
        let mut host = CoolingHost::new(true, Instant::now());
        host.start_native(
            async move {
                tokio::time::sleep(startup).await;
                started.send(()).unwrap();
                NativeSessionClient::from_authenticated_transport(transport).await
            },
            ControlMode::Curve(freergb_control::FanCurve {
                minimum_percent: 40.0,
                points: vec![
                    freergb_control::CurvePoint {
                        temperature_c: 30.0,
                        duty_percent: 50.0,
                    },
                    freergb_control::CurvePoint {
                        temperature_c: 70.0,
                        duty_percent: 100.0,
                    },
                ],
                ..Default::default()
            }),
            RuntimeLimits::default(),
        )
        .unwrap();
        if !startup.is_zero() {
            tokio::time::sleep(Duration::from_millis(1600)).await;
            assert!(
                host.collect_finished().await.unwrap().is_none(),
                "supervision ran before acquisition"
            );
        }
        let report = tokio::time::timeout(Duration::from_secs(18), async {
            loop {
                if let Some(report) = host.collect_finished().await.unwrap() {
                    break report;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        assert!(report.fault.as_deref().unwrap().contains("Stale"));
        assert!(report.native_closed && report.maintenance_authorized);
        assert_eq!(report.return_evidence, "firmware_callback");
        assert!(
            report.minimum_applied_percent < 100,
            "session never started control"
        );
        assert!(
            peer.await.unwrap() > 30,
            "healthy commands did not survive startup"
        );
    }

    #[tokio::test]
    async fn slow_native_start_is_not_stale_but_frozen_post_acquisition_sample_returns_firmware() {
        native_start_then_freeze(Duration::from_secs(8), 0).await;
    }

    #[tokio::test]
    async fn initial_shared_sample_can_predate_host_without_resetting_its_age() {
        native_start_then_freeze(Duration::ZERO, 500).await;
    }

    async fn exercise(proof: bool) {
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
            loop {
                let mut line = String::new();
                if worker.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                let request: Value = serde_json::from_str(&line).unwrap();
                reply["sequence"] = request["sequence"].clone();
                if request["action"] == "close" {
                    reply["state"] = json!("closed");
                    reply["owned"] = json!(false);
                    reply["firmware_return_verified"] = json!(proof);
                    reply["journal_cleared"] = json!(proof);
                    worker
                        .get_mut()
                        .write_all(format!("{reply}\n").as_bytes())
                        .await
                        .unwrap();
                    break;
                }
                reply["duty_percent"] = request["duty_percent"].clone();
                worker
                    .get_mut()
                    .write_all(format!("{reply}\n").as_bytes())
                    .await
                    .unwrap();
            }
        });
        let epoch = Instant::now();
        let mut host = CoolingHost::new(true, epoch);
        let (_samples, samples) = mpsc::channel(2);
        host.start(
            async move { NativeSessionClient::from_authenticated_transport(transport).await },
            ControlMode::Calibration { critical_c: 80.0 },
            SamplerFrame {
                temperature_c: 40.0,
                temperature_at_ms: 0,
                rpm: 900.0,
                rpm_at_ms: 0,
            },
            samples,
        )
        .unwrap();
        let result = host.prepare_maintenance(1000).await;
        assert_eq!(result.is_ok(), proof, "{result:?}");
        assert_eq!(
            host.state(),
            if proof {
                MaintenanceState::Ready
            } else {
                MaintenanceState::Blocked
            }
        );
        assert!(host.prepare_maintenance(1000).await.is_err());
        peer.await.unwrap();
    }

    #[tokio::test]
    async fn maintenance_drains_actual_runtime_and_requires_native_return_proof() {
        exercise(true).await;
        exercise(false).await;
    }

    // A slow first receipt models initialization BEFORE acquisition/ticket issue.
    // Once active, a 2-4 s command acknowledgement must still fail the 1 s lease.
    #[tokio::test]
    async fn slow_initial_receipts_succeed_but_missing_receipt_blocks_recovery() {
        for delay_ms in [Some(2000), Some(4000), None] {
            let (transport, worker) = tokio::io::duplex(4096);
            let (ready, startup) = oneshot::channel();
            let peer = tokio::spawn(async move {
                let mut worker = BufReader::new(worker);
                if let Some(delay_ms) = delay_ms {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    let _ = ready.send(()); // Relay emits ready only after initial worker receipt.
                    let mut reply = json!({"version":1,"session":"a".repeat(32),"ticket":"b".repeat(32),
                        "sequence":0,"state":"active","reason":null,"duty_percent":100,"ticket_ms":250,
                        "lease_ms":1000,"owned":true,"physical_write_verified":false,
                        "telemetry":{"sequence":1,"temperature_c":40.0,"rpm":900.0,"age_ms":0}});
                    worker
                        .get_mut()
                        .write_all(format!("{reply}\n").as_bytes())
                        .await
                        .unwrap();
                    let mut line = String::new();
                    worker.read_line(&mut line).await.unwrap();
                    let command: Value = serde_json::from_str(&line).unwrap();
                    assert_eq!(command["action"], "close");
                    reply["sequence"] = command["sequence"].clone();
                    reply["state"] = json!("closed");
                    reply["owned"] = json!(false);
                    reply["firmware_return_verified"] = json!(true);
                    reply["journal_cleared"] = json!(true);
                    worker
                        .get_mut()
                        .write_all(format!("{reply}\n").as_bytes())
                        .await
                        .unwrap();
                } else {
                    // Model a ready relay whose first receipt never arrives.
                    let _ = ready.send(());
                    let mut line = String::new();
                    // Timeout drops the original pipe; no retry/acquire or fake close.
                    assert_eq!(worker.read_line(&mut line).await.unwrap(), 0);
                }
            });
            let mut host = CoolingHost::new(true, Instant::now());
            host.start_native(
                async move {
                    startup.await.map_err(|_| "Fixture startup lost")?;
                    NativeSessionClient::from_authenticated_transport(transport).await
                },
                ControlMode::Calibration { critical_c: 80.0 },
                RuntimeLimits::default(),
            )
            .unwrap();
            let result = host.prepare_maintenance(8000).await;
            if delay_ms.is_some() {
                assert!(result.is_ok(), "{result:?}");
                assert_eq!(host.state(), MaintenanceState::Ready);
                assert!(host.last_report().unwrap().maintenance_authorized);
            } else {
                assert!(result.unwrap_err().contains("Native reply timed out"));
                assert_eq!(host.state(), MaintenanceState::Blocked);
                assert_eq!(host.gate.unresolved_count(), 1);
                assert!(host.last_report().is_none());
                assert!(host.prepare_maintenance(100).await.is_err());
            }
            peer.await.unwrap();
        }
    }

    #[tokio::test]
    async fn stuck_acquisition_blocks_replacement_and_cannot_orphan_task() {
        let epoch = Instant::now();
        let mut host = CoolingHost::new(true, epoch);
        let (_samples, samples) = mpsc::channel(2);
        host.start(
            std::future::pending::<Result<NativeSessionClient<tokio::io::DuplexStream>, String>>(),
            ControlMode::Calibration { critical_c: 80.0 },
            SamplerFrame {
                temperature_c: 40.0,
                temperature_at_ms: 0,
                rpm: 900.0,
                rpm_at_ms: 0,
            },
            samples,
        )
        .unwrap();
        assert!(host
            .prepare_maintenance(100)
            .await
            .unwrap_err()
            .contains("timed out"));
        assert_eq!(host.state(), MaintenanceState::Blocked);
        assert!(host.prepare_maintenance(100).await.is_err());
    }
}
