//! Bounded fan response experiment, reusable by a future isolated control backend.
//! This model requests duties; it cannot establish wiring identity or write success.
use crate::channels::ChannelRole;
use serde::{Deserialize, Serialize};

const DUTIES: [u8; 4] = [100, 80, 60, 40];
/// Guided channel test (COOLING_BROAD_SUPPORT_PLAN.md, safeguard 1): never below 40%.
pub const CHANNEL_TEST_DUTIES: [u8; 3] = [100, 70, 50];

/// Parameters of a fine calibration sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SweepOptions {
    pub step_percent: u8,
    pub lowest_percent: u8,
    #[serde(default)]
    pub probe_stall: bool,
}
impl SweepOptions {
    pub fn validate(&self) -> Result<(), String> {
        if !(2..=20).contains(&self.step_percent) {
            return Err("Calibration step must be 2..20 percent".into());
        }
        if self.lowest_percent >= 100 || (self.lowest_percent < SWEEP_FLOOR && !self.probe_stall) {
            return Err(
                "Calibration below 40% needs a stall sweep on an unrestricted channel".into(),
            );
        }
        Ok(())
    }
}
const SWEEP_FLOOR: u8 = 40;

/// Mean RPM measured at one tested duty.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestedDuty {
    pub duty_percent: u8,
    pub rpm: f32,
}

