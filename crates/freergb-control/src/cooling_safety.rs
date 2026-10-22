//! Hardware-independent lease policy. A requested fallback is not proof of a write.
use crate::telemetry_guard::{Reading, TelemetryFault, TelemetryGuard};
use serde::{Deserialize, Serialize};

/// Combined supervisor policy. Holds no pipe, driver or GUI objects. Call poll
/// on a supervisor timer even when commands or sampler messages stop arriving.
pub struct SupervisedControl {
    lease: LeaseGuard,
    telemetry: TelemetryGuard,
    fault: Option<TelemetryFault>,
}

impl SupervisedControl {
    pub fn new(
        telemetry: TelemetryGuard,
        now_ms: u64,
        lease_ms: u64,
        floor: u8,
    ) -> Result<Self, String> {
        Self::with_limits(telemetry, now_ms, lease_ms, floor, false)
    }

    /// `allow_stop` is the unrestricted unlock: floors below 20% (fan stop)
    /// are accepted only with it.
    pub fn with_limits(
        mut telemetry: TelemetryGuard,
        now_ms: u64,
        lease_ms: u64,
        floor: u8,
        allow_stop: bool,
    ) -> Result<Self, String> {
        telemetry
            .check(now_ms)
            .map_err(|fault| format!("Unsafe initial telemetry: {fault:?}"))?;
        Ok(Self {
            lease: LeaseGuard::with_floor(now_ms, lease_ms, floor, allow_stop)?,
            telemetry,
            fault: None,
        })
    }

    pub fn telemetry_warning(&self) -> Option<TelemetryFault> {
        self.telemetry.warning()
    }

