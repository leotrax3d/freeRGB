//! Real catalog/install/runtime/state path through the daemon JSON-RPC API.
use freergb_daemon::{api, engine::Engine, lock, SharedEngine};
use freergb_effect_host::marketplace::Manager;
use freergb_effects::EffectSpec;
use serde_json::{json, Value};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
fn call(e: &SharedEngine, method: &str, params: Value) -> Value {
    serde_json::from_str(
        &api::handle_line(
            e,
            &json!({"id":1,"method":method,"params":params}).to_string(),
        )
        .line,
    )
    .unwrap()
}
fn success(e: &SharedEngine, method: &str, params: Value) -> Value {
    let r = call(e, method, params);
    assert!(r.get("error").is_none(), "{r}");
    r["result"].clone()
}
#[test]
fn install_activate_persist_restore_offline_and_uninstall() {
    let root =
        std::env::temp_dir().join(format!("freergb-marketplace-daemon-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let manager = Manager::new(root.join("packages")).unwrap();
    let mut engine = Engine::new(EffectSpec::default_for("solid").unwrap(), 30);
    engine.marketplace = Some(manager.clone());
    let state = root.join("config/state.json");
    let engine = Arc::new(Mutex::new(engine.with_state_file(state.clone())));
    let previous = success(&engine, "effects.get", json!({}));
    success(&engine, "marketplace.install", json!({"id":"aurora"}));
    let deadline = Instant::now() + Duration::from_secs(30);
    while manager.busy() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(manager.view().error.is_none(), "{:?}", manager.view().error);
    let selection = json!({"type":"marketplace","id":"aurora","version":"1.0.0","parameters":{"speed":0.7,"brightness":0.6}});
    success(&engine, "effects.set", selection.clone());
    let now = Instant::now();
    lock(&engine).tick(now);
    assert_eq!(
        success(&engine, "marketplace.status", json!({}))["runtime_error"],
        Value::Null
    );
    assert_eq!(success(&engine, "effects.get", json!({})), selection);
    success(&engine, "profiles.save", json!({"name":"Aurora profile"}));
    success(&engine, "marketplace.disable", json!({"id":"aurora"}));
    lock(&engine).tick(Instant::now());
    assert_eq!(success(&engine, "effects.get", json!({})), previous);
    assert_eq!(
        success(&engine, "marketplace.status", json!({}))["packages"]["disabled"],
        json!(["aurora"])
    );
    assert!(call(&engine, "effects.set", selection.clone())
        .get("error")
        .is_some());
    success(&engine, "marketplace.enable", json!({"id":"aurora"}));
    success(&engine, "effects.set", selection.clone());
    // Invalid parameters and missing packages must leave active state untouched.
    let mut invalid = selection.clone();
    invalid["parameters"]["speed"] = 999.into();
    assert!(call(&engine, "effects.set", invalid).get("error").is_some());
    assert_eq!(success(&engine, "effects.get", json!({})), selection);
    drop(engine);
    drop(manager);
    let mut restored = Engine::new(EffectSpec::default_for("solid").unwrap(), 30);
    restored.marketplace = Some(Manager::new(root.join("packages")).unwrap());
    let restored = Arc::new(Mutex::new(restored.with_state_file(state.clone())));
    assert_eq!(success(&restored, "effects.get", json!({})), selection);
    lock(&restored).tick(Instant::now());
    assert!(lock(&restored).runtime_error().is_none());
    success(&restored, "marketplace.uninstall", json!({"id":"aurora"}));
    lock(&restored).tick(Instant::now());
    assert_eq!(
        success(&restored, "effects.get", json!({}))["type"],
        "solid"
    );
    assert_eq!(
        lock(&restored).runtime_error_code(),
        Some("FX_RUNTIME_FAULT")
    );
    assert!(
        call(&restored, "profiles.load", json!({"name":"Aurora profile"}))
            .get("error")
            .is_some()
    );
    assert_eq!(
        success(&restored, "effects.get", json!({}))["type"],
        "solid"
    );
    drop(restored);
    let _ = std::fs::remove_dir_all(root);
}
#[test]
fn failed_state_write_does_not_acknowledge_or_change_the_effect() {
    let root = std::env::temp_dir().join(format!(
        "freergb-marketplace-state-fail-{}",
        std::process::id()
    ));
    std::fs::write(&root, b"not a directory").unwrap();
    let e = Arc::new(Mutex::new(
        Engine::new(EffectSpec::default_for("solid").unwrap(), 30)
            .with_state_file(root.join("state.json")),
    ));
    let before = success(&e, "effects.get", json!({}));
    assert!(call(
        &e,
        "effects.set",
        json!({"type":"solid","color":{"r":1,"g":2,"b":3}})
    )
    .get("error")
    .is_some());
    assert!(call(&e, "color.set", json!({"color":"#123456"}))
        .get("error")
        .is_some());
    assert_eq!(success(&e, "effects.get", json!({})), before);
    let _ = std::fs::remove_file(root);
}

#[test]
fn development_collection_installs_and_configures_through_lighting_api() {
    let root = std::env::temp_dir().join(format!("freergb-collection-api-{}", std::process::id()));
    let release =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../effects/development");
    let manager = Manager::development(root.clone(), release).unwrap();
    let mut engine = Engine::new(EffectSpec::default_for("solid").unwrap(), 30);
    engine.marketplace = Some(manager.clone());
    let engine = Arc::new(Mutex::new(engine));
    let wait = || {
        let deadline = Instant::now() + Duration::from_secs(30);
        while manager.busy() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(manager.view().error.is_none(), "{:?}", manager.view().error);
    };
    success(&engine, "marketplace.refresh", json!({}));
    wait();
    assert!(!manager.view().ratings.configured);
    for package in manager.view().catalog.into_iter().skip(3) {
        success(&engine, "marketplace.install", json!({"id": package.id}));
        wait();
        for maximum in [false, true] {
            let parameters: serde_json::Map<String, Value> = package
                .parameters
                .iter()
                .map(|p| {
                    (
                        p.key.clone(),
                        json!(if maximum { p.max } else { p.default }),
                    )
                })
                .collect();
            let selection = json!({"type":"marketplace", "id":package.id, "version":package.version, "parameters":parameters});
            success(&engine, "effects.set", selection.clone());
            assert_eq!(success(&engine, "effects.get", json!({})), selection);
            let start = Instant::now();
            for ms in [0, 125, 700, 1900, 3875] {
                lock(&engine).tick(start + Duration::from_millis(ms));
                assert!(
                    lock(&engine).runtime_error().is_none(),
                    "{}: {:?}",
                    package.id,
                    lock(&engine).runtime_error()
                );
            }
        }
        success(
            &engine,
            "effects.set",
            json!({"type":"solid","color":{"r":0,"g":0,"b":0}}),
        );
        success(&engine, "marketplace.uninstall", json!({"id":package.id}));
    }
    assert!(!manager.view().ratings.configured);
    assert!(manager.view().ratings.error.is_none());
    drop(engine);
    drop(manager);
    std::fs::remove_dir_all(root).unwrap();
}
