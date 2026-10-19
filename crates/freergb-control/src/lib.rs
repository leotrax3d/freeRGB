//! Pure cooling policy and macro models; no hardware writes or input hooks.
use freergb_core::Rgb;
use freergb_effects::EffectSpec;
use serde::{Deserialize, Serialize};

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
        }
    }
}

impl FanCurve {
    pub fn validate(&self) -> Result<(), String> {
        if !(2..=16).contains(&self.points.len()) {
            return Err("curve needs 2..16 points".into());
        }
        if !(20.0..=100.0).contains(&self.minimum_percent)
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
                .map_or(true, |last| t <= last - curve.hysteresis_c)
        {
            self.last_duty = target;
            self.last_temperature = Some(t);
        }
        self.last_duty
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
        for step in &self.steps {
            match step {
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
        if delay * u64::from(self.repeat) > 60_000 {
            return Err("macro delay exceeds one minute".into());
        }
        Ok(())
    }
    pub fn needs_input(&self) -> bool {
        self.steps.iter().any(|s| {
            matches!(
                s,
                MacroStep::Text { .. } | MacroStep::Key { .. } | MacroStep::Chord { .. }
            )
        })
    }
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
