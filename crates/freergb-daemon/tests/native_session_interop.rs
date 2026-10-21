//! Explicit opt-in local test binary + current-user pipe; no SYSTEM/driver/hardware.
//! Run with scripts/test-mainboard-interop.ps1. Never authenticates production peers.
#[cfg(windows)]
mod windows {
    use freergb_control::FanCurve;
    use freergb_daemon::cooling_runtime::{self, ControlMode, SamplerFrame};
    use freergb_daemon::native_session::{NativeSessionClient, SessionState};
    use serde_json::{json, Value};
    use std::process::Stdio;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;
    use tokio::process::{Child, ChildStdout, Command};
    use tokio::time::timeout;

    async fn worker() -> (Child, BufReader<ChildStdout>, String) {
        static NEXT_PIPE: AtomicU32 = AtomicU32::new(0);
        let binary = std::env::var_os("FREERGB_NATIVE_SESSION_FIXTURE").expect(
            "Use scripts/test-mainboard-interop.ps1 to build and select the test-only helper",
        );
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let serial = NEXT_PIPE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "freergb-native-rust-fixture-{:08x}{stamp:016x}{serial:08x}",
            std::process::id()
        );
        let mut child = Command::new(binary)
            .args(["--session-child", &name])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        timeout(Duration::from_secs(5), output.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(line.trim(), "ready");
        (child, output, format!(r"\\.\pipe\{name}"))
    }

    async fn result(child: &mut Child, output: &mut BufReader<ChildStdout>) -> Value {
        let mut line = String::new();
        timeout(Duration::from_secs(5), output.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert!(timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success());
        assert_eq!(value["simulated"], true);
        assert_eq!(value["physical_write_verified"], false);
        value
    }

    #[tokio::test]
    #[ignore = "Requires explicit test-only C# helper; run scripts/test-mainboard-interop.ps1"]
    async fn rust_set_close_reaches_real_csharp_session_transport() {
        let (mut child, mut output, name) = worker().await;
        // Current-user synthetic pipe only. Production must additionally verify
        // SYSTEM ownership, service registration and exact worker lifetime.
        let pipe = ClientOptions::new().open(name).unwrap();
        let mut client = NativeSessionClient::from_authenticated_transport(pipe)
            .await
            .unwrap();
        let receipt = client.set_duty(40).await.unwrap();
        assert_eq!(receipt.state, SessionState::Active);
        assert!(receipt.owned);
        assert_eq!(receipt.duty_percent, 40);
        let receipt = client.close().await.unwrap();
        assert_eq!(receipt.state, SessionState::Closed);
        assert!(!receipt.owned);
        let value = result(&mut child, &mut output).await;
        assert_eq!(value["writes"], json!([100, 40, 100, -1]));
        assert_eq!(value["state"], "closed");
    }

    #[tokio::test]
    #[ignore = "Requires explicit test-only C# helper; run scripts/test-mainboard-interop.ps1"]
    async fn dropped_rust_client_leaves_worker_in_owned_emergency() {
        let (mut child, mut output, name) = worker().await;
        let pipe = ClientOptions::new().open(name).unwrap();
        let mut client = NativeSessionClient::from_authenticated_transport(pipe)
            .await
            .unwrap();
        client.set_duty(40).await.unwrap();
        drop(client);
        let value = result(&mut child, &mut output).await;
        assert_eq!(value["writes"], json!([100, 40, 100]));
        assert_eq!(value["state"], "emergency");
        assert_eq!(value["reason"], "peer_disconnected");
        assert_eq!(value["owned"], true);
    }

    #[tokio::test]
    #[ignore = "Requires explicit test-only C# helper; run scripts/test-mainboard-interop.ps1"]
    async fn silent_rust_client_cannot_extend_worker_lease() {
        let (mut child, mut output, name) = worker().await;
        let pipe = ClientOptions::new().open(name).unwrap();
        let mut client = NativeSessionClient::from_authenticated_transport(pipe)
            .await
            .unwrap();
        client.set_duty(40).await.unwrap();
        // Keep the real pipe connected beyond the worker's independent lease.
        tokio::time::sleep(Duration::from_millis(1250)).await;
        drop(client);
        let value = result(&mut child, &mut output).await;
        assert_eq!(value["writes"], json!([100, 40, 100]));
        assert_eq!(value["reason"], "lease_expired");
        assert_eq!(value["state"], "emergency");
        assert_eq!(value["owned"], true);
    }

    #[tokio::test]
    #[ignore = "Requires explicit test-only C# helper; run scripts/test-mainboard-interop.ps1"]
    async fn rust_curve_runtime_drives_and_returns_csharp_session() {
        let (mut child, mut output, name) = worker().await;
        let pipe = ClientOptions::new().open(name).unwrap();
        let client = NativeSessionClient::from_authenticated_transport(pipe)
            .await
            .unwrap();
        let epoch = std::time::Instant::now();
        let initial = SamplerFrame {
            temperature_c: 40.0,
            temperature_at_ms: 0,
            rpm: 900.0,
            rpm_at_ms: 0,
        };
        let (samples, rx) = tokio::sync::mpsc::channel(4);
        let (stop, stop_rx) = tokio::sync::oneshot::channel();
        let mut curve = FanCurve {
            minimum_percent: 40.0,
            ..FanCurve::default()
        };
        curve.points[0].duty_percent = 40.0;
        let task = tokio::spawn(cooling_runtime::run(
            client,
            ControlMode::Curve(curve),
            initial,
            rx,
            stop_rx,
            epoch,
        ));
        for _ in 0..3 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let at = epoch.elapsed().as_millis() as u64;
            samples
                .send(SamplerFrame {
                    temperature_at_ms: at,
                    rpm_at_ms: at,
                    ..initial
                })
                .await
                .unwrap();
        }
        stop.send(()).unwrap();
        let report = timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(report.native_closed);
        assert!(!report.maintenance_authorized);
        let value = result(&mut child, &mut output).await;
        let writes = value["writes"].as_array().unwrap();
        assert_eq!(writes.first(), Some(&json!(100)));
        assert!(writes
            .iter()
            .any(|duty| duty.as_u64().is_some_and(|duty| (40..100).contains(&duty))));
        assert_eq!(&writes[writes.len() - 2..], &[json!(100), json!(-1)]);
    }

