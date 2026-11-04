//! RPM targets from measured duty/RPM pairs (channel test or calibration).
//! Open-loop inversion only: a target is translated to the duty that produced
//! that speed during the measurement. Targets below the slowest measured point
//! are refused (no extrapolation towards a stall); targets above the fastest
//! measured point give 100%.
use crate::calibration::TestedDuty;

#[derive(Clone, Debug, PartialEq)]
pub struct RpmMap {
    /// (duty, rpm) sorted by duty, strictly increasing in both.
    points: Vec<(f32, f32)>,
}

impl RpmMap {
    pub fn from_tested(tested: &[TestedDuty]) -> Result<Self, String> {
        let mut points: Vec<(f32, f32)> = tested
            .iter()
            .map(|t| (f32::from(t.duty_percent), t.rpm))
            .collect();
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        points.dedup_by(|a, b| a.0 == b.0);
        if points.len() < 2 {
            return Err("Run the channel test or calibration first: RPM targets need at least two measured speeds".into());
        }
        if points
            .iter()
            .any(|(d, r)| !(0.0..=100.0).contains(d) || !r.is_finite() || *r < 100.0)
        {
            return Err("Measured speeds are implausible; run the calibration again".into());
        }
        if points.windows(2).any(|w| w[1].1 <= w[0].1) {
            return Err("Measured speeds do not rise with duty; run the calibration again".into());
        }
        Ok(Self { points })
    }

    pub fn min_rpm(&self) -> f32 {
        self.points[0].1
    }

    pub fn max_rpm(&self) -> f32 {
        self.points[self.points.len() - 1].1
    }

    /// Duty in percent for `target` RPM.
    pub fn duty_for(&self, target: f32) -> Result<f32, String> {
        if !target.is_finite() || target <= 0.0 {
            return Err("RPM target must be positive".into());
        }
        if target < self.min_rpm() {
            return Err(format!(
                "Lowest measured speed is {:.0} RPM; lower targets are not extrapolated",
                self.min_rpm()
            ));
        }
        if target >= self.max_rpm() {
            return Ok(100.0);
        }
        for w in self.points.windows(2) {
            let ((d0, r0), (d1, r1)) = (w[0], w[1]);
            if target <= r1 {
                return Ok(d0 + (target - r0) / (r1 - r0) * (d1 - d0));
            }
        }
        Ok(100.0)
    }
}

