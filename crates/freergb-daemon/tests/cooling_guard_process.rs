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
    let mut command = Command::new(env!("CARGO_BIN_EXE_freergb-cooling-guard-fixture"));
    if producer {
        command.arg("--producer");
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
