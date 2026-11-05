//! Pure cooling policy and macro models; no hardware writes or input hooks.
use freergb_core::Rgb;
use freergb_effects::EffectSpec;
use serde::{Deserialize, Serialize};
pub mod calibration;
pub mod channels;
pub mod cooling_safety;
pub mod curves;
pub mod fan_support;
pub mod maintenance;
pub mod plausibility;
pub mod rpm;
pub mod sensor_sources;
pub mod sensors;
pub mod shaping;
pub mod supervisor;
pub mod telemetry_guard;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurvePoint {
    pub temperature_c: f32,
    pub duty_percent: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanCurve {
    pub points: Vec<CurvePoint>,
    pub minimum_percent: f32,
    pub critical_c: f32,
    pub hysteresis_c: f32,
    /// Optional step-up limit (percent per second). Never delays the
    /// critical-temperature or invalid-sensor override to 100%.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rise_percent_per_s: Option<f32>,
    /// Optional step-down limit (percent per second).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_fall_percent_per_s: Option<f32>,
    /// Between points: `linear` (default) or `step` (hold the lower point).
    #[serde(default, skip_serializing_if = "CurveInterpolation::is_linear")]
    pub interpolation: CurveInterpolation,
}

pub const MAX_CURVE_POINTS: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveInterpolation {
    #[default]
    Linear,
    Step,
}
impl CurveInterpolation {
    fn is_linear(&self) -> bool {
        *self == Self::Linear
    }
}

/// FanControl "linear" curve: `min_percent` at or below `min_c`, `max_percent`
/// at or above `max_c`, linear in between. Runs as a two-point [`FanCurve`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearCurve {
    pub min_c: f32,
    pub max_c: f32,
    pub min_percent: f32,
    pub max_percent: f32,
    pub critical_c: f32,
    #[serde(default)]
    pub hysteresis_c: f32,
}
impl LinearCurve {
    pub fn to_curve(&self) -> Result<FanCurve, String> {
        let curve = FanCurve {
            points: vec![
                CurvePoint {
                    temperature_c: self.min_c,
                    duty_percent: self.min_percent,
                },
                CurvePoint {
                    temperature_c: self.max_c,
                    duty_percent: self.max_percent,
                },
            ],
            minimum_percent: self.min_percent,
            critical_c: self.critical_c,
            hysteresis_c: self.hysteresis_c,
            max_rise_percent_per_s: None,
            max_fall_percent_per_s: None,
            interpolation: CurveInterpolation::Linear,
        };
        curve.validate()?;
        Ok(curve)
    }
}

impl Default for FanCurve {
    fn default() -> Self {
        Self {
            points: vec![
                CurvePoint {
                    temperature_c: 25.0,
                    duty_percent: 30.0,
                },
                CurvePoint {
                    temperature_c: 50.0,
                    duty_percent: 55.0,
                },
                CurvePoint {
                    temperature_c: 75.0,
                    duty_percent: 100.0,
                },
            ],
            minimum_percent: 25.0,
            critical_c: 85.0,
            hysteresis_c: 2.0,
            max_rise_percent_per_s: None,
            max_fall_percent_per_s: None,
            interpolation: CurveInterpolation::Linear,
        }
    }
}

impl FanCurve {
    /// Moves the curve point at `index` onto `temperature_c`/`duty_percent`,
    /// clamped so the curve stays valid while dragging or typing: temperatures
    /// stay between the neighbours, duty stays non-decreasing and at or above
    /// the minimum, and the last point cannot move past the critical
    /// temperature. Returns `true` when the point moved.
    pub fn clamp_moved_point(
        &mut self,
        index: usize,
        temperature_c: f32,
        duty_percent: f32,
    ) -> bool {
        if index >= self.points.len() {
            return false;
        }
        let minimum = self.minimum_percent;
        let (min_t, max_t) = (
            if index == 0 {
                -20.0
            } else {
                self.points[index - 1].temperature_c + 0.05
            },
            if index + 1 < self.points.len() {
                self.points[index + 1].temperature_c - 0.05
            } else {
                self.critical_c.min(110.0)
            },
        );
        let (lo, hi) = (
            minimum.max(if index == 0 {
                0.0
            } else {
                self.points[index - 1].duty_percent
            }),
            if index + 1 < self.points.len() {
                self.points[index + 1].duty_percent.max(minimum)
            } else {
                100.0
            },
        );
        let point = &mut self.points[index];
        let before = (point.temperature_c, point.duty_percent);
        point.temperature_c = temperature_c.clamp(min_t.min(max_t), max_t);
        point.duty_percent = duty_percent.clamp(lo.min(hi), hi);
        before != (point.temperature_c, point.duty_percent)
    }