    pub fn observe(&mut self, reading: Reading<'_>, now_ms: u64) -> Result<(), TelemetryFault> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        if let Err(fault) = self.telemetry.observe(reading, now_ms) {
            self.fault = Some(fault);
            self.lease.stop();
            return Err(fault);
        }
        Ok(())
    }

    pub fn accept(&mut self, command: DutyLease, now_ms: u64) -> Result<(), String> {
        self.poll(now_ms);
        if self.fault.is_some() {
            return Err("Telemetry fault is latched".into());
        }
        self.lease.accept(command, now_ms)
    }

    /// Requested fallback only; caller must separately track native write and
    /// physical readback evidence. This function never claims verified speed.
    pub fn poll(&mut self, now_ms: u64) -> u8 {
        if self.fault.is_none() {
            if let Err(fault) = self.telemetry.check(now_ms) {
                self.fault = Some(fault);
                self.lease.stop();
            }
        }
        self.lease.poll(now_ms)
    }

    pub fn stop(&mut self) {
        self.lease.stop();
    }
    pub fn telemetry_fault(&self) -> Option<TelemetryFault> {
        self.fault
    }
    pub fn lease_state(&self) -> GuardState {
        self.lease.state()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardState {
    Armed,
    Expired,
    Stopped,
    ClockFault,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct DutyLease {
    pub sequence: u64,
    pub issued_at_ms: u64,
    pub duty_percent: u8,
}

pub struct LeaseGuard {
    state: GuardState,
    last_now: u64,
    deadline: u64,
    lease_ms: u64,
    sequence: u64,
    floor: u8,
    duty: u8,
}
impl LeaseGuard {
    pub fn new(now_ms: u64, lease_ms: u64, floor: u8) -> Result<Self, String> {
        Self::with_floor(now_ms, lease_ms, floor, false)
    }
    pub fn with_floor(
        now_ms: u64,
        lease_ms: u64,
        floor: u8,
        allow_stop: bool,
    ) -> Result<Self, String> {
        let lowest = if allow_stop { 0 } else { 20 };
        if !(500..=3000).contains(&lease_ms) || !(lowest..=100).contains(&floor) {
            return Err("Lease must be 500..3000 ms and floor 20..100 percent".into());
        }
        let deadline = now_ms.checked_add(lease_ms).ok_or("Lease clock overflow")?;
        Ok(Self {
            state: GuardState::Armed,
            last_now: now_ms,
            deadline,
            lease_ms,
            sequence: 0,
            floor,
            duty: 100,
        })
    }
    pub fn poll(&mut self, now_ms: u64) -> u8 {
        if now_ms < self.last_now {
            self.state = GuardState::ClockFault;
        }
        self.last_now = now_ms;
        if self.state == GuardState::Armed && now_ms >= self.deadline {
            self.state = GuardState::Expired;
        }
        if self.state == GuardState::Armed {
            self.duty
        } else {
            100
        }
    }
    pub fn accept(&mut self, command: DutyLease, now_ms: u64) -> Result<(), String> {
        self.poll(now_ms);
        if self.state != GuardState::Armed {
            return Err("Cooling lease is latched; start a new session explicitly".into());
        }
        if command.sequence <= self.sequence
            || command.issued_at_ms > now_ms
            || now_ms - command.issued_at_ms > 250
            || command.issued_at_ms < self.last_issued()
            || !(self.floor..=100).contains(&command.duty_percent)
        {
            return Err("Invalid, replayed, future or delayed duty command".into());
        }
        self.deadline = command
            .issued_at_ms
            .checked_add(self.lease_ms)
            .ok_or("Lease clock overflow")?;
        self.sequence = command.sequence;
        self.duty = command.duty_percent;
        Ok(())
    }
    fn last_issued(&self) -> u64 {
        self.deadline - self.lease_ms
    }
    pub fn stop(&mut self) {
        if self.state == GuardState::Armed {
            self.state = GuardState::Stopped;
        }
    }
    pub fn state(&self) -> GuardState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn healthy_commands_cannot_mask_a_dead_sampler() {
        let mut telemetry = TelemetryGuard::new("cpu".into(), "fan".into(), 80.0).unwrap();
        for (source, value) in [("cpu", 40.0), ("fan", 900.0)] {
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
        for sequence in 1..=7 {
            let now = sequence * 200;
            control
                .accept(
                    DutyLease {
                        sequence,
                        issued_at_ms: now,
                        duty_percent: 40,
                    },
                    now,
                )
                .unwrap();
            assert_eq!(control.poll(now), 40);
        }
        assert!(control
            .accept(
                DutyLease {
                    sequence: 8,
                    issued_at_ms: 1600,
                    duty_percent: 40
                },
                1600
            )
            .is_err());
        assert_eq!(control.poll(1600), 100);
        assert_eq!(control.telemetry_fault(), Some(TelemetryFault::Stale));
        assert!(control
            .observe(
                Reading {
                    source: "cpu",
                    acquired_ms: 1601,
                    value: 40.0
                },
                1601
            )
            .is_err());
    }
    #[test]
    fn healthy_sampler_cannot_mask_a_dead_controller() {
        let mut telemetry = TelemetryGuard::new("cpu".into(), "fan".into(), 80.0).unwrap();
        for (source, value) in [("cpu", 40.0), ("fan", 900.0)] {
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
        control
            .accept(
                DutyLease {
                    sequence: 1,
                    issued_at_ms: 0,
                    duty_percent: 40,
                },
                0,
            )
            .unwrap();
        for (source, value) in [("cpu", 40.0), ("fan", 900.0)] {
            control
                .observe(
                    Reading {
                        source,
                        acquired_ms: 1000,
                        value,
                    },
                    1000,
                )
                .unwrap();
        }
        assert_eq!(control.poll(1000), 100);
        assert_eq!(control.lease_state(), GuardState::Expired);
        assert_eq!(control.telemetry_fault(), None);
    }
    #[test]
    fn delayed_and_replayed_commands_cannot_extend_a_lease_or_revive_it() {
        let mut g = LeaseGuard::new(0, 1000, 40).unwrap();
        let command = DutyLease {
            sequence: 1,
            issued_at_ms: 0,
            duty_percent: 40,
        };
        g.accept(command, 100).unwrap();
        assert_eq!(g.poll(999), 40);
        assert!(g.accept(command, 999).is_err());
        assert_eq!(g.poll(1000), 100);
        assert_eq!(g.state(), GuardState::Expired);
        assert!(g
            .accept(
                DutyLease {
                    sequence: 2,
                    issued_at_ms: 1000,
                    duty_percent: 50
                },
                1000
            )
            .is_err());
        let mut g = LeaseGuard::new(0, 1000, 40).unwrap();
        assert!(g
            .accept(
                DutyLease {
                    sequence: 1,
                    issued_at_ms: 0,
                    duty_percent: 40
                },
                251
            )
            .is_err());
        assert_eq!(g.poll(1000), 100);
    }
    #[test]
    fn clock_reversal_stop_and_bad_duties_never_request_a_low_fallback() {
        let mut g = LeaseGuard::new(100, 1000, 40).unwrap();
        assert!(g
            .accept(
                DutyLease {
                    sequence: 1,
                    issued_at_ms: 101,
                    duty_percent: 50
                },
                100
            )
            .is_err());
        assert!(g
            .accept(
                DutyLease {
                    sequence: 1,
                    issued_at_ms: 100,
                    duty_percent: 39
                },
                100
            )
            .is_err());
        assert_eq!(g.poll(99), 100);
        assert_eq!(g.state(), GuardState::ClockFault);
        let mut g = LeaseGuard::new(0, 1000, 100).unwrap(); // pump policy
        assert!(g
            .accept(
                DutyLease {
                    sequence: 1,
                    issued_at_ms: 0,
                    duty_percent: 99
                },
                0
            )
            .is_err());
        g.stop();
        assert_eq!(g.poll(10), 100);
        assert_eq!(g.state(), GuardState::Stopped);
        assert!(LeaseGuard::new(u64::MAX, 1000, 40).is_err());
        assert!(LeaseGuard::new(0, 1000, 0).is_err());
        assert!(LeaseGuard::new(0, 1000, 19).is_err());
        let mut stop = LeaseGuard::with_floor(0, 1000, 0, true).unwrap();
        assert!(stop
            .accept(
                DutyLease {
                    sequence: 1,
                    issued_at_ms: 0,
                    duty_percent: 0
                },
                0
            )
            .is_ok());
        assert_eq!(stop.poll(0), 0);
        assert_eq!(stop.poll(1000), 100);
    }
}
