//! Keeps the frontend's contract fixtures in sync with the Rust serde types.
//! Regenerate with `UPDATE_FIXTURES=1 cargo test --test fixtures`.

use freergb_effects::EffectSpec;
use serde_json::{json, Value};

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/api/fixtures/effects.json");

#[test]
fn effect_defaults_fixture_is_current() {
    let defaults: serde_json::Map<String, Value> = EffectSpec::NAMES
        .iter()
        .map(|name| (name.to_string(), json!(EffectSpec::default_for(name).expect("known effect"))))
        .collect();
    let expected = serde_json::to_string_pretty(&defaults).unwrap() + "\n";
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::create_dir_all(std::path::Path::new(PATH).parent().unwrap()).unwrap();
        std::fs::write(PATH, &expected).unwrap();
    }
    let actual = std::fs::read_to_string(PATH).unwrap_or_default().replace("\r\n", "\n");
    assert_eq!(actual, expected, "run with UPDATE_FIXTURES=1 and check the TS types");
}
