//! Bounded Celsius sensor graphs. Missing inputs never become a fabricated reading.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_NODES: usize = 128;
const MAX_HISTORY: usize = 600;
pub const MAX_AGE_MS: u64 = 3000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensorGraph {
    #[serde(default = "graph_version")]
    pub version: u32,
    pub nodes: BTreeMap<String, SensorNode>,
}
fn graph_version() -> u32 {
    1
}
impl Default for SensorGraph {
    fn default() -> Self {
        Self {
            version: 1,
            nodes: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SensorNode {
    Input {
        source: String,
    },
    Offset {
        source: String,
        amount: f64,
        proportional: bool,
    },
    Mix {
        sources: Vec<String>,
        function: MixFunction,
    },
    TimeAverage {
        source: String,
        window_ms: u64,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MixFunction {
    Maximum,
    Minimum,
    Average,
    Sum,
    Subtract,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSample {
    pub value: Option<f64>,
    pub sampled_at_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reading {
    pub value: Option<f64>,
    pub sampled_at_ms: Option<u64>,
    pub error: Option<String>,
}
impl Reading {
    fn missing(message: &str) -> Self {
        Self {
            value: None,
            sampled_at_ms: None,
            error: Some(message.into()),
        }
    }
    fn valid(value: f64, at: u64) -> Self {
        if !value.is_finite() || !(-100.0..=200.0).contains(&value) {
            return Self::missing("derived Celsius value is not finite or outside sanity bounds");
        }
        Self {
            value: Some(value),
            sampled_at_ms: Some(at),
            error: None,
        }
    }
}
impl SensorGraph {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("unsupported sensor graph version".into());
        }
        if self.nodes.len() > MAX_NODES {
            return Err("at most 128 sensor nodes".into());
        }
        for (name, node) in &self.nodes {
            if name.is_empty()
                || name.len() > 64
                || !name
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
            {
                return Err("invalid sensor node name".into());
            }
            match node {
                SensorNode::Input { source }
                    if source.is_empty()
                        || source.len() > 1024
                        || source.chars().any(char::is_control) =>
                {
                    return Err("invalid input source".into())
                }
                SensorNode::Offset { amount, .. }
                    if !amount.is_finite() || amount.abs() > 200.0 =>
                {
                    return Err("invalid sensor offset".into())
                }
                SensorNode::Mix { sources, .. } if sources.is_empty() || sources.len() > 32 => {
                    return Err("mix needs 1..32 sources".into())
                }
                SensorNode::TimeAverage { window_ms, .. }
                    if !(1000..=600_000).contains(window_ms) =>
                {
                    return Err("average window must be 1..600 seconds".into())
                }
                _ => {}
            }
        }
        let mut done = BTreeMap::new();
        for name in self.nodes.keys() {
            self.visit(name, &mut BTreeSet::new(), &mut done, 0)?;
        }
        Ok(())
    }
    fn visit<'a>(
        &'a self,
        name: &'a str,
        visiting: &mut BTreeSet<&'a str>,
        done: &mut BTreeMap<&'a str, usize>,
        depth: usize,
    ) -> Result<usize, String> {
        if depth > 32 {
            return Err("sensor graph exceeds depth 32".into());
        }
        if let Some(height) = done.get(name) {
            return Ok(*height);
        }
        if !visiting.insert(name) {
            return Err("sensor graph contains a cycle".into());
        }
        let node = self
            .nodes
            .get(name)
            .ok_or("sensor reference does not exist")?;
        let mut height = 1;
        for source in node.dependencies() {
            height = height.max(1 + self.visit(source, visiting, done, depth + 1)?);
        }
        if height > 32 {
            return Err("sensor graph exceeds depth 32".into());
        }
        visiting.remove(name);
        done.insert(name, height);
        Ok(height)
    }
}
impl SensorNode {
    fn dependencies(&self) -> Vec<&str> {
        match self {
            Self::Input { .. } => Vec::new(),
            Self::Offset { source, .. } | Self::TimeAverage { source, .. } => vec![source],
            Self::Mix { sources, .. } => sources.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Default)]
pub struct SensorEvaluator {
    history: BTreeMap<String, VecDeque<(u64, f64)>>,
    graph_key: Option<String>,
    last_time: Option<u64>,
}
impl SensorEvaluator {
    pub fn evaluate(
        &mut self,
        graph: &SensorGraph,
        inputs: &BTreeMap<String, InputSample>,
        now_ms: u64,
    ) -> Result<BTreeMap<String, Reading>, String> {
        if let Err(error) = graph.validate() {
            self.history.clear();
            self.graph_key = None;
            self.last_time = None;
            return Err(error);
        }
        // A definition change, clock reset or new session invalidates all averages.
        let key = definition_key(graph);
        if self.graph_key.as_ref() != Some(&key) || self.last_time.is_some_and(|time| now_ms < time)
        {
            self.history.clear();
            self.graph_key = Some(key);
        }
        self.last_time = Some(now_ms);
        let mut values = BTreeMap::new();
        for name in graph.nodes.keys() {
            self.node(name, graph, inputs, now_ms, &mut values);
        }
        Ok(values)
    }
    fn node(
        &mut self,
        name: &str,
        graph: &SensorGraph,
        inputs: &BTreeMap<String, InputSample>,
        now: u64,
        values: &mut BTreeMap<String, Reading>,
    ) -> Reading {
        if let Some(value) = values.get(name) {
            return value.clone();
        }
        let node = &graph.nodes[name];
        let result = match node {
            SensorNode::Input { source } => match inputs.get(source) {
                Some(sample)
                    if sample.sampled_at_ms <= now && now - sample.sampled_at_ms <= MAX_AGE_MS =>
                {
                    sample.value.map_or_else(
                        || Reading::missing("input unavailable"),
                        |value| Reading::valid(value, sample.sampled_at_ms),
                    )
                }
                _ => Reading::missing("input missing, stale or timestamp is in the future"),
            },
            SensorNode::Offset {
                source,
                amount,
                proportional,
            } => {
                let source = self.node(source, graph, inputs, now, values);
                match (source.value, source.sampled_at_ms) {
                    (Some(value), Some(at)) => Reading::valid(
                        if *proportional {
                            value * (1.0 + amount / 100.0)
                        } else {
                            value + amount
                        },
                        at,
                    ),
                    _ => Reading::missing("offset input unavailable"),
                }
            }
            SensorNode::Mix { sources, function } => {
                let sources: Vec<_> = sources
                    .iter()
                    .map(|source| self.node(source, graph, inputs, now, values))
                    .collect();
                if sources
                    .iter()
                    .any(|s| s.value.is_none() || s.sampled_at_ms.is_none())
                {
                    Reading::missing("a required mix input is unavailable")
                } else {
                    let data: Vec<_> = sources.iter().map(|s| s.value.unwrap()).collect();
                    let at = sources
                        .iter()
                        .map(|s| s.sampled_at_ms.unwrap())
                        .min()
                        .unwrap();
                    let value = match function {
                        MixFunction::Maximum => {
                            data.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                        }
                        MixFunction::Minimum => data.iter().copied().fold(f64::INFINITY, f64::min),
                        MixFunction::Average => data.iter().sum::<f64>() / data.len() as f64,
                        MixFunction::Sum => data.iter().sum(),
                        MixFunction::Subtract => data[0] - data[1..].iter().sum::<f64>(),
                    };
                    Reading::valid(value, at)
                }
            }
            SensorNode::TimeAverage { source, window_ms } => {
                let input = self.node(source, graph, inputs, now, values);
                let history = self.history.entry(name.into()).or_default();
                match (input.value, input.sampled_at_ms) {
                    (Some(value), Some(at)) => {
                        if history
                            .back()
                            .is_some_and(|(last, _)| at < *last || at - last > MAX_AGE_MS)
                        {
                            history.clear();
                        }
                        if let Some((last, previous)) =
                            history.back_mut().filter(|(last, _)| *last == at)
                        {
                            let _ = last;
                            *previous = value;
                        } else {
                            history.push_back((at, value));
                        }
                        while history.len() > MAX_HISTORY {
                            history.pop_front();
                        }
                        let cutoff = now.saturating_sub(*window_ms);
                        while history.len() > 1 && history[1].0 <= cutoff {
                            history.pop_front();
                        }
                        let mut area = 0.0;
                        let mut duration = 0;
                        for (index, (start, value)) in history.iter().enumerate() {
                            let start = (*start).max(cutoff);
                            let end = history.get(index + 1).map_or(now, |s| s.0).min(now);
                            if end > start {
                                area += value * (end - start) as f64;
                                duration += end - start;
                            }
                        }
                        Reading::valid(
                            if duration == 0 {
                                value
                            } else {
                                area / duration as f64
                            },
                            at,
                        )
                    }
                    _ => {
                        history.clear();
                        Reading::missing("average input unavailable; history reset")
                    }
                }
            }
        };
        values.insert(name.into(), result.clone());
        result
    }
}

// Stable Debug representation avoids another serialization dependency in the pure model.
fn definition_key(graph: &SensorGraph) -> String {
    format!("{graph:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixing_and_offsets_preserve_the_oldest_input_timestamp() {
        let mut graph = graph();
        graph.nodes.insert(
            "offset".into(),
            SensorNode::Offset {
                source: "cpu".into(),
                amount: 10.0,
                proportional: true,
            },
        );
        let inputs = BTreeMap::from([
            (
                "cpu-package".into(),
                InputSample {
                    value: Some(50.0),
                    sampled_at_ms: 100,
                },
            ),
            (
                "gpu-core".into(),
                InputSample {
                    value: Some(20.0),
                    sampled_at_ms: 200,
                },
            ),
        ]);
        for (function, expected) in [
            (MixFunction::Maximum, 50.0),
            (MixFunction::Minimum, 20.0),
            (MixFunction::Average, 35.0),
            (MixFunction::Sum, 70.0),
            (MixFunction::Subtract, 30.0),
        ] {
            graph.nodes.insert(
                "hot".into(),
                SensorNode::Mix {
                    sources: vec!["cpu".into(), "gpu".into()],
                    function,
                },
            );
            let values = SensorEvaluator::default()
                .evaluate(&graph, &inputs, 200)
                .unwrap();
            assert_eq!(values["hot"].value, Some(expected));
            assert_eq!(values["hot"].sampled_at_ms, Some(100));
            assert!((values["offset"].value.unwrap() - 55.0).abs() < 0.000001);
        }
    }
    #[test]
    fn histories_are_bounded_and_definition_changes_reset_them() {
        let mut graph = SensorGraph::default();
        graph.nodes.insert(
            "raw".into(),
            SensorNode::Input {
                source: "real".into(),
            },
        );
        graph.nodes.insert(
            "avg".into(),
            SensorNode::TimeAverage {
                source: "raw".into(),
                window_ms: 600_000,
            },
        );
        let mut evaluator = SensorEvaluator::default();
        for at in 0..1000 {
            evaluator
                .evaluate(
                    &graph,
                    &BTreeMap::from([(
                        "real".into(),
                        InputSample {
                            value: Some(20.0),
                            sampled_at_ms: at,
                        },
                    )]),
                    at,
                )
                .unwrap();
        }
        assert_eq!(evaluator.history["avg"].len(), MAX_HISTORY);
        graph.nodes.insert(
            "raw".into(),
            SensorNode::Input {
                source: "different".into(),
            },
        );
        let inputs = BTreeMap::from([(
            "different".into(),
            InputSample {
                value: Some(90.0),
                sampled_at_ms: 1000,
            },
        )]);
        assert_eq!(
            evaluator.evaluate(&graph, &inputs, 1000).unwrap()["avg"].value,
            Some(90.0)
        );
        let mut invalid = graph.clone();
        invalid.nodes.insert(
            "bad".into(),
            SensorNode::Offset {
                source: "missing".into(),
                amount: 0.0,
                proportional: false,
            },
        );
        assert!(evaluator.evaluate(&invalid, &inputs, 1001).is_err());
        assert!(evaluator.history.is_empty());
    }
    fn graph() -> SensorGraph {
        SensorGraph {
            version: 1,
            nodes: BTreeMap::from([
                (
                    "cpu".into(),
                    SensorNode::Input {
                        source: "cpu-package".into(),
                    },
                ),
                (
                    "gpu".into(),
                    SensorNode::Input {
                        source: "gpu-core".into(),
                    },
                ),
                (
                    "hot".into(),
                    SensorNode::Mix {
                        sources: vec!["cpu".into(), "gpu".into()],
                        function: MixFunction::Maximum,
                    },
                ),
            ]),
        }
    }
    #[test]
    fn missing_and_stale_inputs_do_not_get_dropped_from_a_mix() {
        let mut evaluator = SensorEvaluator::default();
        let graph = graph();
        let mut inputs = BTreeMap::from([
            (
                "cpu-package".into(),
                InputSample {
                    value: Some(80.0),
                    sampled_at_ms: 0,
                },
            ),
            (
                "gpu-core".into(),
                InputSample {
                    value: Some(50.0),
                    sampled_at_ms: 1000,
                },
            ),
        ]);
        assert_eq!(
            evaluator.evaluate(&graph, &inputs, 1000).unwrap()["hot"].value,
            Some(80.0)
        );
        assert!(evaluator.evaluate(&graph, &inputs, 3001).unwrap()["hot"]
            .value
            .is_none());
        inputs.remove("cpu-package");
        assert!(evaluator.evaluate(&graph, &inputs, 1000).unwrap()["hot"]
            .value
            .is_none());
    }
    #[test]
    fn cycles_missing_references_and_deep_graphs_are_rejected() {
        let mut graph = graph();
        graph.nodes.insert(
            "cpu".into(),
            SensorNode::Offset {
                source: "hot".into(),
                amount: 0.0,
                proportional: false,
            },
        );
        assert!(graph.validate().is_err());
        graph.nodes.remove("cpu");
        assert!(graph.validate().is_err());
        let mut graph = SensorGraph::default();
        graph.nodes.insert(
            "n0".into(),
            SensorNode::Input {
                source: "real".into(),
            },
        );
        for i in 1..40 {
            graph.nodes.insert(
                format!("n{i}"),
                SensorNode::Offset {
                    source: format!("n{}", i - 1),
                    amount: 0.0,
                    proportional: false,
                },
            );
        }
        assert!(graph.validate().is_err());
    }
    #[test]
    fn average_is_time_weighted_and_cannot_renew_stale_input() {
        let mut graph = SensorGraph::default();
        graph.nodes.insert(
            "raw".into(),
            SensorNode::Input {
                source: "real".into(),
            },
        );
        graph.nodes.insert(
            "avg".into(),
            SensorNode::TimeAverage {
                source: "raw".into(),
                window_ms: 10_000,
            },
        );
        let mut evaluator = SensorEvaluator::default();
        let inputs = |value, at| {
            BTreeMap::from([(
                "real".into(),
                InputSample {
                    value: Some(value),
                    sampled_at_ms: at,
                },
            )])
        };
        evaluator.evaluate(&graph, &inputs(20.0, 0), 0).unwrap();
        evaluator
            .evaluate(&graph, &inputs(80.0, 1000), 1000)
            .unwrap();
        let result = evaluator
            .evaluate(&graph, &inputs(80.0, 1000), 2000)
            .unwrap();
        assert_eq!(result["avg"].value, Some(50.0));
        assert_eq!(result["avg"].sampled_at_ms, Some(1000));
        assert!(evaluator
            .evaluate(&graph, &inputs(80.0, 1000), 4001)
            .unwrap()["avg"]
            .value
            .is_none());
        assert_eq!(
            evaluator
                .evaluate(&graph, &inputs(90.0, 5000), 5000)
                .unwrap()["avg"]
                .value,
            Some(90.0)
        );
    }
    #[test]
    fn nonfinite_future_and_overflow_values_fail_closed() {
        let graph = graph();
        let mut evaluator = SensorEvaluator::default();
        for (value, at) in [(f64::NAN, 0), (f64::INFINITY, 0), (50.0, 1)] {
            let inputs = BTreeMap::from([(
                "cpu-package".into(),
                InputSample {
                    value: Some(value),
                    sampled_at_ms: at,
                },
            )]);
            assert!(evaluator.evaluate(&graph, &inputs, 0).unwrap()["cpu"]
                .value
                .is_none());
        }
    }
}
