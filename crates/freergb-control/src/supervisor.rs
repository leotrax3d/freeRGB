//! Independent fault supervisor for boot-time cooling (docs/COOLING_SERVICE.md).
//! Pure policy owned by the protected service's own timer, never by the
//! component it watches: the Rust cooling service (heartbeats), the boot
//! sequence (deadline until every remembered channel was decided) and the
//! native driver/worker (bounded reply time per request). Any fault latches;
//! the owner then returns every channel (100 %, then firmware mode) and does
//! not resume until a new supervisor lifetime validated recovery.
//!
//! Maintenance (upgrade, uninstall, rollback) reuses [`MaintenanceGate`]: the
//! supervisor closes admission, drains every session with verified return and
//! only then authorizes the stage. Faults during a drain block maintenance.
use crate::maintenance::{MaintenanceGate, MaintenanceState, ReleaseEvidence, SessionToken};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Component {
    /// The Rust cooling service stopped sending heartbeats (hang or crash).
    Service,
    /// Boot did not finish deciding its channels in time.
    Boot,
    /// A native/driver request did not complete in time (hung driver).
    Driver,
    /// Clock moved backwards: no deadline can be trusted any more.
    Clock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub service_heartbeat_ms: u64,
    pub boot_ms: u64,
    pub driver_reply_ms: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            service_heartbeat_ms: 3_000,
            boot_ms: 90_000,
            driver_reply_ms: 1_000,
        }
    }
}