    /// Inserts a curve point at the temperature-sorted position and clamps it
    /// into the curve. Returns the insert index, or `None` when the curve is
    /// full (64 points) or already has a point at this temperature slot.
    pub fn insert_curve_point(&mut self, temperature_c: f32, duty_percent: f32) -> Option<usize> {
        if self.points.len() >= MAX_CURVE_POINTS {
            return None;
        }
        let index = self
            .points
            .iter()
            .position(|point| point.temperature_c > temperature_c)
            .unwrap_or(self.points.len());
        self.points.insert(
            index,
            CurvePoint {
                temperature_c,
                duty_percent,
            },
        );
        self.clamp_moved_point(index, temperature_c, duty_percent);
        Some(index)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(2..=MAX_CURVE_POINTS).contains(&self.points.len()) {
            return Err("curve needs 2..64 points".into());
        }
        // Curves may reach 0% (fan stop); native execution enforces each
        // channel's tier floor (40% unless unrestricted) before any write.
        if !(0.0..=100.0).contains(&self.minimum_percent)
            || !(40.0..=110.0).contains(&self.critical_c)
            || !(0.0..=10.0).contains(&self.hysteresis_c)
        {
            return Err("invalid cooling limits".into());
        }
        for (i, point) in self.points.iter().enumerate() {
            if !(-20.0..=110.0).contains(&point.temperature_c)
                || !(self.minimum_percent..=100.0).contains(&point.duty_percent)
            {
                return Err("invalid curve point".into());
            }
            if i > 0
                && (point.temperature_c <= self.points[i - 1].temperature_c
                    || point.duty_percent < self.points[i - 1].duty_percent)
            {
                return Err("temperatures must increase; duty must not decrease".into());
            }
        }
        if self.points.last().unwrap().temperature_c > self.critical_c {
            return Err("curve extends beyond critical temperature".into());
        }
        for rate in [self.max_rise_percent_per_s, self.max_fall_percent_per_s]
            .into_iter()
            .flatten()
        {
            if !(1.0..=100.0).contains(&rate) {
                return Err("step limits must be 1..100 percent per second".into());
            }
        }
        Ok(())
    }

    pub fn duty(&self, temperature_c: Option<f32>, age_ms: u64) -> f32 {
        if self.validate().is_err() || age_ms > 3000 {
            return 100.0;
        }
        let Some(t) = temperature_c.filter(|t| t.is_finite() && (-20.0..=150.0).contains(t)) else {
            return 100.0;
        };
        if t >= self.critical_c {
            return 100.0;
        }
        if t <= self.points[0].temperature_c {
            return self.points[0].duty_percent;
        }
        for pair in self.points.windows(2) {
            if self.interpolation == CurveInterpolation::Step && t < pair[1].temperature_c {
                return pair[0].duty_percent;
            }
            if t <= pair[1].temperature_c {
                let fraction =
                    (t - pair[0].temperature_c) / (pair[1].temperature_c - pair[0].temperature_c);
                return pair[0].duty_percent
                    + fraction * (pair[1].duty_percent - pair[0].duty_percent);
            }
        }
        self.points.last().unwrap().duty_percent
    }
}

#[derive(Default)]
pub struct CurveController {
    last_temperature: Option<f32>,
    last_duty: f32,
    /// Rate-limited output and its time (only with step limits).
    output: Option<(u64, f32)>,
}

impl CurveController {
    pub fn update(&mut self, curve: &FanCurve, temperature: Option<f32>, age_ms: u64) -> f32 {
        let target = curve.duty(temperature, age_ms);
        let valid = temperature.is_some_and(|t| t.is_finite() && (-20.0..=150.0).contains(&t))
            && age_ms <= 3000
            && curve.validate().is_ok();
        if !valid {
            self.last_temperature = None;
            self.last_duty = 100.0;
            return 100.0;
        }
        let t = temperature.unwrap();
        // React to warming immediately; suppress small downward oscillations.
        if target >= self.last_duty
            || self
                .last_temperature
                .is_none_or(|last| t <= last - curve.hysteresis_c)
        {
            self.last_duty = target;
            self.last_temperature = Some(t);
        }
        self.last_duty
    }

