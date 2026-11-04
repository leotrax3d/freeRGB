//! Shared RPM plausibility vectors (see fixtures/plausibility_vectors.json).
use freergb_control::calibration::TestedDuty;
use freergb_control::plausibility::RpmPlausibility;

#[test]
fn rust_plausibility_matches_shared_vectors() {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/plausibility_vectors.json")).unwrap();
    let tested: Vec<TestedDuty> = serde_json::from_value(value["tested"].clone()).unwrap();
    let mut p = RpmPlausibility::new(&tested).unwrap();
    for step in value["steps"].as_array().unwrap() {
        let got = p.observe(
            step["duty_percent"].as_u64().unwrap() as u8,
            step["rpm"].as_f64().unwrap(),
            step["now_ms"].as_u64().unwrap(),
        );
        assert_eq!(got, step["implausible"].as_bool().unwrap(), "{step}");
    }
}
