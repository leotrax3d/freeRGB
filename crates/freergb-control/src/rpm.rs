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

#[cfg(test)]
mod tests {
    use super::*;

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
