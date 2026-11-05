//! Per-channel output shaping applied on top of any curve, manual duty or RPM
//! target: duty offset, per-device min/max, response delay, up/down hysteresis,
//! optional fan stop, resonance avoidance and spin-up boost. Pure policy.
//!
//! Order (each step sees the previous result):
//! 1. fail-safe input (stale/missing/critical) -> 100 %, state reset, nothing else applies;
//! 2. `input + offset_percent`, clamped to `min_percent..=max_percent` and 0..100;
//! 3. hysteresis and delay against the currently held value: a change is a
//!    candidate when it rises by at least `hysteresis_up_percent` or falls by at
//!    least `hysteresis_down_percent` (exactly the threshold counts; a zero
//!    threshold means any change). A candidate is applied once it has been a
//!    candidate continuously for `response_delay_ms` (elapsed `>=` delay); the
//!    latest candidate value is applied. A non-candidate sample restarts the delay;
//! 4. fan stop: held value strictly below `stop_below_percent` -> 0 %, only when
//!    the channel's tier allows stopping (never for pumps; see `validate_for`);
//! 5. safety floor: any non-stopped output is raised to the channel floor;
//! 6. resonance avoidance: an output strictly inside an avoided range moves to
//!    the nearer boundary (ties and boundaries below the floor go up);
//! 7. spin-up boost: leaving 0 % or the stored stall zone holds at least
//!    `spin_up.percent` for `spin_up.ms`.
use serde::{Deserialize, Serialize};