    #[tokio::test]
    #[ignore = "Requires explicit test-only C# helper; run scripts/test-mainboard-interop.ps1"]
    async fn sampler_loss_aborts_rust_runtime_and_retains_worker_recovery_ownership() {
        let (mut child, mut output, name) = worker().await;
        let pipe = ClientOptions::new().open(name).unwrap();
        let client = NativeSessionClient::from_authenticated_transport(pipe)
            .await
            .unwrap();
        let (samples, rx) = tokio::sync::mpsc::channel(4);
        let (_stop, stop_rx) = tokio::sync::oneshot::channel();
        let initial = SamplerFrame {
            temperature_c: 40.0,
            temperature_at_ms: 0,
            rpm: 900.0,
            rpm_at_ms: 0,
        };
        let epoch = std::time::Instant::now();
        let task = tokio::spawn(cooling_runtime::run(
            client,
            ControlMode::Calibration { critical_c: 80.0 },
            initial,
            rx,
            stop_rx,
            epoch,
        ));
        tokio::time::sleep(Duration::from_millis(100)).await;
        drop(samples);
        assert!(timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .is_err());
        let value = result(&mut child, &mut output).await;
        assert_eq!(value["state"], "emergency");
        assert_eq!(value["owned"], true);
        assert_eq!(
            value["writes"].as_array().unwrap().last(),
            Some(&json!(100))
        );
    }

    #[tokio::test]
    #[ignore = "Requires explicit test-only C# helper; run scripts/test-mainboard-interop.ps1"]
    async fn full_rust_calibration_rejects_constant_tachometer_response() {
        let (mut child, mut output, name) = worker().await;
        let pipe = ClientOptions::new().open(name).unwrap();
        let client = NativeSessionClient::from_authenticated_transport(pipe)
            .await
            .unwrap();
        let epoch = std::time::Instant::now();
        let initial = SamplerFrame {
            temperature_c: 40.0,
            temperature_at_ms: 0,
            rpm: 900.0,
            rpm_at_ms: 0,
        };
        let (samples, rx) = tokio::sync::mpsc::channel(4);
        let (_stop, stop_rx) = tokio::sync::oneshot::channel();
        let sampler = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                let at = epoch.elapsed().as_millis() as u64;
                if samples
                    .send(SamplerFrame {
                        temperature_at_ms: at,
                        rpm_at_ms: at,
                        ..initial
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        let report = timeout(
            Duration::from_secs(25),
            cooling_runtime::run(
                client,
                ControlMode::Calibration { critical_c: 80.0 },
                initial,
                rx,
                stop_rx,
                epoch,
            ),
        )
        .await
        .unwrap()
        .unwrap();
        sampler.abort();
        assert!(report.native_closed);
        assert!(!report.maintenance_authorized);
        let calibration = report.calibration.unwrap();
        assert!(!calibration.response_observed);
        assert!(!calibration.hardware_verified);
        assert!(calibration.suggested_floor_percent.is_none());
        let value = result(&mut child, &mut output).await;
        let writes = value["writes"].as_array().unwrap();
        for duty in [100, 80, 60, 40] {
            assert!(writes.contains(&json!(duty)));
        }
        assert_eq!(&writes[writes.len() - 2..], &[json!(100), json!(-1)]);
    }
}