/// PI gains for closed-loop RPM targets.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PiGains {
    /// Percent duty per RPM of error.
    pub kp: f32,
    /// Percent duty per RPM-second of accumulated error.
    pub ki: f32,
    /// Bound of the integral correction (anti-windup), percent.
    pub max_correction_percent: f32,
}
impl Default for PiGains {
    fn default() -> Self {
        Self {
            kp: 0.01,
            ki: 0.02,
            max_correction_percent: 20.0,
        }
    }
}
impl PiGains {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.kp.is_finite() && (0.0..=0.2).contains(&self.kp))
            || !(self.ki.is_finite() && (0.0..=0.5).contains(&self.ki))
            || !(self.max_correction_percent.is_finite()
                && (0.0..=50.0).contains(&self.max_correction_percent))
        {
            return Err("PI gains: kp 0..0.2, ki 0..0.5, correction 0..50 percent".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RpmLoopOutput {
    pub duty_percent: f32,
    /// False while running open loop (bad/stale telemetry, first sample).
    pub closed_loop: bool,
}

/// Closed-loop correction on top of [`RpmMap::duty_for`]: the open-loop duty
/// plus proportional and bounded integral terms. Bad, stale or stalled RPM
/// telemetry drops back to the open-loop duty and clears the integral.
#[derive(Debug, Default)]
pub struct RpmPi {
    integral: f32,
    last_ms: Option<u64>,
}
impl RpmPi {
    pub fn update(
        &mut self,
        map: &RpmMap,
        gains: PiGains,
        target: f32,
        rpm: Option<f32>,
        rpm_age_ms: u64,
        now_ms: u64,
    ) -> RpmLoopOutput {
        let base = map.duty_for(target.max(map.min_rpm())).unwrap_or(100.0);
        let measured = rpm.filter(|r| r.is_finite() && (100.0..=100_000.0).contains(r));
        let reversed = self.last_ms.is_some_and(|last| now_ms < last);
        let (Some(rpm), false, true, true) = (
            measured,
            reversed,
            rpm_age_ms <= 1500,
            gains.validate().is_ok(),
        ) else {
            self.integral = 0.0;
            self.last_ms = None;
            return RpmLoopOutput {
                duty_percent: base,
                closed_loop: false,
            };
        };
        let error = target - rpm;
        let seconds = self
            .last_ms
            .map_or(0.0, |last| (now_ms - last).min(2000) as f32 / 1000.0);
        self.last_ms = Some(now_ms);
        let proportional = gains.kp * error;
        let unclamped = base + proportional + self.integral;
        // Conditional integration: do not wind up against a saturated output.
        let saturated = (unclamped >= 100.0 && error > 0.0) || (unclamped <= 0.0 && error < 0.0);
        if !saturated {
            self.integral = (self.integral + gains.ki * error * seconds)
                .clamp(-gains.max_correction_percent, gains.max_correction_percent);
        }
        RpmLoopOutput {
            duty_percent: (base + proportional + self.integral).clamp(0.0, 100.0),
            closed_loop: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_converges_on_a_drifted_fan_and_falls_back_on_bad_telemetry() {
        let map = RpmMap::from_tested(&tested(&[(100, 2000.0), (40, 800.0)])).unwrap();
        // The real fan runs 10% slower than measured: rpm = 0.9 * map(duty).
        let plant = |duty: f32| 0.9 * (800.0 + (duty - 40.0) / 60.0 * 1200.0);
        let mut pi = RpmPi::default();
        let gains = PiGains::default();
        let open = map.duty_for(1400.0).unwrap();
        let mut duty = open;
        for step in 0..200u64 {
            let out = pi.update(&map, gains, 1400.0, Some(plant(duty)), 0, step * 500);
            assert!(out.closed_loop);
            duty = out.duty_percent;
        }
        assert!((plant(duty) - 1400.0).abs() < 15.0, "rpm {}", plant(duty));
        assert!(duty > open);
        // Stale, stalled or missing telemetry: open loop, integral cleared.
        for (rpm, age) in [(Some(1300.0), 1501), (Some(0.0), 0), (None, 0)] {
            let out = pi.update(&map, gains, 1400.0, rpm, age, 200_000);
            assert_eq!(out.duty_percent, open);
            assert!(!out.closed_loop);
        }
        let first = pi.update(&map, gains, 1400.0, Some(1400.0), 0, 200_500);
        assert_eq!(first.duty_percent, open);
    }

    #[test]
    fn pi_integral_is_bounded_and_does_not_wind_up_at_full_speed() {
        let map = RpmMap::from_tested(&tested(&[(100, 2000.0), (40, 800.0)])).unwrap();
        let mut pi = RpmPi::default();
        let gains = PiGains::default();
        // A blocked fan never reaches the target: output saturates at 100%.
        for step in 0..1000u64 {
            pi.update(&map, gains, 1900.0, Some(500.0), 0, step * 500);
        }
        assert!(pi.integral <= gains.max_correction_percent);
        // Once the fan is free the correction unwinds from a bounded value.
        let out = pi.update(&map, gains, 1900.0, Some(1900.0), 0, 500_500);
        assert!(out.duty_percent <= map.duty_for(1900.0).unwrap() + gains.max_correction_percent);
        assert!(PiGains {
            kp: f32::NAN,
            ..gains
        }
        .validate()
        .is_err());
    }

    fn tested(pairs: &[(u8, f32)]) -> Vec<TestedDuty> {
        pairs
            .iter()
            .map(|&(duty_percent, rpm)| TestedDuty { duty_percent, rpm })
            .collect()
    }

    #[test]
    fn interpolates_between_measured_points() {
        let map =
            RpmMap::from_tested(&tested(&[(100, 1500.0), (70, 1100.0), (50, 800.0)])).unwrap();
        assert_eq!(map.duty_for(800.0).unwrap(), 50.0);
        assert_eq!(map.duty_for(950.0).unwrap(), 60.0);
        assert_eq!(map.duty_for(1300.0).unwrap(), 85.0);
        assert_eq!(map.duty_for(2000.0).unwrap(), 100.0);
    }

    #[test]
    fn refuses_extrapolation_and_bad_measurements() {
        let map = RpmMap::from_tested(&tested(&[(100, 1500.0), (50, 800.0)])).unwrap();
        assert!(map.duty_for(700.0).is_err());
        assert!(map.duty_for(f32::NAN).is_err());
        assert!(RpmMap::from_tested(&tested(&[(100, 1500.0)])).is_err());
        assert!(RpmMap::from_tested(&tested(&[(100, 900.0), (50, 1000.0)])).is_err());
        assert!(RpmMap::from_tested(&tested(&[(100, 900.0), (50, 50.0)])).is_err());
    }
}