impl Limits {
    pub fn validate(&self) -> Result<(), String> {
        let ok = (500..=30_000).contains(&self.service_heartbeat_ms)
            && (5_000..=600_000).contains(&self.boot_ms)
            && (100..=10_000).contains(&self.driver_reply_ms);
        ok.then_some(())
            .ok_or_else(|| "Supervisor limits out of range".into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Booting,
    Healthy,
    /// Return every channel: 100 %, then verified firmware mode. Terminal.
    Fault(Component),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenanceKind {
    Upgrade,
    Uninstall,
    Rollback,
}

/// What the installer may do once the drain finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Replace binaries, then a new supervisor validates recovery and resumes.
    ReplaceAndRestart,
    /// Remove service and files; channels stay in firmware mode.
    RemoveService,
    /// Restore the previous payload, then a new supervisor resumes.
    RestorePreviousAndRestart,
}

pub struct Supervisor {
    limits: Limits,
    started: u64,
    last_now: u64,
    last_heartbeat: u64,
    boot_done: bool,
    /// Start time of the oldest outstanding native request.
    driver_pending: Option<u64>,
    fault: Option<Component>,
    gate: MaintenanceGate,
    maintenance: Option<MaintenanceKind>,
}

impl Supervisor {
    /// `recovery_clear` comes from the protected journal check, never a client.
    pub fn new(limits: Limits, recovery_clear: bool, now_ms: u64) -> Result<Self, String> {
        limits.validate()?;
        Ok(Self {
            limits,
            started: now_ms,
            last_now: now_ms,
            last_heartbeat: now_ms,
            boot_done: false,
            driver_pending: None,
            fault: None,
            gate: MaintenanceGate::new(recovery_clear, now_ms),
            maintenance: None,
        })
    }

    pub fn heartbeat(&mut self, now_ms: u64) {
        if self.tick(now_ms).is_none() {
            self.last_heartbeat = now_ms;
        }
    }

    pub fn boot_complete(&mut self, now_ms: u64) {
        if self.tick(now_ms).is_none() {
            self.boot_done = true;
        }
    }

    pub fn driver_request(&mut self, now_ms: u64) {
        if self.tick(now_ms).is_none() && self.driver_pending.is_none() {
            self.driver_pending = Some(now_ms);
        }
    }

    pub fn driver_reply(&mut self, now_ms: u64) {
        if self.tick(now_ms).is_none() {
            self.driver_pending = None;
        }
    }

    /// A new channel session; refused after a fault or while maintenance runs.
    pub fn admit(&mut self, now_ms: u64) -> Result<SessionToken, String> {
        if let Some(component) = self.tick(now_ms) {
            return Err(format!("Supervisor fault ({component:?}); no new sessions"));
        }
        self.gate.admit(now_ms)
    }

    /// Verified return of one session (normal stop or maintenance drain).
    pub fn released(
        &mut self,
        token: SessionToken,
        evidence: ReleaseEvidence,
        now_ms: u64,
    ) -> Result<(), String> {
        self.tick(now_ms);
        self.gate.released(token, evidence, now_ms)
    }

    pub fn worker_lost(&mut self, token: SessionToken) {
        self.gate.worker_lost(token);
    }

    /// Closes admission and returns the sessions the owner must return.
    pub fn begin_maintenance(
        &mut self,
        kind: MaintenanceKind,
        now_ms: u64,
        timeout_ms: u64,
    ) -> Result<Vec<SessionToken>, String> {
        if let Some(component) = self.tick(now_ms) {
            return Err(format!(
                "Supervisor fault ({component:?}); recovery inspection before maintenance"
            ));
        }
        let tokens = self.gate.begin(now_ms, timeout_ms)?;
        self.maintenance = Some(kind);
        Ok(tokens)
    }

    /// `Some(stage)` only once every session returned with full evidence.
    pub fn maintenance_stage(&mut self, now_ms: u64) -> Option<Stage> {
        let faulted = self.tick(now_ms).is_some();
        let ready = self.gate.poll(now_ms) == MaintenanceState::Ready;
        match (self.maintenance, ready && !faulted) {
            (Some(MaintenanceKind::Upgrade), true) => Some(Stage::ReplaceAndRestart),
            (Some(MaintenanceKind::Uninstall), true) => Some(Stage::RemoveService),
            (Some(MaintenanceKind::Rollback), true) => Some(Stage::RestorePreviousAndRestart),
            _ => None,
        }
    }

    pub fn maintenance_state(&mut self, now_ms: u64) -> MaintenanceState {
        self.gate.poll(now_ms)
    }

    pub fn check(&mut self, now_ms: u64) -> Verdict {
        match self.tick(now_ms) {
            Some(component) => Verdict::Fault(component),
            None if self.boot_done => Verdict::Healthy,
            None => Verdict::Booting,
        }
    }

    fn tick(&mut self, now_ms: u64) -> Option<Component> {
        if self.fault.is_none() {
            self.fault = if now_ms < self.last_now {
                Some(Component::Clock)
            } else if now_ms.saturating_sub(self.last_heartbeat) > self.limits.service_heartbeat_ms
            {
                Some(Component::Service)
            } else if !self.boot_done && now_ms.saturating_sub(self.started) > self.limits.boot_ms {
                Some(Component::Boot)
            } else if self
                .driver_pending
                .is_some_and(|since| now_ms.saturating_sub(since) > self.limits.driver_reply_ms)
            {
                Some(Component::Driver)
            } else {
                None
            };
        }
        self.last_now = self.last_now.max(now_ms);
        self.fault
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: ReleaseEvidence = ReleaseEvidence {
        native_released: true,
        firmware_return_verified: true,
        journal_cleared: true,
    };

    fn healthy() -> Supervisor {
        let mut s = Supervisor::new(Limits::default(), true, 0).unwrap();
        s.boot_complete(1_000);
        s
    }

    #[test]
    fn limits_are_bounded() {
        let bad = Limits {
            driver_reply_ms: 0,
            ..Limits::default()
        };
        assert!(Supervisor::new(bad, true, 0).is_err());
    }

    #[test]
    fn boot_finishes_or_faults_at_its_deadline() {
        let mut s = Supervisor::new(Limits::default(), true, 0).unwrap();
        for t in (0..=90_000).step_by(1_000) {
            s.heartbeat(t);
        }
        assert_eq!(s.check(90_000), Verdict::Booting);
        s.heartbeat(90_500);
        assert_eq!(s.check(90_001), Verdict::Fault(Component::Boot));
        // Late completion cannot clear the latched fault.
        s.boot_complete(90_600);
        assert_eq!(s.check(90_700), Verdict::Fault(Component::Boot));
        assert!(s.admit(90_800).is_err());
    }

    #[test]
    fn service_hang_faults_and_blocks_admission() {
        let mut s = healthy();
        s.heartbeat(2_000);
        assert_eq!(s.check(5_000), Verdict::Healthy);
        assert_eq!(s.check(5_001), Verdict::Fault(Component::Service));
        s.heartbeat(5_002);
        assert_eq!(s.check(5_003), Verdict::Fault(Component::Service));
    }

    #[test]
    fn hung_driver_request_faults_even_with_heartbeats() {
        let mut s = healthy();
        s.driver_request(1_000);
        s.driver_reply(1_500);
        s.driver_request(2_000);
        s.heartbeat(2_500);
        assert_eq!(s.check(3_000), Verdict::Healthy);
        s.heartbeat(3_001);
        assert_eq!(s.check(3_001), Verdict::Fault(Component::Driver));
    }

    #[test]
    fn clock_reversal_faults() {
        let mut s = healthy();
        s.heartbeat(2_000);
        assert_eq!(s.check(1_999), Verdict::Fault(Component::Clock));
    }

    fn drain(kind: MaintenanceKind) -> Option<Stage> {
        let mut s = healthy();
        let a = s.admit(1_100).unwrap();
        let b = s.admit(1_200).unwrap();
        let tokens = s.begin_maintenance(kind, 1_300, 5_000).unwrap();
        assert_eq!(tokens, vec![a, b]);
        assert!(s.admit(1_400).is_err(), "admission closed while draining");
        s.released(a, OK, 1_500).unwrap();
        assert_eq!(s.maintenance_stage(1_600), None);
        s.heartbeat(1_700);
        s.released(b, OK, 1_800).unwrap();
        s.maintenance_stage(1_900)
    }

    #[test]
    fn upgrade_uninstall_and_rollback_drain_before_their_stage() {
        assert_eq!(
            drain(MaintenanceKind::Upgrade),
            Some(Stage::ReplaceAndRestart)
        );
        assert_eq!(
            drain(MaintenanceKind::Uninstall),
            Some(Stage::RemoveService)
        );
        assert_eq!(
            drain(MaintenanceKind::Rollback),
            Some(Stage::RestorePreviousAndRestart)
        );
    }

    #[test]
    fn incomplete_evidence_lost_worker_or_timeout_block_maintenance() {
        for case in 0..3 {
            let mut s = healthy();
            let a = s.admit(1_100).unwrap();
            s.begin_maintenance(MaintenanceKind::Upgrade, 1_200, 1_000)
                .unwrap();
            match case {
                0 => {
                    let partial = ReleaseEvidence {
                        journal_cleared: false,
                        ..OK
                    };
                    assert!(s.released(a, partial, 1_300).is_err());
                }
                1 => s.worker_lost(a),
                _ => s.heartbeat(2_200),
            }
            assert_eq!(s.maintenance_stage(2_200), None, "case {case}");
            assert_eq!(s.maintenance_state(2_200), MaintenanceState::Blocked);
        }
    }

    #[test]
    fn fault_during_drain_or_unresolved_journal_refuses_maintenance() {
        let mut s = healthy();
        let a = s.admit(1_100).unwrap();
        s.begin_maintenance(MaintenanceKind::Uninstall, 1_200, 10_000)
            .unwrap();
        // Service hangs while draining; a late release cannot authorize removal.
        s.released(a, OK, 4_500).ok();
        assert_eq!(s.maintenance_stage(4_600), None);

        let mut blocked = Supervisor::new(Limits::default(), false, 0).unwrap();
        blocked.boot_complete(100);
        assert!(blocked.admit(200).is_err());
        assert!(blocked
            .begin_maintenance(MaintenanceKind::Rollback, 300, 1_000)
            .is_err());
    }
}
