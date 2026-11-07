//! Backend-independent Manual-mode interlock. A fixed duty is only honoured
//! while a fresh, plausible temperature and a live controller loop vouch for
//! it; otherwise the decision is 100% (the safe state). Pure policy: it never
//! writes hardware, and a decision is not proof that a backend applied it.
//!
//! Bound: for hang, suspend and missing/stale sensors, the next `evaluate`
//! call after `max_sensor_age_ms` (sensor) or `max_gap_ms` (loop gap) yields
//! the safe state. A killed process cannot call `evaluate`; that case is
//! covered only by the recovery journal and the out-of-process guard, not by
//! this model.

use serde::Serialize;

pub const DEFAULT_CRITICAL_C: f32 = 85.0;
pub const DEFAULT_HYSTERESIS_C: f32 = 5.0;
pub const DEFAULT_MAX_SENSOR_AGE_MS: u64 = 3000;
pub const DEFAULT_MAX_GAP_MS: u64 = 3000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterlockReason {
    /// No temperature has ever been observed.
    SensorMissing,
    SensorStale,
    SensorInvalid,
    Critical,
    /// The control loop itself stalled (hang or system suspend).
    LoopGap,
    ClockFault,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decision {
    pub duty_percent: f32,
    /// `Some` means the safe state is in force and why.
    pub interlock: Option<InterlockReason>,
}

#[derive(Clone, Debug)]
pub struct ManualInterlock {
    critical_c: f32,
    hysteresis_c: f32,
    max_sensor_age_ms: u64,
    max_gap_ms: u64,
    temperature: Option<(u64, f32)>,
    invalid: bool,
    critical_latched: bool,
    last_now: Option<u64>,
    /// After a loop gap only a sample acquired at/after this time counts.
    fresh_after: u64,
    clock_fault: bool,
    gap: bool,
}

impl Default for ManualInterlock {
    fn default() -> Self {
        Self::new(
            DEFAULT_CRITICAL_C,
            DEFAULT_HYSTERESIS_C,
            DEFAULT_MAX_SENSOR_AGE_MS,
            DEFAULT_MAX_GAP_MS,
        )
    }
}

impl ManualInterlock {
    pub fn new(
        critical_c: f32,
        hysteresis_c: f32,
        max_sensor_age_ms: u64,
        max_gap_ms: u64,
    ) -> Self {
        Self {
            critical_c,
            hysteresis_c: hysteresis_c.max(0.0),
            max_sensor_age_ms,
            max_gap_ms,
            temperature: None,
            invalid: false,
            critical_latched: false,
            last_now: None,
            fresh_after: 0,
            clock_fault: false,
            gap: false,
        }
    }

    /// Worst-case time from loss of a sensor or of the loop to the safe
    /// decision, given one `evaluate` call per `poll_ms`.
    pub fn worst_case_bound_ms(&self, poll_ms: u64) -> u64 {
        self.max_sensor_age_ms.max(self.max_gap_ms) + poll_ms
    }

    /// Feed one temperature sample (acquisition time, monotonic ms). `None`
    /// means the read failed.
    pub fn observe(&mut self, acquired_ms: u64, celsius: Option<f32>) {
        let Some(value) = celsius else {
            return;
        };
        if !value.is_finite() || !(-20.0..=150.0).contains(&value) {
            self.invalid = true;
            return;
        }
        if acquired_ms < self.fresh_after {
            return; // cached sample from before a stall
        }
        if self.temperature.is_some_and(|(at, _)| acquired_ms < at) {
            self.clock_fault = true;
            return;
        }
        self.invalid = false;
        self.temperature = Some((acquired_ms, value));
        if value >= self.critical_c {
            self.critical_latched = true;
        } else if value < self.critical_c - self.hysteresis_c {
            self.critical_latched = false;
        }
    }

    pub fn evaluate(&mut self, requested_percent: f32, now_ms: u64) -> Decision {
        if let Some(previous) = self.last_now {
            if now_ms < previous {
                self.clock_fault = true;
            } else if now_ms - previous > self.max_gap_ms {
                self.gap = true;
                self.fresh_after = now_ms;
            }
        }
        self.last_now = Some(now_ms);
        // A gap clears once a post-gap sample was accepted.
        if self.gap
            && self
                .temperature
                .is_some_and(|(at, _)| at >= self.fresh_after)
        {
            self.gap = false;
        }
        let reason = self.reason(now_ms);
        Decision {
            duty_percent: if reason.is_some() {
                100.0
            } else {
                requested_percent
            },
            interlock: reason,
        }
    }

    fn reason(&self, now_ms: u64) -> Option<InterlockReason> {
        if self.clock_fault {
            return Some(InterlockReason::ClockFault);
        }
        if self.invalid {
            return Some(InterlockReason::SensorInvalid);
        }
        let Some((at, _)) = self.temperature else {
            return Some(InterlockReason::SensorMissing);
        };
        if self.gap {
            return Some(InterlockReason::LoopGap);
        }
        if now_ms.saturating_sub(at) > self.max_sensor_age_ms {
            return Some(InterlockReason::SensorStale);
        }
        self.critical_latched.then_some(InterlockReason::Critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn armed() -> ManualInterlock {
        let mut i = ManualInterlock::default();
        i.observe(0, Some(50.0));
        i
    }

    #[test]
    fn healthy_sensor_keeps_the_requested_duty() {
        let mut i = armed();
        let d = i.evaluate(60.0, 1000);
        assert_eq!((d.duty_percent, d.interlock), (60.0, None));
    }

    #[test]
    fn missing_sensor_is_safe_from_the_first_evaluation() {
        let mut i = ManualInterlock::default();
        let d = i.evaluate(60.0, 0);
        assert_eq!(d.duty_percent, 100.0);
        assert_eq!(d.interlock, Some(InterlockReason::SensorMissing));
    }

    #[test]
    fn stale_sensor_reaches_safe_state_within_bound() {
        let mut i = armed();
        let bound = i.worst_case_bound_ms(1000);
        let mut t = 0;
        let mut safe_at = None;
        while t <= bound {
            t += 1000;
            if i.evaluate(60.0, t).interlock.is_some() {
                safe_at = Some(t);
                break;
            }
        }
        let safe_at = safe_at.expect("safe state reached");
        assert!(safe_at <= bound);
        // A fresh sample releases it again.
        i.observe(safe_at + 100, Some(50.0));
        assert_eq!(i.evaluate(60.0, safe_at + 200).interlock, None);
    }

    #[test]
    fn critical_temperature_latches_until_hysteresis_cleared() {
        let mut i = armed();
        i.observe(1000, Some(86.0));
        assert_eq!(
            i.evaluate(60.0, 1100).interlock,
            Some(InterlockReason::Critical)
        );
        i.observe(2000, Some(83.0));
        assert_eq!(
            i.evaluate(60.0, 2100).interlock,
            Some(InterlockReason::Critical),
            "still inside hysteresis"
        );
        i.observe(3000, Some(70.0));
        assert_eq!(i.evaluate(60.0, 3100).duty_percent, 60.0);
    }

    #[test]
    fn suspend_or_hang_gap_ignores_cached_samples_until_a_fresh_one() {
        let mut i = armed();
        // Loop wakes after a 60 s suspend: a sample acquired before the
        // wake-up must not re-arm Manual.
        assert_eq!(i.evaluate(60.0, 1_000).duty_percent, 60.0);
        let d = i.evaluate(60.0, 60_000);
        assert_eq!(d.duty_percent, 100.0);
        i.observe(59_000, Some(50.0));
        assert_eq!(i.evaluate(60.0, 60_100).duty_percent, 100.0);
        i.observe(60_200, Some(50.0));
        assert_eq!(i.evaluate(60.0, 60_300).duty_percent, 60.0);
    }

    #[test]
    fn invalid_sample_and_clock_regression_are_safe() {
        let mut i = armed();
        i.observe(500, Some(f32::NAN));
        assert_eq!(
            i.evaluate(60.0, 600).interlock,
            Some(InterlockReason::SensorInvalid)
        );
        i.observe(700, Some(50.0));
        assert_eq!(i.evaluate(60.0, 800).interlock, None);
        assert_eq!(
            i.evaluate(60.0, 700).interlock,
            Some(InterlockReason::ClockFault)
        );
        i.observe(900, Some(50.0));
        assert_eq!(
            i.evaluate(60.0, 1000).interlock,
            Some(InterlockReason::ClockFault),
            "clock fault is latched"
        );
    }
}
