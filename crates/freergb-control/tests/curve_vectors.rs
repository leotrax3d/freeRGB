//! Shared curve vectors: the protected C# background controller
//! (`providers/mainboard/BackgroundCooling.cs`) is checked against the same file.
use freergb_control::{CurveController, FanCurve};

#[test]
fn rust_curve_controller_matches_shared_vectors() {
    let text = include_str!("fixtures/curve_vectors.json");
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    let curve: FanCurve = serde_json::from_value(value["curve"].clone()).unwrap();
    let mut controller = CurveController::default();
    for step in value["steps"].as_array().unwrap() {
        let temperature = step["temperature_c"].as_f64().map(|t| t as f32);
        let duty = controller.update(&curve, temperature, step["age_ms"].as_u64().unwrap());
        let expected = step["duty_percent"].as_f64().unwrap() as f32;
        assert!((duty - expected).abs() < 1e-3, "{step}: {duty}");
    }
}
