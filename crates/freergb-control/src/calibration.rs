//! Bounded fan response experiment, reusable by a future isolated control backend.
//! This model requests duties; it cannot establish wiring identity or write success.
use crate::channels::ChannelRole;
use serde::{Deserialize, Serialize};

const DUTIES: [u8; 4] = [100, 80, 60, 40];

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
    stage: usize,
    readings: Vec<f32>,
    means: Vec<f32>,
    critical_c: f32,
    pending_result: Option<CalibrationResult>,
}
impl Calibration {
    pub fn new(role: ChannelRole, critical_c: f32, now_ms: u64) -> Result<Self, String> {
        if role != ChannelRole::Fan {
            return Err("Automatic duty-reduction calibration excludes pumps".into());
        }
        if !critical_c.is_finite() || !(40.0..=100.0).contains(&critical_c) {
            return Err("Critical temperature must be 40..100 Celsius".into());
        }
        now_ms
            .checked_add(60_000)
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
            stage: 0,
            readings: Vec::new(),
            means: Vec::new(),
            critical_c,
            pending_result: None,
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
        let rpm = sample
            .rpm
            .filter(|r| r.is_finite() && (100.0..=100_000.0).contains(r));
        let temp = sample
            .temperature_c
            .filter(|t| t.is_finite() && (-20.0..=150.0).contains(t));
        let fault = if now - self.began > 60_000 {
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
        if now - acknowledged > 10_000 {
            self.restore(now, Some("unstable_or_missing_samples"));
            return self.output();
        }
        if now - acknowledged < 2000 {
            self.output.phase = Settling;
            return self.output();
        }
        self.output.phase = Sampling;
        let sampled = now - sample.rpm_age_ms;
        if sampled < acknowledged.saturating_add(2000)
            || self
                .last_sample
                .is_some_and(|last| sampled < last.saturating_add(250))
        {
            return self.output();
        }
        self.last_sample = Some(sampled);
        self.readings.push(rpm.unwrap());
        if self.readings.len() < 5 {
            return self.output();
        }
        let minimum = self.readings.iter().copied().fold(f32::INFINITY, f32::min);
        let maximum = self.readings.iter().copied().fold(0.0, f32::max);
        if minimum < maximum * 0.8 {
            self.restore(now, Some("unstable_rpm"));
            return self.output();
        }
        let mean = self.readings.iter().sum::<f32>() / self.readings.len() as f32;
        if self.means.last().is_some_and(|prior| mean > prior * 1.1) {
            self.restore(now, Some("inconsistent_response"));
            return self.output();
        }
        self.means.push(mean);
        self.stage += 1;
        if self.stage == DUTIES.len() {
            let response = mean <= self.means[0] * 0.9;
            self.pending_result = Some(CalibrationResult {
                response_observed: response,
                lowest_tested_percent: 40,
                suggested_floor_percent: response.then_some(50),
                hardware_verified: false,
            });
            self.restore(now, None);
        } else {
            self.output.phase = Applying;
            self.output.sequence += 1;
            self.output.duty_percent = DUTIES[self.stage];
            self.issued = now;
            self.acknowledged = None;
            self.last_sample = None;
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
