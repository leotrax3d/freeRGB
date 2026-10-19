use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};

pub const CAPACITY: usize = 600;
const MAX_SENSORS: usize = 128;
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub at_s: f64,
    pub value: Option<f64>,
}
#[derive(Default)]
pub struct Histories {
    pub series: BTreeMap<(String, String), VecDeque<Sample>>,
    last_seen: BTreeMap<(String, String), f64>,
}
impl Histories {
    pub fn observe(&mut self, devices: &[Value], now_s: f64) {
        let mut current = BTreeMap::new();
        for device in devices {
            let Some(id) = device["info"]["id"].as_str() else {
                continue;
            };
            let Some(sensors) = device["sensors"].as_array() else {
                continue;
            };
            for sensor in sensors.iter().take(MAX_SENSORS) {
                let Some(sensor_id) = sensor["id"].as_str() else {
                    continue;
                };
                let key = (id.to_owned(), sensor_id.to_owned());
                if !self.series.contains_key(&key) && self.series.len() >= MAX_SENSORS {
                    continue;
                }
                self.series.entry(key.clone()).or_default();
                self.last_seen.insert(key.clone(), now_s);
                let fresh = device["status"] == "Connected"
                    && sensor["age_ms"].as_u64().is_some_and(|age| age <= 3000);
                current.insert(
                    key,
                    sensor["value"].as_f64().filter(|v| fresh && v.is_finite()),
                );
            }
        }
        self.series.retain(|key, _| {
            self.last_seen
                .get(key)
                .is_some_and(|seen| now_s - seen <= 60.0)
        });
        self.last_seen
            .retain(|key, _| self.series.contains_key(key));
        for (key, series) in &mut self.series {
            if series.len() == CAPACITY {
                series.pop_front();
            }
            series.push_back(Sample {
                at_s: now_s,
                value: current.get(key).copied().flatten(),
            });
        }
    }
}

pub fn graph(ui: &mut eframe::egui::Ui, series: &VecDeque<Sample>) {
    use eframe::egui;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(600.0), 90.0),
        egui::Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, 3.0, egui::Color32::from_gray(24));
    let values: Vec<_> = series.iter().filter_map(|s| s.value).collect();
    if let (Some(min), Some(max)) = (
        values.iter().copied().reduce(f64::min),
        values.iter().copied().reduce(f64::max),
    ) {
        let start = series.front().unwrap().at_s;
        let span = (series.back().unwrap().at_s - start).max(1.0);
        let position = |sample: &Sample, value: f64| {
            egui::pos2(
                rect.left() + ((sample.at_s - start) / span) as f32 * rect.width(),
                rect.bottom() - ((value - min + 1.0) / (max - min + 2.0)) as f32 * rect.height(),
            )
        };
        for (a, b) in series.iter().zip(series.iter().skip(1)) {
            if let (Some(va), Some(vb)) = (a.value, b.value) {
                ui.painter().line_segment(
                    [position(a, va), position(b, vb)],
                    egui::Stroke::new(1.5_f32, egui::Color32::LIGHT_BLUE),
                );
            }
        }
        ui.small(format!(
            "Range {min:.0}–{max:.0} · up to 600 samples · stale data leaves gaps"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn missing_and_stale_samples_are_gaps_and_removed_devices_expire() {
        let mut history = Histories::default();
        let mut device = json!({"info":{"id":"dev"},"status":"Connected","sensors":[{"id":"rpm:0","value":1200,"age_ms":0}]});
        history.observe(std::slice::from_ref(&device), 0.0);
        device["sensors"][0]["age_ms"] = json!(3001);
        history.observe(&[device], 1.0);
        history.observe(&[], 2.0);
        let series = history.series.values().next().unwrap();
        assert_eq!(series[0].value, Some(1200.0));
        assert_eq!(series[1].value, None);
        assert_eq!(series[2].value, None);
        history.observe(&[], 62.0);
        assert!(history.series.is_empty());
    }
    #[test]
    fn long_runs_keep_only_recent_samples() {
        let mut history = Histories::default();
        let device = json!({"info":{"id":"dev"},"status":"Connected","sensors":[{"id":"rpm:0","value":1200,"age_ms":0}]});
        for time in 0..1000 {
            history.observe(std::slice::from_ref(&device), f64::from(time));
        }
        let series = history.series.values().next().unwrap();
        assert_eq!(series.len(), CAPACITY);
        assert_eq!(series[0].at_s, 400.0);
    }
}
