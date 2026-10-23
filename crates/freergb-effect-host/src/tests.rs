use crate::{
    marketplace::{Catalog, Manager},
    runtime::WasmEffect,
};
use freergb_core::{Canvas, Rgb};
use freergb_effects::Effect;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
fn directory() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let p = std::env::temp_dir().join(format!(
        "freergb-effect-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn wait(m: &Arc<Manager>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while m.busy() {
        assert!(Instant::now() < deadline, "job timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn module(body: &str) -> Vec<u8> {
    wat::parse_str(format!(r#"(module (memory (export "memory") 1) (func (export "set_param") (param i32 f64)) (func (export "render") (param f64 i32 i32) (result i32) {body}))"#)).unwrap()
}
#[test]
fn all_rust_examples_render_animate_and_configure() {
    let m = Manager::new(directory()).unwrap();
    for e in m.view().catalog {
        m.install(&e.id).unwrap();
        wait(&m);
        assert!(m.view().error.is_none(), "{:?}", m.view().error);
        let fault = WasmEffect::fault();
        let mut effect = m
            .load(&e.id, &e.version, &BTreeMap::new(), fault.clone())
            .unwrap();
        let mut first = Canvas::new(320, 200);
        effect.render(Duration::ZERO, &mut first);
        let mut later = Canvas::new(320, 200);
        effect.render(Duration::from_millis(700), &mut later);
        assert!(
            fault.lock().unwrap().is_none(),
            "{:?}",
            fault.lock().unwrap()
        );
        assert_ne!(first, later);
        let mut dark = m
            .load(
                &e.id,
                &e.version,
                &BTreeMap::from([("brightness".into(), 0.0)]),
                WasmEffect::fault(),
            )
            .unwrap();
        dark.render(Duration::ZERO, &mut later);
        assert_eq!(later, Canvas::new(320, 200));
    }
}
#[test]
fn traps_fuel_and_invalid_buffers_disable_only_the_effect() {
    for body in [
        "(loop br 0) i32.const 0",
        "unreachable",
        "i32.const -1",
        "i32.const 65535",
    ] {
        let fault = WasmEffect::fault();
        let mut effect = WasmEffect::load(&module(body), &[], fault.clone()).unwrap();
        let mut canvas = Canvas::new(4, 4);
        canvas.fill(Rgb::WHITE);
        effect.render(Duration::ZERO, &mut canvas);
        assert!(fault.lock().unwrap().is_some());
        assert_eq!(canvas, Canvas::new(4, 4));
        effect.render(Duration::from_secs(1), &mut canvas);
        assert_eq!(canvas, Canvas::new(4, 4));
    }
    assert!(WasmEffect::load(&module("i32.const 0"), &[], WasmEffect::fault()).is_ok());
}
#[test]
fn forbids_host_imports_start_and_excess_memory() {
    for wat in [
        r#"(module (import "wasi_snapshot_preview1" "fd_write" (func)))"#,
        r#"(module (func $start (loop br 0)) (start $start))"#,
        r#"(module (memory 129))"#,
    ] {
        assert!(WasmEffect::load(&wat::parse_str(wat).unwrap(), &[], WasmEffect::fault()).is_err());
    }
    assert!(WasmEffect::load(&vec![0; 512 * 1024 + 1], &[], WasmEffect::fault()).is_err());
}
#[test]
fn catalog_rejects_traversal_remote_urls_permissions_and_unknown_api() {
    let original = include_bytes!("../../../effects/catalog.json");
    let mut c: serde_json::Value = serde_json::from_slice(original).unwrap();
    for bad in [
        "../escape.wasm",
        "https://evil.example/code.wasm",
        "/absolute.wasm",
    ] {
        c["effects"][0]["artifact"]["url"] = bad.into();
        assert!(Catalog::parse(&serde_json::to_vec(&c).unwrap()).is_err());
    }
    let mut c = Catalog::parse(original).unwrap();
    c.effects[0].permissions.push("audio".into());
    assert!(c.effects[0].validate().is_err());
    c.effects[0].permissions.clear();
    c.effects[0].api = 2;
    assert!(c.effects[0].validate().is_err());
}
#[test]
fn downloaded_install_update_restart_tamper_and_uninstall() {
    let release = directory();
    let root = directory();
    let mut catalog = Catalog::parse(include_bytes!("../../../effects/catalog.json")).unwrap();
    std::fs::write(
        release.join("catalog.json"),
        serde_json::to_vec(&catalog).unwrap(),
    )
    .unwrap();
    for entry in &catalog.effects {
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../effects/packages")
                .join(&entry.artifact.url),
            release.join(&entry.artifact.url),
        )
        .unwrap();
    }
    let m = Manager::development(root.clone(), release.clone()).unwrap();
    m.refresh().unwrap();
    wait(&m);
    m.install("aurora").unwrap();
    wait(&m);
    assert!(m.view().error.is_none());
    let p = BTreeMap::new();
    m.load("aurora", "1.0.0", &p, WasmEffect::fault()).unwrap();
    assert!(m
        .load(
            "aurora",
            "1.0.0",
            &BTreeMap::from([("speed".into(), 99.0)]),
            WasmEffect::fault()
        )
        .is_err());
    assert!(m
        .load(
            "aurora",
            "1.0.0",
            &BTreeMap::from([("unknown".into(), 1.0)]),
            WasmEffect::fault()
        )
        .is_err());
    catalog.sequence = 2;
    catalog.effects[0].version = "1.1.0".into();
    std::fs::write(
        release.join("catalog.json"),
        serde_json::to_vec(&catalog).unwrap(),
    )
    .unwrap();
    m.refresh().unwrap();
    wait(&m);
    m.install("aurora").unwrap();
    wait(&m);
    assert!(m.view().error.is_none());
    // Old saved selections remain resolvable after updating the installed version.
    let reopened = Manager::development(root.clone(), release.clone()).unwrap();
    assert_eq!(reopened.view().installed[0].version, "1.1.0");
    reopened
        .load("aurora", "1.0.0", &p, WasmEffect::fault())
        .unwrap();
    reopened
        .load("aurora", "1.1.0", &p, WasmEffect::fault())
        .unwrap();
    catalog.sequence = 1;
    std::fs::write(
        release.join("catalog.json"),
        serde_json::to_vec(&catalog).unwrap(),
    )
    .unwrap();
    reopened.refresh().unwrap();
    wait(&reopened);
    assert!(reopened.view().error.as_ref().unwrap().contains("rollback"));
    std::fs::write(root.join("packages/aurora/1.1.0.wasm"), b"corrupt").unwrap();
    assert!(reopened
        .load("aurora", "1.1.0", &p, WasmEffect::fault())
        .is_err());
    reopened.uninstall("aurora").unwrap();
    assert!(reopened.view().installed.is_empty());
    assert!(reopened
        .load("aurora", "1.0.0", &p, WasmEffect::fault())
        .is_err());
}
#[test]
fn bad_download_never_changes_the_installed_index() {
    let release = directory();
    let root = directory();
    let catalog = include_bytes!("../../../effects/catalog.json");
    std::fs::write(release.join("catalog.json"), catalog).unwrap();
    std::fs::write(release.join("aurora-1.0.0.wasm"), b"not trusted").unwrap();
    let m = Manager::development(root.clone(), release).unwrap();
    m.refresh().unwrap();
    wait(&m);
    m.install("aurora").unwrap();
    wait(&m);
    assert!(m.view().error.is_some());
    assert!(m.view().installed.is_empty());
    assert!(!root.join("installed.json").exists());
}

#[test]
fn legacy_official_catalog_has_publisher_and_new_metadata_is_validated() {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("../../../effects/catalog.json")).unwrap();
    value["effects"][0]
        .as_object_mut()
        .unwrap()
        .remove("publisher");
    let catalog = crate::marketplace::Catalog::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(catalog.effects[0].publisher.name, "freeRGB Team");
    value["effects"][0]["publisher"] = serde_json::json!({"id":"../escape","name":"Fake"});
    assert!(crate::marketplace::Catalog::parse(&serde_json::to_vec(&value).unwrap()).is_err());
}