const MAX_RANGES: usize = 8;
const MAX_DELAY_MS: u64 = 60_000;
const MAX_SPIN_UP_MS: u64 = 10_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvoidUnit {
    #[default]
    Percent,
    /// Resolved to duty through the channel's measured RPM map before control.
    Rpm,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvoidRange {
    pub from: f32,
    pub to: f32,
    #[serde(default)]
    pub unit: AvoidUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpinUp {
    pub percent: f32,
    pub ms: u64,
}

fn hundred() -> f32 {
    100.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputShaping {
    #[serde(default)]
    pub offset_percent: f32,
    #[serde(default)]
    pub min_percent: f32,
    #[serde(default = "hundred")]
    pub max_percent: f32,
    #[serde(default)]
    pub response_delay_ms: u64,
    #[serde(default)]
    pub hysteresis_up_percent: f32,
    #[serde(default)]
    pub hysteresis_down_percent: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub avoid: Vec<AvoidRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_below_percent: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spin_up: Option<SpinUp>,
    /// Stall duty from this channel's stored calibration (daemon-owned, never
    /// from JSON). Leaving this zone also triggers the spin-up boost.
    #[serde(skip)]
    pub stall_percent: Option<f32>,
}

impl Default for OutputShaping {
    fn default() -> Self {
        Self {
            offset_percent: 0.0,
            min_percent: 0.0,
            max_percent: 100.0,
            response_delay_ms: 0,
            hysteresis_up_percent: 0.0,
            hysteresis_down_percent: 0.0,
            avoid: Vec::new(),
            stop_below_percent: None,
            spin_up: None,
            stall_percent: None,
        }
    }
}

fn percent(p: f32) -> bool {
    p.is_finite() && (0.0..=100.0).contains(&p)
}

impl OutputShaping {
    pub fn validate(&self) -> Result<(), String> {
        if !self.offset_percent.is_finite() || self.offset_percent.abs() > 100.0 {
            return Err("offset must be -100..100 percent".into());
        }
        if !percent(self.min_percent)
            || !percent(self.max_percent)
            || self.min_percent > self.max_percent
        {
            return Err("need 0 <= min <= max <= 100 percent".into());
        }
        if self.response_delay_ms > MAX_DELAY_MS {
            return Err("response delay must be at most 60 s".into());
        }
        for h in [self.hysteresis_up_percent, self.hysteresis_down_percent] {
            if !h.is_finite() || !(0.0..=50.0).contains(&h) {
                return Err("hysteresis must be 0..50 percent".into());
            }
        }
        if self.avoid.len() > MAX_RANGES {
            return Err("at most 8 avoided ranges".into());
        }
        for range in &self.avoid {
            let bound = match range.unit {
                AvoidUnit::Percent => 100.0,
                AvoidUnit::Rpm => 100_000.0,
            };
            if !(range.from.is_finite() && range.to.is_finite())
                || range.from < 0.0
                || range.to > bound
                || range.from >= range.to
            {
                return Err("avoided range needs 0 <= from < to".into());
            }
        }
        if let Some(stop) = self.stop_below_percent {
            if !stop.is_finite() || !(1.0..=50.0).contains(&stop) {
                return Err("fan stop threshold must be 1..50 percent".into());
            }
        }
        if let Some(spin) = self.spin_up {
            if !percent(spin.percent) || spin.ms == 0 || spin.ms > MAX_SPIN_UP_MS {
                return Err("spin-up needs 0..100 percent for 1..10000 ms".into());
            }
        }
        Ok(())
    }

    /// `validate` plus the channel's tier: fan stop only where the tier allows
    /// stopping and never on pumps; RPM ranges must have been resolved.
    pub fn validate_for(&self, allow_stop: bool, pump: bool) -> Result<(), String> {
        self.validate()?;
        if self.stop_below_percent.is_some() && (pump || !allow_stop) {
            return Err(if pump {
                "Pumps never stop".into()
            } else {
                "Fan stop needs an unrestricted channel".into()
            });
        }
        if self.avoid.iter().any(|r| r.unit == AvoidUnit::Rpm) {
            return Err("RPM ranges need this channel's measured speed map".into());
        }
        Ok(())
    }

    /// Converts RPM avoidance ranges to duty with `duty_for` (the measured map).
    pub fn resolve_rpm(
        &mut self,
        duty_for: impl Fn(f32) -> Result<f32, String>,
    ) -> Result<(), String> {
        for range in &mut self.avoid {
            if range.unit == AvoidUnit::Rpm {
                let from = duty_for(range.from).unwrap_or(0.0);
                let to = duty_for(range.to)?;
                *range = AvoidRange {
                    from,
                    to,
                    unit: AvoidUnit::Percent,
                };
                if from >= to {
                    return Err("avoided RPM range is narrower than the measured map".into());
                }
            }
        }
        Ok(())
    }
}

/// Safety facts of the channel (from the admitted tier), never user input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelLimits {
    pub floor_percent: f32,
    pub allow_stop: bool,
}

#[derive(Debug, Default)]
pub struct OutputShaper {
    held: Option<f32>,
    pending_since: Option<u64>,
    last_out: Option<f32>,
    boost_until: Option<u64>,
    last_ms: Option<u64>,
}

impl OutputShaper {
    pub fn apply(
        &mut self,
        cfg: &OutputShaping,
        input: f32,
        failsafe: bool,
        limits: ChannelLimits,
        now_ms: u64,
    ) -> f32 {
        let reversed = self.last_ms.is_some_and(|last| now_ms < last);
        self.last_ms = Some(now_ms);
        if failsafe || reversed || !input.is_finite() || cfg.validate().is_err() {
            *self = Self {
                last_ms: Some(now_ms),
                last_out: Some(100.0),
                ..Self::default()
            };
            return 100.0;
        }
        let value = (input + cfg.offset_percent)
            .clamp(cfg.min_percent, cfg.max_percent)
            .clamp(0.0, 100.0);
        let held = match self.held {
            None => value,
            Some(held) => {
                let delta = value - held;
                let candidate = (delta > 0.0 && delta >= cfg.hysteresis_up_percent)
                    || (delta < 0.0 && -delta >= cfg.hysteresis_down_percent);
                if !candidate {
                    self.pending_since = None;
                    held
                } else {
                    let since = *self.pending_since.get_or_insert(now_ms);
                    if now_ms - since >= cfg.response_delay_ms {
                        self.pending_since = None;
                        value
                    } else {
                        held
                    }
                }
            }
        };
        self.held = Some(held);
        let stopped = limits.allow_stop && cfg.stop_below_percent.is_some_and(|stop| held < stop);
        let mut out = if stopped {
            0.0
        } else {
            held.max(limits.floor_percent).min(100.0)
        };
        if out > 0.0 {
            out = avoid(&cfg.avoid, out, limits.floor_percent);
            let was_still = self
                .last_out
                .is_some_and(|last| last <= 0.0 || cfg.stall_percent.is_some_and(|s| last <= s));
            if let Some(spin) = cfg.spin_up {
                if was_still && self.boost_until.is_none_or(|until| now_ms >= until) {
                    self.boost_until = Some(now_ms.saturating_add(spin.ms));
                }
                if self.boost_until.is_some_and(|until| now_ms < until) {
                    out = out.max(spin.percent);
                }
            }
        } else {
            self.boost_until = None;
        }
        self.last_out = Some(out);
        out
    }
}

fn avoid(ranges: &[AvoidRange], value: f32, floor: f32) -> f32 {
    // Overlapping or touching ranges merge into one, so a snap never lands
    // inside another range.
    let mut sorted: Vec<(f32, f32)> = ranges.iter().map(|r| (r.from, r.to)).collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f32, f32)> = Vec::new();
    for (from, to) in sorted {
        match merged.last_mut() {
            Some(last) if from <= last.1 => last.1 = last.1.max(to),
            _ => merged.push((from, to)),
        }
    }
    for (from, to) in merged {
        if value > from && value < to {
            let up = value - from >= to - value;
            return if up || from < floor {
                to.min(100.0)
            } else {
                from
            };
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAN: ChannelLimits = ChannelLimits {
        floor_percent: 40.0,
        allow_stop: false,
    };
    const FREE: ChannelLimits = ChannelLimits {
        floor_percent: 0.0,
        allow_stop: true,
    };

    fn run(cfg: &OutputShaping, limits: ChannelLimits, steps: &[(u64, f32)]) -> Vec<f32> {
        let mut s = OutputShaper::default();
        steps
            .iter()
            .map(|&(t, v)| s.apply(cfg, v, false, limits, t))
            .collect()
    }

    #[test]
    fn offset_and_min_max_apply_before_the_floor() {
        let cfg = OutputShaping {
            offset_percent: 10.0,
            min_percent: 50.0,
            max_percent: 80.0,
            ..OutputShaping::default()
        };
        assert_eq!(run(&cfg, FAN, &[(0, 20.0)]), [50.0]);
        assert_eq!(run(&cfg, FAN, &[(0, 60.0)]), [70.0]);
        assert_eq!(run(&cfg, FAN, &[(0, 95.0)]), [80.0]);
        let low = OutputShaping {
            offset_percent: -30.0,
            ..OutputShaping::default()
        };
        assert_eq!(run(&low, FAN, &[(0, 50.0)]), [40.0]); // floor wins
    }

    #[test]
    fn failsafe_bypasses_max_and_resets_state() {
        let cfg = OutputShaping {
            max_percent: 60.0,
            response_delay_ms: 5000,
            ..OutputShaping::default()
        };
        let mut s = OutputShaper::default();
        assert_eq!(s.apply(&cfg, 50.0, false, FAN, 0), 50.0);
        assert_eq!(s.apply(&cfg, 50.0, true, FAN, 100), 100.0);
        // After a fault the first healthy value applies without delay.
        assert_eq!(s.apply(&cfg, 45.0, false, FAN, 200), 45.0);
        assert_eq!(s.apply(&cfg, 45.0, false, FAN, 100), 100.0); // clock reversal
    }

    #[test]
    fn hysteresis_boundaries_are_inclusive() {
        let cfg = OutputShaping {
            hysteresis_up_percent: 5.0,
            hysteresis_down_percent: 10.0,
            ..OutputShaping::default()
        };
        let out = run(
            &cfg,
            FREE,
            &[
                (0, 50.0),
                (1, 54.9),
                (2, 55.0), // exactly +5 applies
                (3, 45.1),
                (4, 45.0), // exactly -10 applies
            ],
        );
        assert_eq!(out, [50.0, 50.0, 55.0, 55.0, 45.0]);
    }

    #[test]
    fn delay_applies_after_continuous_candidate_and_restarts_on_interruption() {
        let cfg = OutputShaping {
            response_delay_ms: 2000,
            hysteresis_up_percent: 2.0,
            hysteresis_down_percent: 2.0,
            ..OutputShaping::default()
        };
        let out = run(
            &cfg,
            FREE,
            &[
                (0, 50.0),
                (1000, 60.0), // candidate since 1000
                (2999, 62.0),
                (3000, 63.0), // elapsed == delay: latest value applies
                (4000, 70.0), // new candidate since 4000
                (5000, 64.0), // within hysteresis: restarts
                (6000, 70.0),
                (7999, 70.0),
                (8000, 70.0),
            ],
        );
        assert_eq!(out, [50.0, 50.0, 50.0, 63.0, 63.0, 63.0, 63.0, 63.0, 70.0]);
    }

    #[test]
    fn fan_stop_needs_permission_and_spin_up_boosts_restart() {
        let cfg = OutputShaping {
            stop_below_percent: Some(20.0),
            spin_up: Some(SpinUp {
                percent: 60.0,
                ms: 1000,
            }),
            ..OutputShaping::default()
        };
        assert!(cfg.validate_for(false, false).is_err());
        assert!(cfg.validate_for(true, true).is_err());
        cfg.validate_for(true, false).unwrap();
        let out = run(
            &cfg,
            FREE,
            &[
                (0, 30.0),    // first output: the fan was running before control
                (100, 10.0),  // stop
                (200, 25.0),  // restart -> boost
                (1199, 25.0), // still boosting
                (1200, 25.0), // boost over
            ],
        );
        assert_eq!(out, [30.0, 0.0, 60.0, 60.0, 25.0]);
        // Without permission the threshold never stops the fan.
        assert_eq!(run(&cfg, FAN, &[(0, 10.0)]), [40.0]);
    }

    #[test]
    fn leaving_the_stall_zone_boosts_too() {
        let cfg = OutputShaping {
            spin_up: Some(SpinUp {
                percent: 50.0,
                ms: 500,
            }),
            stall_percent: Some(15.0),
            ..OutputShaping::default()
        };
        let out = run(&cfg, FREE, &[(0, 10.0), (100, 30.0), (700, 30.0)]);
        assert_eq!(out, [10.0, 50.0, 30.0]);
    }

    #[test]
    fn resonance_ranges_snap_deterministically() {
        let cfg = OutputShaping {
            avoid: vec![
                AvoidRange {
                    from: 50.0,
                    to: 60.0,
                    unit: AvoidUnit::Percent,
                },
                AvoidRange {
                    from: 58.0,
                    to: 70.0,
                    unit: AvoidUnit::Percent,
                },
            ],
            ..OutputShaping::default()
        };
        let at = |v| run(&cfg, FAN, &[(0, v)])[0];
        assert_eq!(at(50.0), 50.0); // boundaries are allowed
        assert_eq!(at(54.0), 50.0);
        assert_eq!(at(55.0), 50.0); // overlapping ranges merge to 50..70
        assert_eq!(at(60.0), 70.0); // tie goes up
        assert_eq!(at(65.0), 70.0);
        // A lower boundary below the floor is never used.
        let low = OutputShaping {
            avoid: vec![AvoidRange {
                from: 30.0,
                to: 50.0,
                unit: AvoidUnit::Percent,
            }],
            ..OutputShaping::default()
        };
        assert_eq!(run(&low, FAN, &[(0, 41.0)]), [50.0]);
    }

    #[test]
    fn rpm_ranges_resolve_through_the_map_and_invalid_input_is_rejected() {
        let mut cfg = OutputShaping {
            avoid: vec![AvoidRange {
                from: 900.0,
                to: 1100.0,
                unit: AvoidUnit::Rpm,
            }],
            ..OutputShaping::default()
        };
        assert!(cfg.validate_for(false, false).is_err());
        cfg.resolve_rpm(|rpm| Ok(rpm / 20.0)).unwrap();
        assert_eq!((cfg.avoid[0].from, cfg.avoid[0].to), (45.0, 55.0));
        cfg.validate_for(false, false).unwrap();
        for bad in [
            OutputShaping {
                min_percent: 70.0,
                max_percent: 60.0,
                ..OutputShaping::default()
            },
            OutputShaping {
                response_delay_ms: 60_001,
                ..OutputShaping::default()
            },
            OutputShaping {
                hysteresis_up_percent: f32::NAN,
                ..OutputShaping::default()
            },
            OutputShaping {
                avoid: vec![AvoidRange {
                    from: 60.0,
                    to: 50.0,
                    unit: AvoidUnit::Percent,
                }],
                ..OutputShaping::default()
            },
        ] {
            assert!(bad.validate().is_err());
        }
        let json = r#"{"offset_percent":5,"avoid":[{"from":40,"to":45}],"spin_up":{"percent":70,"ms":800}}"#;
        let parsed: OutputShaping = serde_json::from_str(json).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed.max_percent, 100.0);
        assert!(serde_json::from_str::<OutputShaping>(r#"{"stall_percent":3}"#).is_err());
    }
}