    /// `update` plus the curve's optional step limits. Invalid/stale sensors and
    /// the critical temperature still jump to 100% immediately.
    pub fn update_at(
        &mut self,
        curve: &FanCurve,
        temperature: Option<f32>,
        age_ms: u64,
        now_ms: u64,
    ) -> f32 {
        if self.output.is_some_and(|(at, _)| now_ms < at) {
            // A reversed clock must not freeze a previously low output through
            // the rate limiter. Request the same fallback as stale telemetry.
            self.last_temperature = None;
            self.last_duty = 100.0;
            self.output = Some((now_ms, 100.0));
            return 100.0;
        }
        let target = self.update(curve, temperature, age_ms);
        let emergency = target >= 100.0
            && (self.last_temperature.is_none()
                || temperature.is_some_and(|t| t >= curve.critical_c));
        let duty = match self.output {
            Some((at, previous)) if !emergency => {
                let seconds = now_ms.saturating_sub(at) as f32 / 1000.0;
                let mut duty = target;
                if let Some(rise) = curve.max_rise_percent_per_s {
                    duty = duty.min(previous + rise * seconds);
                }
                if let Some(fall) = curve.max_fall_percent_per_s {
                    duty = duty.max(previous - fall * seconds);
                }
                duty
            }
            _ => target,
        };
        self.output = Some((now_ms, duty));
        duty
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MacroStep {
    Delay { ms: u32 },
    Color { color: Rgb },
    Brightness { value: f32 },
    Effect { effect: EffectSpec },
    Text { text: String },
    Key { key: String },
    KeyDown { key: String },
    KeyUp { key: String },
    Chord { keys: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroDefinition {
    pub steps: Vec<MacroStep>,
    pub repeat: u16,
}

impl MacroDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() || self.steps.len() > 128 || !(1..=20).contains(&self.repeat) {
            return Err("macro needs 1..128 steps and 1..20 repeats".into());
        }
        let mut delay = 0u64;
        let mut held = std::collections::BTreeSet::new();
        for step in &self.steps {
            match step {
                MacroStep::KeyDown { key } => {
                    if !macro_key(key) || !held.insert(key) {
                        return Err("unsupported or already held macro key".into());
                    }
                }
                MacroStep::KeyUp { key } => {
                    if !held.remove(key) {
                        return Err("macro key release without press".into());
                    }
                }
                MacroStep::Text { .. } | MacroStep::Key { .. } | MacroStep::Chord { .. }
                    if !held.is_empty() =>
                {
                    return Err("release held keys before text, key or chord steps".into())
                }
                MacroStep::Delay { ms } => {
                    if *ms > 10_000 {
                        return Err("step delay exceeds 10 seconds".into());
                    }
                    delay += u64::from(*ms);
                }
                MacroStep::Brightness { value } if !(0.0..=1.0).contains(value) => {
                    return Err("invalid brightness".into())
                }
                MacroStep::Effect { effect } => effect.validate()?,
                MacroStep::Chord { keys } => {
                    chord_plan(keys)?;
                }
                MacroStep::Text { text } if text.is_empty() || text.len() > 1024 => {
                    return Err("text needs 1..1024 UTF-8 bytes".into())
                }
                MacroStep::Key { key }
                    if ![
                        "enter",
                        "escape",
                        "tab",
                        "space",
                        "backspace",
                        "up",
                        "down",
                        "left",
                        "right",
                        "media_play_pause",
                        "volume_up",
                        "volume_down",
                        "volume_mute",
                    ]
                    .contains(&key.as_str()) =>
                {
                    return Err("unsupported key".into())
                }
                _ => {}
            }
        }
        if !held.is_empty() {
            return Err("macro must release every held key".into());
        }
        if delay * u64::from(self.repeat) > 60_000 {
            return Err("macro delay exceeds one minute".into());
        }
        Ok(())
    }
    pub fn needs_input(&self) -> bool {
        self.steps.iter().any(|s| {
            matches!(
                s,
                MacroStep::Text { .. }
                    | MacroStep::Key { .. }
                    | MacroStep::Chord { .. }
                    | MacroStep::KeyDown { .. }
                    | MacroStep::KeyUp { .. }
            )
        })
    }
}

/// Bounded portable key vocabulary for recorded press/release pairs.
pub fn macro_key(key: &str) -> bool {
    (key.len() == 1
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
        || [
            "ctrl",
            "alt",
            "shift",
            "meta",
            "enter",
            "escape",
            "tab",
            "space",
            "backspace",
            "up",
            "down",
            "left",
            "right",
        ]
        .contains(&key)
}

/// Canonical modifier-first chord. Exactly one ordinary key; no raw OS key codes.
pub fn chord_plan(keys: &[String]) -> Result<Vec<String>, String> {
    const MODIFIERS: [&str; 4] = ["ctrl", "alt", "shift", "meta"];
    if !(2..=5).contains(&keys.len()) {
        return Err("chord needs 1..4 modifiers and one key".into());
    }
    let mut unique = std::collections::BTreeSet::new();
    let mut ordinary = None;
    for key in keys {
        if !unique.insert(key.as_str()) {
            return Err("duplicate chord key".into());
        }
        if MODIFIERS.contains(&key.as_str()) {
            continue;
        }
        let valid = (key.len() == 1
            && key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
            || [
                "enter",
                "escape",
                "tab",
                "space",
                "backspace",
                "up",
                "down",
                "left",
                "right",
            ]
            .contains(&key.as_str());
        if !valid || ordinary.replace(key.clone()).is_some() {
            return Err("chord needs exactly one supported non-modifier key".into());
        }
    }
    let ordinary = ordinary.ok_or("chord has no non-modifier key")?;
    let mut plan: Vec<String> = MODIFIERS
        .into_iter()
        .filter(|m| unique.contains(m))
        .map(str::to_owned)
        .collect();
    if plan.is_empty() {
        return Err("chord requires a modifier".into());
    }
    plan.push(ordinary);
    Ok(plan)
}

/// Attempt release of every attempted press, even after a backend failure.
/// A failing OS backend cannot guarantee release; callers must report the error.
pub fn execute_chord(
    keys: &[String],
    mut event: impl FnMut(&str, bool) -> Result<(), String>,
) -> Result<(), String> {
    let plan = chord_plan(keys)?;
    let mut attempted = Vec::new();
    let mut error = None;
    for key in &plan {
        attempted.push(key);
        if let Err(e) = event(key, true) {
            error = Some(e);
            break;
        }
    }
    for key in attempted.into_iter().rev() {
        if let Err(e) = event(key, false) {
            if error.is_none() {
                error = Some(e);
            }
        }
    }
    error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recorded_keys_require_balanced_portable_pairs() {
        let valid = MacroDefinition {
            repeat: 1,
            steps: vec![
                MacroStep::KeyDown { key: "ctrl".into() },
                MacroStep::KeyDown { key: "a".into() },
                MacroStep::KeyUp { key: "a".into() },
                MacroStep::KeyUp { key: "ctrl".into() },
            ],
        };
        assert!(valid.validate().is_ok() && valid.needs_input());
        for steps in [
            vec![MacroStep::KeyDown { key: "a".into() }],
            vec![MacroStep::KeyUp { key: "a".into() }],
            vec![MacroStep::KeyDown {
                key: "raw:42".into(),
            }],
            vec![
                MacroStep::KeyDown { key: "ctrl".into() },
                MacroStep::Text { text: "x".into() },
                MacroStep::KeyUp { key: "ctrl".into() },
            ],
        ] {
            assert!(MacroDefinition { repeat: 1, steps }.validate().is_err());
        }
    }
    #[test]
    fn reversed_clock_bypasses_rate_limits_and_recovers_from_full_speed() {
        let curve = FanCurve {
            max_rise_percent_per_s: Some(1.0),
            max_fall_percent_per_s: Some(1.0),
            ..FanCurve::default()
        };
        let mut controller = CurveController::default();
        assert_eq!(controller.update_at(&curve, Some(25.0), 0, 1000), 30.0);
        assert_eq!(controller.update_at(&curve, Some(70.0), 0, 999), 100.0);
        assert_eq!(controller.update_at(&curve, Some(25.0), 0, 1999), 99.0);
        assert_eq!(controller.update_at(&curve, Some(90.0), 0, 2000), 100.0);
    }
    #[test]
    fn chords_validate_and_always_attempt_modifier_cleanup() {
        let keys = vec!["c".into(), "ctrl".into()];
        assert_eq!(chord_plan(&keys).unwrap(), vec!["ctrl", "c"]);
        let mut events = Vec::new();
        assert!(execute_chord(&keys, |key, down| {
            events.push((key.to_owned(), down));
            if key == "c" && down {
                Err("simulated input error".into())
            } else {
                Ok(())
            }
        })
        .is_err());
        assert_eq!(
            events,
            vec![
                ("ctrl".into(), true),
                ("c".into(), true),
                ("c".into(), false),
                ("ctrl".into(), false)
            ]
        );
        for bad in [
            vec!["ctrl".into(), "ctrl".into()],
            vec!["ctrl".into(), "a".into(), "b".into()],
            vec!["ctrl".into(), "raw:42".into()],
        ] {
            assert!(chord_plan(&bad).is_err());
        }
        let definition = MacroDefinition {
            repeat: 1,
            steps: vec![MacroStep::Chord { keys }],
        };
        assert!(definition.validate().is_ok() && definition.needs_input());
    }
    #[test]
    fn curve_interpolation_and_fail_safe() {
        let curve = FanCurve::default();
        assert!(curve.validate().is_ok());
        assert_eq!(curve.duty(Some(37.5), 0), 42.5);
        for temperature in [None, Some(f32::NAN), Some(90.0), Some(-50.0)] {
            assert_eq!(curve.duty(temperature, 0), 100.0);
        }
        assert_eq!(curve.duty(Some(25.0), 3001), 100.0);
        let mut invalid = curve.clone();
        invalid.points[1].temperature_c = 25.0;
        assert!(invalid.validate().is_err());
        assert_eq!(invalid.duty(Some(25.0), 0), 100.0);
    }
    #[test]
    fn step_graphs_hold_the_lower_point_and_linear_curves_bind_as_graphs() {
        let mut curve = FanCurve {
            interpolation: CurveInterpolation::Step,
            ..FanCurve::default()
        };
        assert_eq!(curve.duty(Some(37.5), 0), 30.0);
        assert_eq!(curve.duty(Some(50.0), 0), 55.0); // exactly on a point
        assert_eq!(curve.duty(Some(74.9), 0), 55.0);
        assert_eq!(curve.duty(Some(80.0), 0), 100.0);
        assert_eq!(curve.duty(Some(90.0), 0), 100.0); // critical still wins
                                                      // Arbitrary point counts up to 64.
        curve.points = (0..64)
            .map(|i| CurvePoint {
                temperature_c: 10.0 + i as f32,
                duty_percent: 30.0 + i as f32,
            })
            .collect();
        assert!(curve.validate().is_ok());
        curve.points.push(CurvePoint {
            temperature_c: 80.0,
            duty_percent: 100.0,
        });
        assert!(curve.validate().is_err());
        let json = serde_json::to_string(&FanCurve::default()).unwrap();
        assert!(!json.contains("interpolation"));
        let linear = LinearCurve {
            min_c: 30.0,
            max_c: 70.0,
            min_percent: 40.0,
            max_percent: 100.0,
            critical_c: 85.0,
            hysteresis_c: 0.0,
        }
        .to_curve()
        .unwrap();
        assert_eq!(linear.duty(Some(20.0), 0), 40.0);
        assert_eq!(linear.duty(Some(50.0), 0), 70.0);
        assert_eq!(linear.duty(Some(75.0), 0), 100.0);
        assert!(LinearCurve {
            min_c: 70.0,
            max_c: 30.0,
            min_percent: 40.0,
            max_percent: 100.0,
            critical_c: 85.0,
            hysteresis_c: 0.0,
        }
        .to_curve()
        .is_err());
    }
    #[test]
    fn hysteresis_does_not_delay_emergency_or_recovery() {
        let curve = FanCurve::default();
        let mut controller = CurveController::default();
        assert_eq!(controller.update(&curve, Some(50.0), 0), 55.0);
        assert_eq!(controller.update(&curve, Some(49.0), 0), 55.0);
        assert!(controller.update(&curve, Some(47.0), 0) < 55.0);
        assert_eq!(controller.update(&curve, None, 0), 100.0);
        assert_eq!(controller.update(&curve, Some(25.0), 0), 30.0);
        assert_eq!(controller.update(&curve, Some(100.0), 0), 100.0);
    }
    #[test]
    fn macro_bounds_and_input_classification() {
        let mut definition = MacroDefinition {
            steps: vec![MacroStep::Delay { ms: 10_000 }],
            repeat: 6,
        };
        assert!(definition.validate().is_ok());
        definition.repeat = 7;
        assert!(definition.validate().is_err());
        definition = MacroDefinition {
            steps: vec![MacroStep::Text {
                text: "hello".into(),
            }],
            repeat: 1,
        };
        assert!(definition.validate().is_ok() && definition.needs_input());
    }
}
