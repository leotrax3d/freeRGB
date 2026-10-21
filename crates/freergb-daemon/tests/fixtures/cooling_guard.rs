//! Synthetic sink only. Feature-gated, never shipped or able to access hardware.
use freergb_control::cooling_safety::{DutyLease, GuardState, LeaseGuard};
use serde_json::json;
use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
mod supervised_guard;

fn emit(state: impl serde::Serialize, duty: u8) {
    println!(
        "{}",
        json!({"state":state,"duty_percent":duty,"simulated":true})
    );
    std::io::stdout().flush().unwrap();
}
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--supervised") {
        supervised_guard::run();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--producer") {
        let mut guard = Command::new(std::env::current_exe().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut input = guard.stdin.take().unwrap();
        writeln!(
            input,
            "{{\"sequence\":1,\"issued_at_ms\":0,\"duty_percent\":40}}"
        )
        .unwrap();
        input.flush().unwrap();
        // Intentionally silent/alive producer. Tests also kill it to exercise EOF.
        std::thread::sleep(Duration::from_secs(10));
        drop(input);
        let _ = guard.wait();
        return;
    }
    let began = Instant::now();
    let mut guard = LeaseGuard::new(0, 1000, 40).unwrap();
    let (tx, rx) = mpsc::sync_channel(32);
    std::thread::spawn(move || {
        let mut input = std::io::BufReader::new(std::io::stdin());
        loop {
            let mut line = Vec::new();
            let read = (&mut input).take(513).read_until(b'\n', &mut line);
            let command = match read {
                Ok(n) if n > 0 && n <= 512 && line.last() == Some(&b'\n') => {
                    serde_json::from_slice::<DutyLease>(&line).ok()
                }
                _ => None,
            };
            let end = command.is_none();
            if tx.try_send(command).is_err() || end {
                break;
            }
        }
    });
    emit("ready", 100);
    loop {
        let incoming = rx.recv_timeout(Duration::from_millis(20));
        let now = began.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        guard.poll(now);
        match incoming {
            Ok(Some(command)) => {
                if guard.accept(command, now).is_err() {
                    guard.stop();
                }
                emit(guard.state(), guard.poll(now));
            }
            Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => guard.stop(),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if guard.state() != GuardState::Armed {
            emit(guard.state(), 100);
            break;
        }
    }
}