#[derive(Deserialize)]
pub struct CalibrationPreviewRequest {
    pub role: ChannelRole,
    pub critical_c: f32,
    pub samples: Vec<CalibrationSample>,
}
#[derive(Serialize)]
pub struct CalibrationPreview {
    pub initial: CalibrationOutput,
    pub trace: Vec<CalibrationOutput>,
    pub final_output: CalibrationOutput,
    pub simulated: bool,
    pub hardware_control: bool,
}
impl CalibrationPreviewRequest {
    pub fn run(&self) -> Result<CalibrationPreview, String> {
        if self.samples.len() > 512 || self.samples.iter().any(|s| s.at_ms > 61_001) {
            return Err("Preview supports at most 512 samples within 0..61001 ms".into());
        }
        let mut calibration = Calibration::new(self.role, self.critical_c, 0)?;
        let initial = calibration.output();
        let trace = self
            .samples
            .iter()
            .map(|s| calibration.update(*s))
            .collect();
        Ok(CalibrationPreview {
            initial,
            trace,
            final_output: calibration.output(),
            simulated: true,
            hardware_control: false,
        })
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CalibrationSample {
    pub at_ms: u64,
    pub applied_sequence: Option<u64>,
    pub rpm: Option<f32>,
    pub rpm_age_ms: u64,
    pub temperature_c: Option<f32>,
    pub temperature_age_ms: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationPhase {
    Applying,
    Settling,
    Sampling,
    Restoring,
    Completed,
    Aborted,
    RestorationFailed,
}
#[derive(Clone, Debug, Serialize)]
pub struct CalibrationResult {
    pub response_observed: bool,
    pub lowest_tested_percent: u8,
    pub suggested_floor_percent: Option<u8>,
    pub hardware_verified: bool,
    /// Mean RPM per tested duty, highest duty first.
    pub tested: Vec<TestedDuty>,
    /// Every lower duty produced at least 5% lower RPM (monotonic, plausible);
    /// fine sweeps: no step rose by more than 2% (non-increasing within noise).
    pub monotonic_response: bool,
    /// Highest duty at which the fan stopped (stall sweep only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stall_percent: Option<u8>,
    /// Lowest duty that restarted the fan from standstill (stall sweep only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_percent: Option<u8>,
}
impl CalibrationResult {
    /// Guided channel test verdict: plausible, monotonic response at 100/70/50%.
    pub fn channel_test_passed(&self) -> bool {
        self.response_observed
            && self.monotonic_response
            && self.tested.len() == CHANNEL_TEST_DUTIES.len()
            && self.tested.iter().all(|t| t.rpm >= 100.0)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct CalibrationOutput {
    pub phase: CalibrationPhase,
    pub sequence: u64,
    pub duty_percent: u8,
    pub reason: Option<String>,
    pub result: Option<CalibrationResult>,
}
pub struct Calibration {
    output: CalibrationOutput,
    began: u64,
    last_now: u64,
    issued: u64,
    acknowledged: Option<u64>,
    last_sample: Option<u64>,
    settling_rpm: Option<f32>,
    settling_direction: i8,
    sampling_began: Option<u64>,
    stage: usize,
    readings: Vec<f32>,
    means: Vec<f32>,
    critical_c: f32,
    pending_result: Option<CalibrationResult>,
    duties: Vec<u8>,
    probe_stall: bool,
    /// Number of descending stages (the rest probe the start duty upwards).
    descending: usize,
    stall_percent: Option<u8>,
    start_percent: Option<u8>,
    timeout_ms: u64,
    /// Each lower duty must give at most this fraction of the previous RPM
    /// (0.95 for the coarse calibration, 1.02 noise tolerance for sweeps).
    monotonic_ratio: f32,
}
impl Calibration {
    pub fn new(role: ChannelRole, critical_c: f32, now_ms: u64) -> Result<Self, String> {
        Self::with_duties(role, critical_c, now_ms, DUTIES.to_vec(), false)
    }
    /// Fine calibration: 100% down to `lowest_percent` in `step_percent` steps.
    /// Below the 40% safety floor only with `probe_stall`, which the caller
    /// sets only for unrestricted channels: a stopped fan is then recorded as
    /// the stall duty and the sweep probes upwards for the start duty.
    pub fn sweep(
        role: ChannelRole,
        critical_c: f32,
        now_ms: u64,
        options: SweepOptions,
    ) -> Result<Self, String> {
        options.validate()?;
        let mut duties = Vec::new();
        let mut duty = 100u8;
        while duty > options.lowest_percent {
            duties.push(duty);
            duty = duty.saturating_sub(options.step_percent);
        }
        duties.push(options.lowest_percent);
        let mut sweep = Self::with_duties(role, critical_c, now_ms, duties, options.probe_stall)?;
        // Fine steps change the speed less and many fans saturate near 100%
        // (owner acceptance B1, 2026-11-05: nothing was stored). A sweep only
        // needs a non-increasing response within 2% noise; `response_observed`
        // still requires a 10% overall drop.
        sweep.monotonic_ratio = 1.02;
        Ok(sweep)
    }
    /// Guided channel test: 100/70/50% with the same supervision as calibration.
    pub fn channel_test(role: ChannelRole, critical_c: f32, now_ms: u64) -> Result<Self, String> {
        Self::with_duties(
            role,
            critical_c,
            now_ms,
            CHANNEL_TEST_DUTIES.to_vec(),
            false,
        )
    }
    fn with_duties(
        role: ChannelRole,
        critical_c: f32,
        now_ms: u64,
        duties: Vec<u8>,
        probe_stall: bool,
    ) -> Result<Self, String> {
        if role != ChannelRole::Fan {
            return Err("Automatic duty-reduction calibration excludes pumps".into());
        }
        if !critical_c.is_finite() || !(40.0..=100.0).contains(&critical_c) {
            return Err("Critical temperature must be 40..100 Celsius".into());
        }
        now_ms
            .checked_add(1_000_000)
            .ok_or("Calibration clock overflow")?;
        Ok(Self {
            output: CalibrationOutput {
                phase: CalibrationPhase::Applying,
                sequence: 1,
                duty_percent: 100,
                reason: None,
                result: None,
            },
            began: now_ms,
            last_now: now_ms,
            issued: now_ms,
            acknowledged: None,
            last_sample: None,
            settling_rpm: None,
            settling_direction: 0,
            sampling_began: None,
            stage: 0,
            readings: Vec::new(),
            means: Vec::new(),
            critical_c,
            pending_result: None,
            descending: duties.len(),
            // Include the bounded settling and measurement windows, duty ACKs,
            // and up to five restart probes for a stall sweep.
            timeout_ms: 60_000
                .max(26_000 + 17_000 * (duties.len() as u64 - 1 + if probe_stall { 5 } else { 0 })),
            duties,
            probe_stall,
            monotonic_ratio: 0.95,
            stall_percent: None,
            start_percent: None,
        })
    }
    pub fn output(&self) -> CalibrationOutput {
        self.output.clone()
    }
    fn restore(&mut self, now: u64, reason: Option<&str>) {
        self.output.phase = CalibrationPhase::Restoring;
        self.output.sequence += 1;
        self.output.duty_percent = 100;
        self.output.reason = reason.map(str::to_owned);
        self.issued = now;
        self.acknowledged = None;
    }
    pub fn cancel(&mut self, now_ms: u64) -> CalibrationOutput {
        let now_ms = now_ms.max(self.last_now);
        self.last_now = now_ms;
        if self.output.phase == CalibrationPhase::Restoring {
            self.output.reason = Some("cancelled".into());
            self.pending_result = None;
        }
        if !matches!(
            self.output.phase,
            CalibrationPhase::Completed
                | CalibrationPhase::Aborted
                | CalibrationPhase::RestorationFailed
                | CalibrationPhase::Restoring
        ) {
            self.restore(now_ms, Some("cancelled"));
        }
        self.output()
    }
    pub fn update(&mut self, sample: CalibrationSample) -> CalibrationOutput {
        use CalibrationPhase::*;
        if matches!(self.output.phase, Completed | Aborted | RestorationFailed) {
            return self.output();
        }
        let now = sample.at_ms;
        if now < self.last_now {
            if self.output.phase == Restoring {
                self.output.phase = RestorationFailed;
                self.output.reason = Some("clock_reversal_during_restoration".into());
            } else {
                self.restore(self.last_now, Some("clock_reversal"));
            }
            return self.output();
        }
        self.last_now = now;
        // A stall sweep (unrestricted channels only) counts a stopped fan as
        // 0 RPM instead of a fault, but never while restoring full speed.
        let stall_ok = self.probe_stall && self.output.phase != Restoring;
        let rpm = sample.rpm.filter(|r| r.is_finite()).and_then(|r| {
            if (100.0..=100_000.0).contains(&r) {
                Some(r)
            } else if stall_ok && (0.0..100.0).contains(&r) {
                Some(0.0)
            } else {
                None
            }
        });
        let temp = sample
            .temperature_c
            .filter(|t| t.is_finite() && (-20.0..=150.0).contains(t));
        let fault = if now - self.began > self.timeout_ms {
            Some("session_timeout")
        } else if sample.rpm_age_ms > 1500
            || sample.temperature_age_ms > 1500
            || sample.rpm_age_ms > now
            || sample.temperature_age_ms > now
        {
            Some("stale_or_future_sensor")
        } else if rpm.is_none() {
            Some("missing_or_stalled_rpm")
        } else if temp.is_none() {
            Some("missing_temperature")
        } else if temp.unwrap() >= self.critical_c {
            Some("critical_temperature")
        } else if sample
            .applied_sequence
            .is_some_and(|s| s > self.output.sequence)
        {
            Some("unexpected_acknowledgment")
        } else {
            None
        };
        if self.output.phase == Restoring {
            if self.output.reason.is_none() {
                self.output.reason = fault.map(str::to_owned);
                if fault.is_some() {
                    self.pending_result = None;
                }
            }
            if now.saturating_sub(self.issued) > 1000 {
                self.output.phase = RestorationFailed;
                self.output.reason = Some("full_speed_acknowledgment_timeout".into());
            } else if sample.applied_sequence == Some(self.output.sequence) {
                self.output.phase = if self.output.reason.is_some() {
                    Aborted
                } else {
                    Completed
                };
                if self.output.phase == Completed {
                    self.output.result = self.pending_result.take();
                }
            }
            return self.output();
        }
        if let Some(fault) = fault {
            self.restore(now, Some(fault));
            return self.output();
        }
        if self.acknowledged.is_none() && now - self.issued > 1000 {
            self.restore(now, Some("duty_acknowledgment_timeout"));
            return self.output();
        }
        if sample.applied_sequence != Some(self.output.sequence) {
            if now - self.issued > 1000 {
                self.restore(now, Some("duty_acknowledgment_timeout"));
            }
            return self.output();
        }
        let acknowledged = *self.acknowledged.get_or_insert(now);
        let settle_limit = if self.stage == 0 { 15_000 } else { 8_000 };
        if self.sampling_began.is_none() && now - acknowledged > settle_limit {
            self.restore(now, Some("rpm_settling_timeout"));
            return self.output();
        }
        if self.sampling_began.is_some_and(|start| now - start > 8000) {
            self.restore(now, Some("unstable_or_missing_samples"));
            return self.output();
        }
        let sampled = now - sample.rpm_age_ms;
        if sampled < acknowledged
            || self
                .last_sample
                .is_some_and(|last| sampled < last.saturating_add(250))
        {
            return self.output();
        }
        self.last_sample = Some(sampled);
        if self.sampling_began.is_none() {
            self.output.phase = Settling;
            let current = rpm.unwrap();
            let prior = self.settling_rpm.replace(current);
            let direction = prior.map_or(0, |prior| {
                // Ignore tachometer noise, but follow an accelerating or
                // decelerating fan before opening the measurement window.
                let tolerance = (prior.max(current) * 0.01).max(10.0);
                if current > prior + tolerance {
                    1
                } else if current < prior - tolerance {
                    -1
                } else {
                    0
                }
            });
            let moving = direction != 0
                && (self.settling_direction == 0 || direction == self.settling_direction);
            self.settling_direction = direction;
            if sampled < acknowledged.saturating_add(2000) || prior.is_none() || moving {
                if now - acknowledged >= settle_limit {
                    self.restore(now, Some("rpm_settling_timeout"));
                }
                return self.output();
            }
            // A reversal is measured too: oscillation must still fail the
            // unchanged five-sample stability check below.
            self.sampling_began = Some(now);
        }
        self.output.phase = Sampling;
        self.readings.push(rpm.unwrap());
        if self.readings.len() < 5 {
            return self.output();
        }
        let duty = self.duties[self.stage];
        let rising = self.stage >= self.descending;
        let minimum = self.readings.iter().copied().fold(f32::INFINITY, f32::min);
        let maximum = self.readings.iter().copied().fold(0.0, f32::max);
        // A stall stage must end stopped: spinning down or up is not a verdict.
        let stalled = minimum == 0.0 && self.readings.last() == Some(&0.0);
        if (minimum == 0.0 && !stalled) || (!stalled && minimum < maximum * 0.8) {
            self.restore(now, Some("unstable_rpm"));
            return self.output();
        }
        let mean = self.readings.iter().sum::<f32>() / self.readings.len() as f32;
        let mut finished = false;
        if stalled {
            if !rising {
                // Stall found: probe upwards from here to the last running duty.
                self.stall_percent = Some(duty);
                let running = self.stage.checked_sub(1).map(|s| self.duties[s]);
                self.duties.truncate(self.stage + 1);
                self.descending = self.stage;
                if let Some(running) = running {
                    // At most five probes between the stall and the last running duty.
                    let step = ((running - duty) / 5).max(1);
                    let mut next = duty.saturating_add(step);
                    while next < running {
                        self.duties.push(next);
                        next = next.saturating_add(step);
                    }
                    self.duties.push(running);
                }
            }
        } else if rising {
            self.start_percent = Some(duty);
            finished = true;
        } else {
            if self.means.last().is_some_and(|prior| mean > prior * 1.1) {
                self.restore(now, Some("inconsistent_response"));
                return self.output();
            }
            self.means.push(mean);
        }
        self.stage += 1;
        if self.stage == self.duties.len() || finished {
            if self.stall_percent.is_some() && self.start_percent.is_none() && stalled {
                // Never restarted below the last running duty: use that one.
                self.start_percent = self
                    .descending
                    .checked_sub(1)
                    .and_then(|i| self.duties.get(i))
                    .copied();
            }
            let response = self
                .means
                .last()
                .is_some_and(|last| *last <= self.means[0] * 0.9)
                || self.stall_percent.is_some();
            let lowest = self
                .duties
                .get(self.means.len().saturating_sub(1))
                .copied()
                .unwrap_or(100);
            let suggested = if self.probe_stall {
                self.start_percent
                    .map(|start| start.max(self.stall_percent.unwrap_or(0).saturating_add(1)))
                    .or(response.then_some(lowest))
            } else {
                response.then_some(lowest.max(50))
            };
            self.pending_result = Some(CalibrationResult {
                response_observed: response,
                lowest_tested_percent: lowest,
                suggested_floor_percent: suggested,
                hardware_verified: false,
                tested: self
                    .duties
                    .iter()
                    .zip(&self.means)
                    .map(|(&duty_percent, &rpm)| TestedDuty { duty_percent, rpm })
                    .collect(),
                monotonic_response: self.means.len() >= 2
                    && self
                        .means
                        .windows(2)
                        .all(|w| w[1] <= w[0] * self.monotonic_ratio),
                stall_percent: self.stall_percent,
                start_percent: self.start_percent,
            });
            self.restore(now, None);
        } else {
            self.output.phase = Applying;
            self.output.sequence += 1;
            self.output.duty_percent = self.duties[self.stage];
            self.issued = now;
            self.acknowledged = None;
            self.last_sample = None;
            self.settling_rpm = None;
            self.settling_direction = 0;
            self.sampling_began = None;
            self.readings.clear();
        }
        self.output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(at_ms: u64, sequence: u64, rpm: f32) -> CalibrationSample {
        CalibrationSample {
            at_ms,
            applied_sequence: Some(sequence),
            rpm: Some(rpm),
            rpm_age_ms: 0,
            temperature_c: Some(40.0),
            temperature_age_ms: 0,
        }
    }
    #[test]
    fn bounded_response_test_restores_full_speed_before_completion() {
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        let mut now = 0;
        for duty in DUTIES {
            assert_eq!(c.output().duty_percent, duty);
            let seq = c.output().sequence;
            c.update(sample(now, seq, f32::from(duty) * 20.0));
            now += 2000;
            for _ in 0..5 {
                c.update(sample(now, seq, f32::from(duty) * 20.0));
                now += 250;
            }
        }
        assert_eq!(c.output().phase, CalibrationPhase::Restoring);
        assert_eq!(c.output().duty_percent, 100);
        let out = c.update(sample(now, c.output().sequence, 2000.0));
        assert_eq!(out.phase, CalibrationPhase::Completed);
        let result = out.result.unwrap();
        assert_eq!(result.suggested_floor_percent, Some(50));
        assert!(!result.hardware_verified);
        assert!(Calibration::new(ChannelRole::Pump, 85.0, 0).is_err());
    }
    fn run_test(rpm_at: impl Fn(u8) -> f32) -> CalibrationResult {
        let mut c = Calibration::channel_test(ChannelRole::Fan, 85.0, 0).unwrap();
        let mut now = 0;
        for duty in CHANNEL_TEST_DUTIES {
            assert_eq!(c.output().duty_percent, duty);
            assert!(c.output().duty_percent >= 40);
            let seq = c.output().sequence;
            c.update(sample(now, seq, rpm_at(duty)));
            now += 2000;
            for _ in 0..5 {
                c.update(sample(now, seq, rpm_at(duty)));
                now += 250;
            }
        }
        assert_eq!(c.output().duty_percent, 100);
        let out = c.update(sample(now, c.output().sequence, 2000.0));
        assert_eq!(out.phase, CalibrationPhase::Completed);
        out.result.unwrap()
    }
    #[test]
    fn channel_test_records_rpm_per_duty_and_monotonic_response() {
        let good = run_test(|d| f32::from(d) * 15.0);
        assert!(good.monotonic_response && good.response_observed);
        assert_eq!(good.lowest_tested_percent, 50);
        assert_eq!(
            good.tested
                .iter()
                .map(|t| t.duty_percent)
                .collect::<Vec<_>>(),
            vec![100, 70, 50]
        );
        assert_eq!(good.tested[1].rpm, 1050.0);
        let flat = run_test(|_| 1200.0);
        assert!(!flat.monotonic_response && !flat.response_observed);
        assert!(Calibration::channel_test(ChannelRole::Pump, 85.0, 0).is_err());
    }
    #[test]
    fn sensor_fault_stall_critical_and_lost_ack_request_full_speed() {
        for case in 0..5 {
            let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
            let mut s = sample(2000, 1, 1000.0);
            match case {
                0 => s.rpm = Some(0.0),
                1 => s.temperature_c = Some(85.0),
                2 => s.rpm_age_ms = 1501,
                3 => s.temperature_c = None,
                _ => s.applied_sequence = None,
            }
            let out = c.update(s);
            assert_eq!(out.phase, CalibrationPhase::Restoring);
            assert_eq!(out.duty_percent, 100);
            assert_eq!(
                c.update(sample(3001, out.sequence, 1000.0)).phase,
                CalibrationPhase::RestorationFailed
            );
        }
    }
    #[test]
    fn cached_reading_is_not_counted_repeatedly_and_cancel_requires_restore_ack() {
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        c.update(sample(0, 1, 2000.0));
        c.update(sample(2000, 1, 2000.0));
        for at in [2250, 2500, 2750, 3000] {
            let mut s = sample(at, 1, 2000.0);
            s.rpm_age_ms = at - 2000;
            c.update(s);
        }
        assert_eq!(c.output().sequence, 1);
        let out = c.cancel(3000);
        assert_eq!(out.phase, CalibrationPhase::Restoring);
        assert_eq!(
            c.update(sample(3000, out.sequence, 2000.0)).phase,
            CalibrationPhase::Aborted
        );
    }
    #[test]
    fn late_ack_clock_reversal_and_unstable_samples_never_complete_successfully() {
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        assert_eq!(
            c.update(sample(1001, 1, 2000.0)).reason.as_deref(),
            Some("duty_acknowledgment_timeout")
        );
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        c.update(sample(100, 1, 2000.0));
        assert_eq!(
            c.update(sample(99, 1, 2000.0)).phase,
            CalibrationPhase::Restoring
        );
        assert_eq!(
            c.update(sample(98, 2, 2000.0)).phase,
            CalibrationPhase::RestorationFailed
        );
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        c.update(sample(0, 1, 2000.0));
        for (index, rpm) in [2000.0, 1000.0, 2000.0, 1000.0, 2000.0]
            .into_iter()
            .enumerate()
        {
            c.update(sample(2000 + index as u64 * 250, 1, rpm));
        }
        assert_eq!(c.output().reason.as_deref(), Some("unstable_rpm"));
        assert_eq!(c.output().duty_percent, 100);
        assert!(Calibration::new(ChannelRole::Fan, 85.0, u64::MAX).is_err());
    }
    #[test]
    fn slow_owner_ramp_settles_before_measuring_full_speed() {
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        for at in (0..=6500).step_by(250) {
            let rpm = 830.0 + 860.0 * (at as f32 / 5000.0).min(1.0);
            let out = c.update(sample(at, 1, rpm));
            assert!(out.reason.is_none(), "{out:?}");
            if at <= 5000 {
                assert_eq!(out.phase, CalibrationPhase::Settling);
            }
            if out.sequence != 1 {
                break;
            }
        }
        assert_eq!(c.output().phase, CalibrationPhase::Applying);
        assert_eq!(c.output().duty_percent, 80);
        assert_eq!(c.means, vec![1690.0]);
    }

    #[test]
    fn endless_ramp_hits_settling_deadline_and_requires_restore_ack() {
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        for at in (0..=15_000).step_by(250) {
            c.update(sample(at, 1, 830.0 + at as f32 * 0.5));
        }
        let out = c.output();
        assert_eq!(out.phase, CalibrationPhase::Restoring);
        assert_eq!(out.reason.as_deref(), Some("rpm_settling_timeout"));
        assert_eq!(out.duty_percent, 100);
        let out = c.update(sample(15_250, out.sequence, 2000.0));
        assert_eq!(out.phase, CalibrationPhase::Aborted);
        assert!(out.result.is_none());
    }

    #[test]
    fn oscillation_after_a_ramp_still_fails_measurement_stability() {
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        for at in (0..=3000).step_by(250) {
            c.update(sample(at, 1, 830.0 + at as f32 * 0.2));
        }
        for (i, rpm) in [1000.0, 1700.0, 1000.0, 1700.0, 1000.0]
            .into_iter()
            .enumerate()
        {
            c.update(sample(3250 + i as u64 * 250, 1, rpm));
        }
        assert_eq!(c.output().phase, CalibrationPhase::Restoring);
        assert_eq!(c.output().reason.as_deref(), Some("unstable_rpm"));
    }
    #[test]
    fn constant_rpm_does_not_produce_a_floor_recommendation() {
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        let mut now = 0;
        for _ in DUTIES {
            let seq = c.output().sequence;
            c.update(sample(now, seq, 2000.0));
            now += 2000;
            for _ in 0..5 {
                c.update(sample(now, seq, 2000.0));
                now += 250;
            }
        }
        assert!(c.output().result.is_none()); // restoration has not been acknowledged
        let out = c.update(sample(now, c.output().sequence, 2000.0));
        assert_eq!(out.phase, CalibrationPhase::Completed);
        let result = out.result.unwrap();
        assert!(!result.response_observed);
        assert_eq!(result.suggested_floor_percent, None);
        let request = CalibrationPreviewRequest {
            role: ChannelRole::Fan,
            critical_c: 85.0,
            samples: vec![sample(0, 1, 2000.0); 513],
        };
        assert!(request.run().is_err());
    }
    #[test]
    fn fault_during_final_restore_suppresses_result_without_extending_deadline() {
        let mut c = Calibration::new(ChannelRole::Fan, 85.0, 0).unwrap();
        let mut now = 0;
        for duty in DUTIES {
            let seq = c.output().sequence;
            c.update(sample(now, seq, f32::from(duty) * 20.0));
            now += 2000;
            for _ in 0..5 {
                c.update(sample(now, seq, f32::from(duty) * 20.0));
                now += 250;
            }
        }
        assert_eq!(c.output().phase, CalibrationPhase::Restoring);
        let mut s = sample(now, c.output().sequence, 2000.0);
        s.temperature_c = Some(90.0);
        let out = c.update(s);
        assert_eq!(out.phase, CalibrationPhase::Aborted);
        assert_eq!(out.reason.as_deref(), Some("critical_temperature"));
        assert!(out.result.is_none());
        assert_eq!(out.duty_percent, 100);
    }
}

#[cfg(test)]
mod sweep_tests {
    use super::*;

    /// Runs a sweep against a simulated fan with hysteresis: it stops below
    /// `stall`, and once stopped needs at least `start` to spin up again.
    fn simulate(options: SweepOptions, stall: u8, start: u8) -> CalibrationOutput {
        let mut c = Calibration::sweep(ChannelRole::Fan, 85.0, 0, options).unwrap();
        let mut stopped = false;
        let mut now = 0;
        for _ in 0..200 {
            let out = c.output();
            if matches!(
                out.phase,
                CalibrationPhase::Completed | CalibrationPhase::Aborted
            ) {
                return out;
            }
            let duty = out.duty_percent;
            stopped = if stopped { duty < start } else { duty < stall };
            let rpm = if stopped { 0.0 } else { f32::from(duty) * 20.0 };
            let s = |at| CalibrationSample {
                at_ms: at,
                applied_sequence: Some(out.sequence),
                rpm: Some(rpm),
                rpm_age_ms: 0,
                temperature_c: Some(40.0),
                temperature_age_ms: 0,
            };
            if out.phase == CalibrationPhase::Restoring {
                c.update(s(now));
                continue;
            }
            c.update(s(now));
            if c.output().sequence != out.sequence {
                continue;
            }
            now += 2000;
            for _ in 0..5 {
                if c.output().sequence != out.sequence {
                    break;
                }
                c.update(s(now));
                now += 250;
            }
        }
        panic!("sweep did not finish: {:?}", c.output());
    }

    #[test]
    fn fine_sweep_records_every_step_and_never_goes_below_floor() {
        let options = SweepOptions {
            step_percent: 5,
            lowest_percent: 40,
            probe_stall: false,
        };
        let c = Calibration::sweep(ChannelRole::Fan, 85.0, 0, options).unwrap();
        assert_eq!(c.duties.len(), 13);
        let out = simulate(options, 0, 0);
        assert_eq!(out.phase, CalibrationPhase::Completed);
        let result = out.result.unwrap();
        assert_eq!(result.tested.len(), 13);
        assert!(result.tested.iter().all(|t| t.duty_percent >= 40));
        assert!(result.monotonic_response);
        assert_eq!(result.stall_percent, None);
        assert!(Calibration::sweep(ChannelRole::Pump, 85.0, 0, options).is_err());
        for bad in [
            SweepOptions {
                step_percent: 1,
                ..options
            },
            SweepOptions {
                lowest_percent: 20,
                ..options
            },
        ] {
            assert!(bad.validate().is_err());
        }
    }

    #[test]
    fn stall_sweep_finds_stall_and_start_thresholds() {
        let options = SweepOptions {
            step_percent: 5,
            lowest_percent: 0,
            probe_stall: true,
        };
        // Stops below 22%, restarts only at >= 30%.
        let out = simulate(options, 22, 30);
        assert_eq!(out.phase, CalibrationPhase::Completed, "{:?}", out.reason);
        let result = out.result.unwrap();
        assert_eq!(result.stall_percent, Some(20));
        // Probe upwards from 20 in 1% steps up to the last running 25%:
        // 21..24 stay stopped, 25 is the fallback (never restarted below it).
        assert_eq!(result.start_percent, Some(25));
        assert_eq!(result.suggested_floor_percent, Some(25));
        assert!(result.tested.iter().all(|t| t.rpm >= 100.0));
        assert_eq!(result.tested.last().unwrap().duty_percent, 25);
        // A fan that restarts within the probe range reports that duty.
        let out = simulate(options, 22, 23);
        assert_eq!(out.result.unwrap().start_percent, Some(23));
        assert_eq!(out.duty_percent, 100);
    }

    #[test]
    fn stall_in_a_normal_sweep_is_a_fault() {
        let options = SweepOptions {
            step_percent: 10,
            lowest_percent: 40,
            probe_stall: false,
        };
        let out = simulate(options, 55, 60);
        assert_eq!(out.phase, CalibrationPhase::Aborted);
        assert_eq!(out.reason.as_deref(), Some("missing_or_stalled_rpm"));
    }
}
