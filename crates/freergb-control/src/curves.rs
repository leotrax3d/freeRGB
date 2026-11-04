//! FanControl-style composite curves: graph, constant, two-threshold trigger,
//! mix and follow nodes evaluated as an acyclic graph. Pure policy, no writes.
//!
//! Safety: every node carries a fail-safe flag. A stale or invalid sensor, an
//! invalid node or a critical temperature yields 100% and the flag propagates
//! through mix/follow nodes, so a `minimum` mix can never mask a failed input.
use crate::{CurveController, FanCurve, LinearCurve};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_NODES: usize = 64;
const MAX_SOURCES: usize = 16;
const MAX_HOLD_MS: u64 = 600_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CurveNode {
    /// Point graph over one sensor (the existing [`FanCurve`] evaluator).
    Graph { sensor: String, curve: FanCurve },
    /// FanControl "linear": two temperatures, two duties (a two-point graph).
    Linear {
        sensor: String,
        min_c: f32,
        max_c: f32,
        min_percent: f32,
        max_percent: f32,
        critical_c: f32,
    },
    /// Adaptive equilibrium: moves the duty towards holding `target_c`. The
    /// change per second is `gain_percent_per_c * (t - target + trend_s * dT/dt)`,
    /// bounded by `max_change_percent_per_s`, within `min..max` percent. Starts
    /// at `max_percent`; stale/missing/critical input fails safe and restarts.
    Equilibrium {
        sensor: String,
        target_c: f32,
        critical_c: f32,
        min_percent: f32,
        max_percent: f32,
        #[serde(default = "default_gain")]
        gain_percent_per_c: f32,
        #[serde(default = "default_change")]
        max_change_percent_per_s: f32,
        #[serde(default = "default_trend")]
        trend_s: f32,
    },
    /// Fixed duty, independent of sensors.
    Constant { percent: f32 },
    /// Two-threshold switch: `load_percent` once the sensor reaches `load_c`,
    /// back to `idle_percent` only at or below `idle_c`. A change must hold
    /// for `hold_ms` before it is applied; heating to `critical_c` never waits.
    Trigger {
        sensor: String,
        idle_c: f32,
        load_c: f32,
        idle_percent: f32,
        load_percent: f32,
        critical_c: f32,
        #[serde(default)]
        hold_ms: u64,
    },
    /// Combines other curve nodes.
    Mix {
        sources: Vec<String>,
        function: CurveMixFunction,
    },
    /// Another node's output scaled and offset (FanControl "sync").
    Follow {
        source: String,
        #[serde(default = "one")]
        scale: f32,
        #[serde(default)]
        offset_percent: f32,
    },
}
fn one() -> f32 {
    1.0
}
fn default_gain() -> f32 {
    1.0
}
fn default_change() -> f32 {
    2.0
}
fn default_trend() -> f32 {
    10.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveMixFunction {
    Maximum,
    Minimum,
    Average,
    Sum,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurveSet {
    pub nodes: BTreeMap<String, CurveNode>,
}

/// A sensor temperature and the age of that sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SensorValue {
    pub celsius: Option<f32>,
    pub age_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveOutput {
    pub percent: f32,
    /// True when the output is 100% because of missing/stale/invalid data or a
    /// critical temperature, not because the curve asked for it.
    pub failsafe: bool,
}
const FAILSAFE: CurveOutput = CurveOutput {
    percent: 100.0,
    failsafe: true,
};

fn valid_percent(p: f32) -> bool {
    p.is_finite() && (0.0..=100.0).contains(&p)
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
}
fn valid_sensor(sensor: &str) -> bool {
    !sensor.is_empty() && sensor.len() <= 1024 && !sensor.chars().any(char::is_control)
}

impl CurveNode {
    fn sources(&self) -> &[String] {
        match self {
            Self::Mix { sources, .. } => sources,
            Self::Follow { source, .. } => std::slice::from_ref(source),
            _ => &[],
        }
    }

    fn linear_curve(&self) -> Option<Result<FanCurve, String>> {
        match self {
            Self::Linear {
                min_c,
                max_c,
                min_percent,
                max_percent,
                critical_c,
                ..
            } => Some(
                LinearCurve {
                    min_c: *min_c,
                    max_c: *max_c,
                    min_percent: *min_percent,
                    max_percent: *max_percent,
                    critical_c: *critical_c,
                    hysteresis_c: 0.0,
                }
                .to_curve(),
            ),
            _ => None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Linear { sensor, .. } => {
                if !valid_sensor(sensor) {
                    return Err("invalid linear sensor".into());
                }
                self.linear_curve().expect("linear").map(|_| ())
            }
            Self::Equilibrium {
                sensor,
                target_c,
                critical_c,
                min_percent,
                max_percent,
                gain_percent_per_c,
                max_change_percent_per_s,
                trend_s,
            } => {
                if !valid_sensor(sensor) {
                    return Err("invalid equilibrium sensor".into());
                }
                if !(target_c.is_finite() && critical_c.is_finite())
                    || !(20.0..=100.0).contains(target_c)
                    || !(40.0..=110.0).contains(critical_c)
                    || target_c >= critical_c
                {
                    return Err("equilibrium needs 20 <= target < critical <= 110 C".into());
                }
                if !valid_percent(*min_percent)
                    || !valid_percent(*max_percent)
                    || min_percent > max_percent
                {
                    return Err("equilibrium needs 0 <= min <= max <= 100 percent".into());
                }
                if !(gain_percent_per_c.is_finite() && (0.1..=20.0).contains(gain_percent_per_c))
                    || !(max_change_percent_per_s.is_finite()
                        && (0.1..=50.0).contains(max_change_percent_per_s))
                    || !(trend_s.is_finite() && (0.0..=120.0).contains(trend_s))
                {
                    return Err(
                        "equilibrium gain 0.1..20, change 0.1..50 %/s, trend 0..120 s".into(),
                    );
                }
                Ok(())
            }
            Self::Graph { sensor, curve } => {
                if !valid_sensor(sensor) {
                    return Err("invalid graph sensor".into());
                }
                curve.validate()
            }
            Self::Constant { percent } if !valid_percent(*percent) => {
                Err("constant must be 0..100 percent".into())
            }
            Self::Constant { .. } => Ok(()),
            Self::Trigger {
                sensor,
                idle_c,
                load_c,
                idle_percent,
                load_percent,
                critical_c,
                hold_ms,
            } => {
                if !valid_sensor(sensor) {
                    return Err("invalid trigger sensor".into());
                }
                if !valid_percent(*idle_percent)
                    || !valid_percent(*load_percent)
                    || idle_percent > load_percent
                {
                    return Err("trigger needs 0 <= idle <= load <= 100 percent".into());
                }
                if !(idle_c.is_finite() && load_c.is_finite() && critical_c.is_finite())
                    || !(-20.0..=110.0).contains(idle_c)
                    || idle_c >= load_c
                    || load_c > critical_c
                    || !(40.0..=110.0).contains(critical_c)
                {
                    return Err("trigger needs idle < load <= critical temperatures".into());
                }
                if *hold_ms > MAX_HOLD_MS {
                    return Err("trigger hold must be at most 600 s".into());
                }
                Ok(())
            }
            Self::Mix { sources, .. } if sources.is_empty() || sources.len() > MAX_SOURCES => {
                Err("mix needs 1..16 sources".into())
            }
            Self::Mix { .. } => Ok(()),
            Self::Follow {
                scale,
                offset_percent,
                ..
            } => {
                if !scale.is_finite() || !(0.0..=4.0).contains(scale) {
                    return Err("follow scale must be 0..4".into());
                }
                if !offset_percent.is_finite() || offset_percent.abs() > 100.0 {
                    return Err("follow offset must be -100..100 percent".into());
                }
                Ok(())
            }
        }
    }
}

impl CurveSet {
    pub fn validate(&self) -> Result<(), String> {
        if self.nodes.len() > MAX_NODES {
            return Err("at most 64 curve nodes".into());
        }
        for (name, node) in &self.nodes {
            if !valid_name(name) {
                return Err(format!("invalid curve name {name:?}"));
            }
            node.validate().map_err(|e| format!("{name}: {e}"))?;
            for source in node.sources() {
                if !self.nodes.contains_key(source) {
                    return Err(format!("{name}: unknown source {source:?}"));
                }
            }
        }
        self.order().map(|_| ())
    }

    /// Topological order (sources before users); errors on cycles.
    fn order(&self) -> Result<Vec<&str>, String> {
        fn visit<'a>(
            set: &'a CurveSet,
            name: &'a str,
            done: &mut BTreeSet<&'a str>,
            active: &mut BTreeSet<&'a str>,
            out: &mut Vec<&'a str>,
        ) -> Result<(), String> {
            if done.contains(name) {
                return Ok(());
            }
            if !active.insert(name) {
                return Err(format!("curve cycle through {name:?}"));
            }
            if let Some(node) = set.nodes.get(name) {
                for source in node.sources() {
                    visit(set, source, done, active, out)?;
                }
            }
            active.remove(name);
            done.insert(name);
            out.push(name);
            Ok(())
        }
        let (mut done, mut active, mut out) = (BTreeSet::new(), BTreeSet::new(), Vec::new());
        for name in self.nodes.keys() {
            visit(self, name, &mut done, &mut active, &mut out)?;
        }
        Ok(out)
    }

    /// Sensor identifiers referenced by graph and trigger nodes.
    pub fn sensors(&self) -> BTreeSet<&str> {
        self.nodes
            .values()
            .filter_map(|node| match node {
                CurveNode::Graph { sensor, .. }
                | CurveNode::Trigger { sensor, .. }
                | CurveNode::Linear { sensor, .. }
                | CurveNode::Equilibrium { sensor, .. } => Some(sensor.as_str()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Default)]
struct TriggerState {
    loaded: bool,
    /// Since when the opposite condition has held.
    pending_since: Option<u64>,
}

struct EquilibriumState {
    duty: f32,
    at_ms: u64,
    celsius: f32,
}

/// Stateful evaluator; keep one per running curve set.
#[derive(Default)]
pub struct CurveEvaluator {
    graphs: BTreeMap<String, CurveController>,
    triggers: BTreeMap<String, TriggerState>,
    equilibria: BTreeMap<String, EquilibriumState>,
}

fn fresh(value: SensorValue) -> Option<f32> {
    value
        .celsius
        .filter(|t| t.is_finite() && (-20.0..=150.0).contains(t))
        .filter(|_| value.age_ms <= 3000)
}

impl CurveEvaluator {
    /// Evaluates every node. An invalid set fails safe as a whole.
    pub fn evaluate(
        &mut self,
        set: &CurveSet,
        sensor: impl Fn(&str) -> SensorValue,
        now_ms: u64,
    ) -> BTreeMap<String, CurveOutput> {
        let mut out = BTreeMap::new();
        if set.validate().is_err() {
            self.graphs.clear();
            self.triggers.clear();
            self.equilibria.clear();
            for name in set.nodes.keys() {
                out.insert(name.clone(), FAILSAFE);
            }
            return out;
        }
        self.graphs.retain(|name, _| set.nodes.contains_key(name));
        self.triggers.retain(|name, _| set.nodes.contains_key(name));
        self.equilibria
            .retain(|name, _| set.nodes.contains_key(name));
        for name in set.order().expect("validated") {
            let node = &set.nodes[name];
            let result = match node {
                CurveNode::Linear { sensor: id, .. } => {
                    let curve = node.linear_curve().expect("linear").expect("validated");
                    let value = sensor(id);
                    let controller = self.graphs.entry(name.to_string()).or_default();
                    let percent = controller.update_at(&curve, value.celsius, value.age_ms, now_ms);
                    CurveOutput {
                        percent,
                        failsafe: !fresh(value).is_some_and(|t| t < curve.critical_c),
                    }
                }
                CurveNode::Equilibrium {
                    sensor: id,
                    target_c,
                    critical_c,
                    min_percent,
                    max_percent,
                    gain_percent_per_c,
                    max_change_percent_per_s,
                    trend_s,
                } => match fresh(sensor(id)).filter(|t| t < critical_c) {
                    Some(t) => {
                        let state =
                            self.equilibria
                                .entry(name.to_string())
                                .or_insert(EquilibriumState {
                                    duty: *max_percent,
                                    at_ms: now_ms,
                                    celsius: t,
                                });
                        if now_ms < state.at_ms {
                            // Reversed clock: restart from the safe end.
                            *state = EquilibriumState {
                                duty: *max_percent,
                                at_ms: now_ms,
                                celsius: t,
                            };
                        }
                        let seconds = (now_ms - state.at_ms).min(5000) as f32 / 1000.0;
                        if seconds > 0.0 {
                            let trend = (t - state.celsius) / seconds;
                            let rate = (gain_percent_per_c * (t - target_c + trend_s * trend))
                                .clamp(-max_change_percent_per_s, *max_change_percent_per_s);
                            state.duty =
                                (state.duty + rate * seconds).clamp(*min_percent, *max_percent);
                            state.at_ms = now_ms;
                            state.celsius = t;
                        }
                        CurveOutput {
                            percent: state.duty,
                            failsafe: false,
                        }
                    }
                    None => {
                        self.equilibria.remove(name);
                        FAILSAFE
                    }
                },
                CurveNode::Constant { percent } => CurveOutput {
                    percent: *percent,
                    failsafe: false,
                },
                CurveNode::Graph { sensor: id, curve } => {
                    let value = sensor(id);
                    let controller = self.graphs.entry(name.to_string()).or_default();
                    let percent = controller.update_at(curve, value.celsius, value.age_ms, now_ms);
                    let healthy = value.celsius.is_some_and(|t| {
                        t.is_finite() && (-20.0..=150.0).contains(&t) && t < curve.critical_c
                    }) && value.age_ms <= 3000;
                    CurveOutput {
                        percent,
                        failsafe: !healthy,
                    }
                }
                CurveNode::Trigger {
                    sensor: id,
                    idle_c,
                    load_c,
                    idle_percent,
                    load_percent,
                    critical_c,
                    hold_ms,
                } => {
                    let value = sensor(id);
                    let state = self.triggers.entry(name.to_string()).or_default();
                    match value
                        .celsius
                        .filter(|t| t.is_finite() && (-20.0..=150.0).contains(t))
                        .filter(|_| value.age_ms <= 3000)
                    {
                        Some(t) if t < *critical_c => {
                            let wants_change = if state.loaded {
                                t <= *idle_c
                            } else {
                                t >= *load_c
                            };
                            if !wants_change {
                                state.pending_since = None;
                            } else {
                                let since = *state.pending_since.get_or_insert(now_ms);
                                if now_ms.saturating_sub(since) >= *hold_ms {
                                    state.loaded = !state.loaded;
                                    state.pending_since = None;
                                }
                            }
                            CurveOutput {
                                percent: if state.loaded {
                                    *load_percent
                                } else {
                                    *idle_percent
                                },
                                failsafe: false,
                            }
                        }
                        _ => {
                            // Resume in the load state once data returns.
                            state.loaded = true;
                            state.pending_since = None;
                            FAILSAFE
                        }
                    }
                }
                CurveNode::Mix { sources, function } => {
                    let inputs: Vec<CurveOutput> = sources.iter().map(|s| out[s]).collect();
                    if inputs.iter().any(|i| i.failsafe) {
                        FAILSAFE
                    } else {
                        let values = inputs.iter().map(|i| i.percent);
                        let percent = match function {
                            CurveMixFunction::Maximum => values.fold(0.0, f32::max),
                            CurveMixFunction::Minimum => values.fold(100.0, f32::min),
                            CurveMixFunction::Average => values.sum::<f32>() / inputs.len() as f32,
                            CurveMixFunction::Sum => values.sum::<f32>(),
                        };
                        CurveOutput {
                            percent: percent.clamp(0.0, 100.0),
                            failsafe: false,
                        }
                    }
                }
                CurveNode::Follow {
                    source,
                    scale,
                    offset_percent,
                } => {
                    let input = out[source];
                    if input.failsafe {
                        FAILSAFE
                    } else {
                        CurveOutput {
                            percent: (input.percent * scale + offset_percent).clamp(0.0, 100.0),
                            failsafe: false,
                        }
                    }
                }
            };
            out.insert(name.to_string(), result);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temps<'a>(map: &'a [(&'a str, f32)]) -> impl Fn(&str) -> SensorValue + 'a {
        move |id| SensorValue {
            celsius: map.iter().find(|(k, _)| *k == id).map(|(_, v)| *v),
            age_ms: 100,
        }
    }
    fn set(nodes: Vec<(&str, CurveNode)>) -> CurveSet {
        CurveSet {
            nodes: nodes.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
    fn graph(sensor: &str) -> CurveNode {
        CurveNode::Graph {
            sensor: sensor.into(),
            curve: FanCurve::default(),
        }
    }
    fn trigger(hold_ms: u64) -> CurveNode {
        CurveNode::Trigger {
            sensor: "gpu".into(),
            idle_c: 45.0,
            load_c: 60.0,
            idle_percent: 30.0,
            load_percent: 70.0,
            critical_c: 90.0,
            hold_ms,
        }
    }

    #[test]
    fn mix_and_follow_combine_outputs() {
        let s = set(vec![
            ("cpu", graph("cpu")),
            ("floor", CurveNode::Constant { percent: 40.0 }),
            (
                "max",
                CurveNode::Mix {
                    sources: vec!["cpu".into(), "floor".into()],
                    function: CurveMixFunction::Maximum,
                },
            ),
            (
                "case",
                CurveNode::Follow {
                    source: "max".into(),
                    scale: 0.5,
                    offset_percent: 10.0,
                },
            ),
        ]);
        let out = CurveEvaluator::default().evaluate(&s, temps(&[("cpu", 50.0)]), 0);
        assert_eq!(out["cpu"].percent, 55.0);
        assert_eq!(out["max"].percent, 55.0);
        assert_eq!(out["case"].percent, 37.5);
        assert!(!out["case"].failsafe);
    }

    #[test]
    fn minimum_mix_cannot_mask_a_missing_sensor() {
        let s = set(vec![
            ("cpu", graph("cpu")),
            ("quiet", CurveNode::Constant { percent: 20.0 }),
            (
                "min",
                CurveNode::Mix {
                    sources: vec!["cpu".into(), "quiet".into()],
                    function: CurveMixFunction::Minimum,
                },
            ),
        ]);
        let out = CurveEvaluator::default().evaluate(&s, temps(&[]), 0);
        assert_eq!(out["min"], FAILSAFE);
    }

    #[test]
    fn cycles_and_unknown_sources_are_rejected_and_fail_safe() {
        let cyclic = set(vec![
            (
                "a",
                CurveNode::Follow {
                    source: "b".into(),
                    scale: 1.0,
                    offset_percent: 0.0,
                },
            ),
            (
                "b",
                CurveNode::Follow {
                    source: "a".into(),
                    scale: 1.0,
                    offset_percent: 0.0,
                },
            ),
        ]);
        assert!(cyclic.validate().unwrap_err().contains("cycle"));
        let out = CurveEvaluator::default().evaluate(&cyclic, temps(&[]), 0);
        assert!(out.values().all(|o| *o == FAILSAFE));
        let dangling = set(vec![(
            "a",
            CurveNode::Mix {
                sources: vec!["nope".into()],
                function: CurveMixFunction::Sum,
            },
        )]);
        assert!(dangling.validate().unwrap_err().contains("unknown source"));
    }

    #[test]
    fn trigger_switches_with_two_thresholds_and_hold() {
        let s = set(vec![("t", trigger(2000))]);
        let mut e = CurveEvaluator::default();
        let at = |e: &mut CurveEvaluator, t: f32, ms: u64| {
            e.evaluate(&s, temps(&[("gpu", t)]), ms)["t"].percent
        };
        assert_eq!(at(&mut e, 50.0, 0), 30.0);
        assert_eq!(at(&mut e, 65.0, 1000), 30.0); // hold not yet elapsed
        assert_eq!(at(&mut e, 65.0, 3000), 70.0);
        assert_eq!(at(&mut e, 50.0, 4000), 70.0); // between thresholds: stays loaded
        assert_eq!(at(&mut e, 40.0, 5000), 70.0);
        assert_eq!(at(&mut e, 50.0, 6000), 70.0); // interrupted: hold restarts
        assert_eq!(at(&mut e, 40.0, 7000), 70.0);
        assert_eq!(at(&mut e, 40.0, 9000), 30.0);
        // Critical temperature never waits for the hold.
        let out = e.evaluate(&s, temps(&[("gpu", 95.0)]), 9100);
        assert_eq!(out["t"], FAILSAFE);
    }

    #[test]
    fn trigger_resumes_loaded_after_sensor_loss() {
        let s = set(vec![("t", trigger(0))]);
        let mut e = CurveEvaluator::default();
        assert_eq!(
            e.evaluate(&s, temps(&[("gpu", 50.0)]), 0)["t"].percent,
            30.0
        );
        assert_eq!(e.evaluate(&s, temps(&[]), 100)["t"], FAILSAFE);
        assert_eq!(
            e.evaluate(&s, temps(&[("gpu", 50.0)]), 200)["t"].percent,
            70.0
        );
    }

    fn equilibrium() -> CurveNode {
        CurveNode::Equilibrium {
            sensor: "cpu".into(),
            target_c: 60.0,
            critical_c: 90.0,
            min_percent: 30.0,
            max_percent: 90.0,
            gain_percent_per_c: 1.0,
            max_change_percent_per_s: 2.0,
            trend_s: 0.0,
        }
    }

    #[test]
    fn equilibrium_settles_on_a_simulated_thermal_plant() {
        // Heat 120 W; cooling proportional to duty: dT/dt = 0.02*(heat - k*duty*(T-25)).
        let s = set(vec![("eq", equilibrium())]);
        let mut e = CurveEvaluator::default();
        let mut t = 70.0_f32;
        let mut duty = 0.0;
        for step in 0..3000u64 {
            let out = e.evaluate(&s, temps(&[("cpu", t)]), step * 500)["eq"];
            assert!(!out.failsafe);
            assert!((30.0..=90.0).contains(&out.percent));
            if step == 0 {
                assert_eq!(out.percent, 90.0); // starts at the safe end
            }
            duty = out.percent;
            t += 0.5 * 0.02 * (120.0 - 0.06 * duty * (t - 25.0));
        }
        assert!((t - 60.0).abs() < 1.0, "temperature {t}, duty {duty}");
        assert!(set(vec![("eq", equilibrium())]).validate().is_ok());
    }

    #[test]
    fn equilibrium_change_rate_is_bounded_and_failures_restart_at_max() {
        let s = set(vec![("eq", equilibrium())]);
        let mut e = CurveEvaluator::default();
        assert_eq!(
            e.evaluate(&s, temps(&[("cpu", 30.0)]), 0)["eq"].percent,
            90.0
        );
        // 30 C below target wants -30 %/s; the limit allows 2 %/s.
        assert_eq!(
            e.evaluate(&s, temps(&[("cpu", 30.0)]), 1000)["eq"].percent,
            88.0
        );
        assert_eq!(e.evaluate(&s, temps(&[]), 2000)["eq"], FAILSAFE);
        assert_eq!(
            e.evaluate(&s, temps(&[("cpu", 95.0)]), 2500)["eq"],
            FAILSAFE
        );
        assert_eq!(
            e.evaluate(&s, temps(&[("cpu", 30.0)]), 3000)["eq"].percent,
            90.0
        );
        let mut bad = equilibrium();
        if let CurveNode::Equilibrium { target_c, .. } = &mut bad {
            *target_c = 95.0;
        }
        assert!(set(vec![("eq", bad)]).validate().is_err());
    }

    #[test]
    fn linear_node_is_a_two_point_graph() {
        let s = set(vec![(
            "lin",
            CurveNode::Linear {
                sensor: "cpu".into(),
                min_c: 40.0,
                max_c: 80.0,
                min_percent: 20.0,
                max_percent: 100.0,
                critical_c: 85.0,
            },
        )]);
        let out = CurveEvaluator::default().evaluate(&s, temps(&[("cpu", 60.0)]), 0);
        assert_eq!(out["lin"].percent, 60.0);
        let out = CurveEvaluator::default().evaluate(&s, temps(&[]), 0);
        assert_eq!(out["lin"], FAILSAFE);
        let json = r#"{"nodes":{"lin":{"type":"linear","sensor":"s","min_c":30,"max_c":70,"min_percent":30,"max_percent":90,"critical_c":85}}}"#;
        serde_json::from_str::<CurveSet>(json)
            .unwrap()
            .validate()
            .unwrap();
    }

    #[test]
    fn stale_samples_fail_safe() {
        let s = set(vec![("cpu", graph("cpu"))]);
        let out = CurveEvaluator::default().evaluate(
            &s,
            |_| SensorValue {
                celsius: Some(30.0),
                age_ms: 5000,
            },
            0,
        );
        assert_eq!(out["cpu"], FAILSAFE);
    }

    #[test]
    fn serde_round_trip_uses_tagged_nodes() {
        let json = r#"{"nodes":{"gpu":{"type":"trigger","sensor":"lhm:/gpu/0/temperature/0","idle_c":45,"load_c":60,"idle_percent":30,"load_percent":70,"critical_c":90},"case":{"type":"follow","source":"gpu","offset_percent":-5}}}"#;
        let s: CurveSet = serde_json::from_str(json).unwrap();
        s.validate().unwrap();
        assert_eq!(
            s.sensors().into_iter().collect::<Vec<_>>(),
            ["lhm:/gpu/0/temperature/0"]
        );
        let back: CurveSet = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        for node in [
            CurveNode::Constant { percent: 120.0 },
            CurveNode::Follow {
                source: "x".into(),
                scale: f32::NAN,
                offset_percent: 0.0,
            },
            CurveNode::Trigger {
                sensor: "s".into(),
                idle_c: 60.0,
                load_c: 50.0,
                idle_percent: 30.0,
                load_percent: 70.0,
                critical_c: 90.0,
                hold_ms: 0,
            },
        ] {
            let s = set(vec![
                ("x", node),
                ("y", CurveNode::Constant { percent: 1.0 }),
            ]);
            assert!(s.validate().is_err());
        }
    }
}
