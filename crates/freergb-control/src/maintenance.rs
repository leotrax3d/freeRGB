//! Maintenance admission policy, owned by one supervisor event loop.
//! This is not an OS lock or authentication boundary. Production integration
//! must serialize admission and maintenance in the protected supervisor.
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenanceState {
    Running,
    Draining,
    Ready,
    Blocked,
}

/// Supervisor-issued lifetime identity; never reuse a worker PID as this token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionToken(u64);

/// Evidence from the authenticated backend, not values asserted by a GUI.
/// Native callback completion alone does not establish firmware restoration.
#[derive(Clone, Copy, Debug)]
pub struct ReleaseEvidence {
    pub native_released: bool,
    pub firmware_return_verified: bool,
    pub journal_cleared: bool,
}

pub struct MaintenanceGate {
    state: MaintenanceState,
    sessions: BTreeSet<SessionToken>,
    next: u64,
    last_now: u64,
    deadline: Option<u64>,
}

impl MaintenanceGate {
    /// Unresolved journal state fails closed, including unsupported/corrupt state.
    pub fn new(recovery_clear: bool, now_ms: u64) -> Self {
        Self {
            state: if recovery_clear {
                MaintenanceState::Running
            } else {
                MaintenanceState::Blocked
            },
            sessions: BTreeSet::new(),
            next: 0,
            last_now: now_ms,
            deadline: None,
        }
    }

    pub fn admit(&mut self, now_ms: u64) -> Result<SessionToken, String> {
        self.poll(now_ms);
        if self.state != MaintenanceState::Running || self.sessions.len() >= 128 {
            return Err("Control admission is closed or full".into());
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Session identity exhausted")?;
        let token = SessionToken(self.next);
        self.sessions.insert(token);
        Ok(token)
    }

    /// Closes admission before returning the exact sessions that must release.
    /// Repeated begin calls never extend the deadline.
    pub fn begin(&mut self, now_ms: u64, timeout_ms: u64) -> Result<Vec<SessionToken>, String> {
        self.poll(now_ms);
        if self.state != MaintenanceState::Running || !(100..=30_000).contains(&timeout_ms) {
            return Err("Maintenance cannot start in this state or with this timeout".into());
        }
        let deadline = now_ms
            .checked_add(timeout_ms)
            .ok_or("Maintenance clock overflow")?;
        self.deadline = Some(deadline);
        self.state = if self.sessions.is_empty() {
            MaintenanceState::Ready
        } else {
            MaintenanceState::Draining
        };
        Ok(self.sessions.iter().copied().collect())
    }

    pub fn released(
        &mut self,
        token: SessionToken,
        evidence: ReleaseEvidence,
        now_ms: u64,
    ) -> Result<(), String> {
        self.poll(now_ms);
        if !matches!(
            self.state,
            MaintenanceState::Running | MaintenanceState::Draining
        ) || !self.sessions.contains(&token)
        {
            return Err("Unknown, expired or already released session".into());
        }
        if !evidence.native_released
            || !evidence.firmware_return_verified
            || !evidence.journal_cleared
        {
            self.state = MaintenanceState::Blocked;
            return Err("Release evidence incomplete; preserve recovery state and binaries".into());
        }
        self.sessions.remove(&token);
        if self.state == MaintenanceState::Draining && self.sessions.is_empty() {
            self.state = MaintenanceState::Ready;
        }
        Ok(())
    }

    /// Worker disappearance is not successful release, even if its pipe is closed.
    pub fn worker_lost(&mut self, token: SessionToken) {
        if self.sessions.contains(&token) {
            self.state = MaintenanceState::Blocked;
        }
    }

    pub fn poll(&mut self, now_ms: u64) -> MaintenanceState {
        if now_ms < self.last_now {
            self.state = MaintenanceState::Blocked;
        }
        self.last_now = now_ms;
        if self.state == MaintenanceState::Draining
            && self.deadline.is_some_and(|end| now_ms >= end)
        {
            self.state = MaintenanceState::Blocked;
        }
        self.state
    }

    /// Ready is terminal: after replacement, a new supervisor must validate the
    /// journal/backend and create its own gate. There is no automatic reopening.
    pub fn state(&self) -> MaintenanceState {
        self.state
    }
    pub fn unresolved_count(&self) -> usize {
        self.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn evidence() -> ReleaseEvidence {
        ReleaseEvidence {
            native_released: true,
            firmware_return_verified: true,
            journal_cleared: true,
        }
    }
    #[test]
    fn maintenance_closes_admission_and_waits_for_every_exact_session() {
        let mut gate = MaintenanceGate::new(true, 0);
        let a = gate.admit(0).unwrap();
        let b = gate.admit(1).unwrap();
        assert_eq!(gate.begin(2, 1000).unwrap(), vec![a, b]);
        assert!(gate.admit(3).is_err());
        gate.released(a, evidence(), 4).unwrap();
        assert_eq!(gate.state(), MaintenanceState::Draining);
        assert!(gate.released(a, evidence(), 5).is_err());
        gate.released(b, evidence(), 6).unwrap();
        assert_eq!(gate.state(), MaintenanceState::Ready);
        assert!(gate.admit(7).is_err());
        assert!(gate.begin(7, 1000).is_err());
    }
    #[test]
    fn partial_release_crash_timeout_and_clock_fault_preserve_unresolved_sessions() {
        for failure in 0..5 {
            let mut gate = MaintenanceGate::new(true, 0);
            let token = gate.admit(0).unwrap();
            gate.begin(10, 1000).unwrap();
            match failure {
                0..=2 => {
                    let mut proof = evidence();
                    match failure {
                        0 => proof.native_released = false,
                        1 => proof.firmware_return_verified = false,
                        _ => proof.journal_cleared = false,
                    }
                    assert!(gate.released(token, proof, 20).is_err());
                }
                3 => gate.worker_lost(token),
                _ => {
                    gate.poll(1010);
                }
            }
            assert_eq!(gate.state(), MaintenanceState::Blocked);
            assert_eq!(gate.unresolved_count(), 1);
            assert!(gate.released(token, evidence(), 1011).is_err());
            assert!(gate.admit(1011).is_err());
        }
        let mut gate = MaintenanceGate::new(true, 10);
        assert_eq!(gate.poll(9), MaintenanceState::Blocked);
        assert!(MaintenanceGate::new(false, 0).begin(0, 1000).is_err());
    }
    #[test]
    fn normal_close_does_not_reuse_tokens_and_retry_cannot_extend_drain() {
        let mut gate = MaintenanceGate::new(true, 0);
        let old = gate.admit(0).unwrap();
        gate.released(old, evidence(), 1).unwrap();
        let current = gate.admit(2).unwrap();
        assert_ne!(old, current);
        gate.begin(3, 100).unwrap();
        assert!(gate.begin(100, 1000).is_err());
        assert!(gate.released(old, evidence(), 100).is_err());
        assert_eq!(gate.poll(103), MaintenanceState::Blocked);
        assert_eq!(MaintenanceGate::new(true, 0).begin(0, 100).unwrap(), vec![]);
        assert!(MaintenanceGate::new(true, u64::MAX)
            .begin(u64::MAX, 100)
            .is_err());
    }
}
