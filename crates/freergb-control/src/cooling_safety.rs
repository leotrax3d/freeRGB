//! Hardware-independent lease policy. A requested fallback is not proof of a write.
use serde::{Deserialize, Serialize};

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
        if !(500..=3000).contains(&lease_ms) || !(20..=100).contains(&floor) {
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
    }
}
