#![cfg(feature = "cooling-fixture")]
//! Real process boundaries, synthetic duty sink. No service, driver or hardware.
use serde_json::Value;
use std::io::{BufRead, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

struct Fixture(Child);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn spawn(producer: bool) -> (Fixture, Receiver<Value>) {
    spawn_mode(if producer { Some("--producer") } else { None })
}
fn spawn_mode(mode: Option<&str>) -> (Fixture, Receiver<Value>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_freergb-cooling-guard-fixture"));
    if let Some(mode) = mode {
        command.arg(mode);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(output).lines() {
            let Ok(line) = line else { break };
            let value: Value = serde_json::from_str(&line).unwrap();
            if tx.send(value).is_err() {
                break;
            }
        }
    });
    (Fixture(child), rx)
}
fn send(child: &mut Fixture, message: Value) {
    let input = child.0.stdin.as_mut().unwrap();
    writeln!(input, "{message}").unwrap();
    input.flush().unwrap();
}
#[test]
fn blocked_sensor_input_does_not_block_rust_supervision() {
    let (mut process, rx) = spawn_mode(Some("--supervised"));
    assert_eq!(read(&rx)["state"], "armed");
    // A partial frame leaves the input thread blocked while controller ticks continue.
    process
        .0
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"{\"type\":")
        .unwrap();
    let fallback = read(&rx);
    assert_eq!(fallback["state"], "emergency");
    assert_eq!(fallback["detail"], "Some(Stale)");
    assert_eq!(fallback["duty_percent"], 100);
    assert_eq!(fallback["physical_write_verified"], false);
}
#[test]
fn critical_heat_latches_in_the_supervisor_process() {
    let (mut process, rx) = spawn_mode(Some("--supervised"));
    assert_eq!(read(&rx)["state"], "armed");
    send(
        &mut process,
        serde_json::json!({"type":"temperature","value":80}),
    );
    let fallback = read(&rx);
    assert_eq!(fallback["state"], "emergency");
    assert_eq!(fallback["detail"], "Some(Critical)");
}
#[test]
fn fan_stall_is_detected_while_command_heartbeats_continue() {
    let (mut process, rx) = spawn_mode(Some("--supervised"));
    assert_eq!(read(&rx)["state"], "armed");
    send(&mut process, serde_json::json!({"type":"rpm","value":0}));
    let fallback = read(&rx);
    assert_eq!(fallback["state"], "emergency");
    assert_eq!(fallback["detail"], "Some(Stalled)");
    assert_eq!(fallback["duty_percent"], 100);
}
#[test]
fn input_loss_during_maintenance_cannot_authorize_replacement() {
    let (mut process, rx) = spawn_mode(Some("--supervised"));
    assert_eq!(read(&rx)["state"], "armed");
    send(&mut process, serde_json::json!({"type":"maintenance"}));
    assert_eq!(read(&rx)["state"], "draining");
    drop(process.0.stdin.take());
    let result = read(&rx);
    assert_eq!(result["state"], "blocked");
    assert_eq!(result["detail"], "input_lost");
    assert_eq!(result["physical_write_verified"], false);
}
#[test]
fn maintenance_requires_complete_return_and_does_not_wait_forever() {
    for outcome in ["complete", "partial", "timeout"] {
        let (mut process, rx) = spawn_mode(Some("--supervised"));
        assert_eq!(read(&rx)["state"], "armed");
        send(&mut process, serde_json::json!({"type":"maintenance"}));
        let draining = read(&rx);
        assert_eq!(draining["state"], "draining");
        assert_eq!(draining["detail"], "admission_closed");
        assert_eq!(draining["duty_percent"], 100);
        if outcome != "timeout" {
            send(
                &mut process,
                serde_json::json!({"type":"release","native_released":true,"firmware_return_verified":outcome=="complete","journal_cleared":true}),
            );
        }
        let result = read(&rx);
        assert_eq!(
            result["state"],
            if outcome == "complete" {
                "ready_for_maintenance"
            } else {
                "blocked"
            }
        );
        assert_eq!(result["simulated"], true);
        assert_eq!(result["physical_write_verified"], false);
    }
}
fn read(rx: &Receiver<Value>) -> Value {
    rx.recv_timeout(Duration::from_secs(5))
        .expect("guard did not respond independently")
}
fn armed(rx: &Receiver<Value>) {
    assert_eq!(read(rx)["state"], "ready");
    let active = read(rx);
    assert_eq!(active["state"], "armed");
    assert_eq!(active["duty_percent"], 40);
    assert_eq!(active["simulated"], true);
}
#[test]
fn producer_hang_cannot_keep_low_duty_alive() {
    let (mut producer, rx) = spawn(true);
    armed(&rx);
    let began = Instant::now();
    let fallback = read(&rx);
    assert_eq!(fallback["state"], "expired");
    assert_eq!(fallback["duty_percent"], 100);
    assert!(began.elapsed() < Duration::from_secs(4));
    assert!(
        producer.0.try_wait().unwrap().is_none(),
        "producer must remain hung while guard acts"
    );
}
#[test]
fn killed_producer_releases_pipe_and_guard_requests_full_speed() {
    let (mut producer, rx) = spawn(true);
    armed(&rx);
    producer.0.kill().unwrap();
    producer.0.wait().unwrap();
    let fallback = read(&rx);
    assert_eq!(fallback["state"], "stopped");
    assert_eq!(fallback["duty_percent"], 100);
}
#[test]
fn oversized_protocol_message_stops_without_waiting_for_newline() {
    let (mut guard, rx) = spawn(false);
    assert_eq!(read(&rx)["state"], "ready");
    guard
        .0
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&[b'x'; 513])
        .unwrap();
    let fallback = read(&rx);
    assert_eq!(fallback["state"], "stopped");
    assert_eq!(fallback["duty_percent"], 100);
}
