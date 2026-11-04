//! Runtime RPM plausibility (COOLING_BROAD_SUPPORT_PLAN.md, safeguard 2).
//! Mirrors `RpmPlausibility` in `providers/mainboard/BackgroundCooling.cs`;
//! both must agree with `tests/fixtures/plausibility_vectors.json`.
//! Pure policy: the caller decides whether a deviation faults (LHM-supported)
//! or only warns (unrestricted), see `fan_support::ChannelLimits`.
use crate::calibration::TestedDuty;

/// Time after a duty change (>= 5 points) before deviations count.
pub const SETTLE_MS: u64 = 4000;
/// How long a deviation must persist before it is reported.
pub const PERSIST_MS: u64 = 5000;

#[derive(Clone, Debug)]
pub struct RpmPlausibility {
    /// (duty, rpm), duty strictly rising, rpm non-decreasing.
    points: Vec<(u8, f64)>,
    changed_at: Option<u64>,
    deviating_since: Option<u64>,
    last_duty: Option<u8>,
}

impl RpmPlausibility {
    pub fn new(tested: &[TestedDuty]) -> Result<Self, String> {
        let mut points: Vec<(u8, f64)> = tested
            .iter()
            .map(|t| (t.duty_percent, f64::from(t.rpm)))
            .collect();
        points.sort_by_key(|p| p.0);
        if !(2..=8).contains(&points.len()) {
            return Err("Plausibility needs 2..8 tested duties".into());
        }
        if points
            .iter()
            .any(|&(d, r)| !(40..=100).contains(&d) || !(100.0..=100_000.0).contains(&r))
        {
            return Err("Invalid tested duty/RPM".into());
        }
        if points
            .windows(2)
            .any(|w| w[1].0 <= w[0].0 || w[1].1 < w[0].1)
        {
            return Err("Tested RPM must rise with duty".into());
        }
        Ok(Self {
            points,
            changed_at: None,
            deviating_since: None,
            last_duty: None,
        })
    }

    /// Expected RPM inside the tested range; `None` below/above it.
    pub fn expected(&self, duty: u8) -> Option<f64> {
        let (first, last) = (self.points[0], self.points[self.points.len() - 1]);
        if duty < first.0 || duty > last.0 {
            return None;
        }
        self.points.windows(2).find(|w| duty <= w[1].0).map(|w| {
            let ((d0, r0), (d1, r1)) = (w[0], w[1]);
            r0 + f64::from(duty - d0) / f64::from(d1 - d0) * (r1 - r0)
        })
    }

    pub fn within(expected: f64, rpm: f64) -> bool {
        rpm >= expected * 0.6 - 150.0 && rpm <= expected * 1.4 + 150.0
    }

    /// `true` once a deviation persisted (`rpm_implausible`).
    pub fn observe(&mut self, duty: u8, rpm: f64, now_ms: u64) -> bool {
        if self.last_duty != Some(duty) {
            if self.last_duty.is_none_or(|last| last.abs_diff(duty) >= 5) {
                self.changed_at = Some(now_ms);
            }
            self.last_duty = Some(duty);
        }
        let settled = self
            .changed_at
            .is_some_and(|at| now_ms.saturating_sub(at) >= SETTLE_MS);
        let deviating = settled
            && self
                .expected(duty)
                .is_some_and(|expected| !Self::within(expected, rpm));
        if !deviating {
            self.deviating_since = None;
            return false;
        }
        let since = *self.deviating_since.get_or_insert(now_ms);
        now_ms - since >= PERSIST_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tested() -> Vec<TestedDuty> {
        [(100, 1500.0), (70, 1100.0), (50, 800.0)]
            .into_iter()
            .map(|(duty_percent, rpm)| TestedDuty { duty_percent, rpm })
            .collect()
    }

    #[test]
    fn interpolates_inside_the_tested_range_only() {
        let p = RpmPlausibility::new(&tested()).unwrap();
        assert_eq!(p.expected(60), Some(950.0));
        assert_eq!(p.expected(40), None);
        assert!(RpmPlausibility::new(&tested()[..1]).is_err());
        let falling =
            [(50, 900.0), (100, 800.0)].map(|(duty_percent, rpm)| TestedDuty { duty_percent, rpm });
        assert!(RpmPlausibility::new(&falling).is_err());
    }
}
