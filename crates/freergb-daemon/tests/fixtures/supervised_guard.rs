//! Process acceptance only: local synthetic samples and a memory-only duty sink.
use freergb_control::cooling_safety::{DutyLease, GuardState, SupervisedControl};
use freergb_control::maintenance::{MaintenanceGate, MaintenanceState, ReleaseEvidence};
use freergb_control::telemetry_guard::{Reading, TelemetryGuard};
use serde::Deserialize;
use serde_json::json;
use std::io::{BufRead, Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Message {
    Temperature {
        value: f32,
    },
    Rpm {
        value: f32,
    },
    Maintenance,
    Release {
        native_released: bool,
        firmware_return_verified: bool,
        journal_cleared: bool,
    },
}

fn emit(state: &str, duty: u8, detail: String) {
    println!(
        "{}",
        json!({"state":state,"duty_percent":duty,"detail":detail,"simulated":true,"physical_write_verified":false})
    );
    std::io::stdout().flush().unwrap();
}

pub fn run() {
    let (tx, rx) = mpsc::sync_channel(32);
    std::thread::spawn(move || {
        let mut input = std::io::BufReader::new(std::io::stdin());
        loop {
            let mut line = Vec::new();
            let message = match (&mut input).take(513).read_until(b'\n', &mut line) {
                Ok(n) if n > 0 && n <= 512 && line.last() == Some(&b'\n') => {
                    serde_json::from_slice::<Message>(&line).ok()
                }
                _ => None,
            };
            let end = message.is_none();
            if tx.try_send(message).is_err() || end {
                break;
            }
        }
    });
    let began = Instant::now();
    let mut telemetry =
        TelemetryGuard::new("fixture/cpu".into(), "fixture/fan".into(), 80.0).unwrap();
    for (source, value) in [("fixture/cpu", 40.0), ("fixture/fan", 900.0)] {
        telemetry
            .observe(
                Reading {
                    source,
                    acquired_ms: 0,
                    value,
                },
                0,
            )
            .unwrap();
    }
    let mut control = SupervisedControl::new(telemetry, 0, 1000, 40).unwrap();
    let mut maintenance = MaintenanceGate::new(true, 0);
    let token = maintenance.admit(0).unwrap();
    let mut sequence = 1;
    control
        .accept(
            DutyLease {
                sequence,
                issued_at_ms: 0,
                duty_percent: 40,
            },
            0,
        )
        .unwrap();
    emit("armed", 40, String::new());
    loop {
        let incoming = rx.recv_timeout(Duration::from_millis(20));
        // Synthetic startup readings occupy tick zero; the first input can
        // arrive within the same wall-clock millisecond as the ready message.
        let now = began.elapsed().as_millis() as u64 + 1;
        maintenance.poll(now);
        match incoming {
            Ok(Some(Message::Temperature { value })) => {
                let _ = control.observe(
                    Reading {
                        source: "fixture/cpu",
                        acquired_ms: now,
                        value,
                    },
                    now,
                );
            }
            Ok(Some(Message::Rpm { value })) => {
                let _ = control.observe(
                    Reading {
                        source: "fixture/fan",
                        acquired_ms: now,
                        value,
                    },
                    now,
                );
            }
            Ok(Some(Message::Maintenance)) => {
                control.stop();
                if maintenance.begin(now, 1000).is_err() {
                    emit("blocked", 100, "maintenance request rejected".into());
                    return;
                }
                assert!(maintenance.admit(now).is_err());
                emit("draining", 100, "admission_closed".into());
            }
            Ok(Some(Message::Release {
                native_released,
                firmware_return_verified,
                journal_cleared,
            })) => {
                if maintenance.state() != MaintenanceState::Draining {
                    emit("blocked", 100, "unexpected release".into());
                    return;
                }
                let _ = maintenance.released(
                    token,
                    ReleaseEvidence {
                        native_released,
                        firmware_return_verified,
                        journal_cleared,
                    },
                    now,
                );
            }
            Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                control.stop();
                maintenance.worker_lost(token);
                emit("blocked", 100, "input_lost".into());
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        match maintenance.state() {
            MaintenanceState::Ready => {
                emit(
                    "ready_for_maintenance",
                    100,
                    "synthetic evidence only".into(),
                );
                return;
            }
            MaintenanceState::Blocked => {
                emit("blocked", 100, "recovery_required".into());
                return;
            }
            MaintenanceState::Draining => {
                continue;
            }
            MaintenanceState::Running => {}
        }
        // Synthetic controller remains healthy while sensor input can stall.
        sequence += 1;
        let _ = control.accept(
            DutyLease {
                sequence,
                issued_at_ms: now,
                duty_percent: 40,
            },
            now,
        );
        if control.lease_state() != GuardState::Armed {
            emit(
                "emergency",
                control.poll(now),
                format!("{:?}", control.telemetry_fault()),
            );
            return;
        }
    }
}
