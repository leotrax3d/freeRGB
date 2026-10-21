//! Explicit opt-in local test binary + current-user pipe; no SYSTEM/driver/hardware.
//! Run with scripts/test-mainboard-interop.ps1. Never authenticates production peers.
#[cfg(windows)]
mod windows {
    use freergb_daemon::native_session::{NativeSessionClient, SessionState};
    use serde_json::{json, Value};
    use std::process::Stdio;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;
    use tokio::process::{Child, ChildStdout, Command};
    use tokio::time::timeout;

    async fn worker() -> (Child, BufReader<ChildStdout>, String) {
        let binary = std::env::var_os("FREERGB_NATIVE_SESSION_FIXTURE").expect(
            "Use scripts/test-mainboard-interop.ps1 to build and select the test-only helper",
        );
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            ^ (u128::from(std::process::id()) << 64);
        let name = format!("freergb-native-rust-fixture-{nonce:032x}");
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
}
