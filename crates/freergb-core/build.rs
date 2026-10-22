//! Exposes the pinned LibreHardwareMonitor version from the single source of
//! truth, `packaging/windows/mainboard-packages.json`, as `FREERGB_LHM_VERSION`.
use std::path::Path;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packaging/windows/mainboard-packages.json");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let text = std::fs::read_to_string(&manifest).expect("read mainboard-packages.json");
    let document: serde_json::Value = serde_json::from_str(&text).expect("parse manifest");
    let version = document["packages"]["lhm"]["version"]
        .as_str()
        .expect("packages.lhm.version");
    let parts: Vec<_> = version.split('.').collect();
    assert!(
        parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())),
        "invalid pinned LHM version {version}"
    );
    println!("cargo:rustc-env=FREERGB_LHM_VERSION={version}");
}
