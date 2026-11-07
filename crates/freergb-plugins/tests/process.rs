use freergb_core::{Frame, LedLayout, Rgb};
use freergb_device_api::{DeviceStatus, RgbDevice};
use freergb_plugins::{Access, Manifest, ProcessDevice, PROTOCOL_VERSION};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn manifest(fault: &str) -> Manifest {
    Manifest {
        api_version: PROTOCOL_VERSION,
        id: "test".into(),
        name: "Test strip".into(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_freergb-plugin-virtual")),
        args: if fault.is_empty() {
            vec![]
        } else {
            vec!["--fault".into(), fault.into()]
        },
        access: Access::Virtual,
        layout: LedLayout::strip(3),
        timeout_ms: 500,
    }
}

#[test]
fn lifecycle_transfers_exact_rgb_and_reconnects() {
    let path = std::env::temp_dir().join(format!("freergb-plugin-{}.jsonl", std::process::id()));
    let mut m = manifest("");
    m.args = vec!["--log".into(), path.to_string_lossy().into_owned()];
    let mut device = ProcessDevice::new(m).unwrap();
    device.connect().unwrap();
    device.connect().unwrap(); // Idempotent; must not spawn or initialize twice.
    assert_eq!(device.status(), DeviceStatus::Connected);
    let frame = Frame::from_colors(vec![Rgb::RED, Rgb::GREEN, Rgb::BLUE]);
    assert!(device.push_frame(&Frame::solid(1, Rgb::WHITE)).is_err());
    device.push_frame(&frame).unwrap();
    device.disconnect(Some(Rgb::BLACK)).unwrap();
    device.disconnect(None).unwrap();
    let rows: Vec<serde_json::Value> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["method"], "initialize");
    assert_eq!(
        rows[1]["params"]["colors"],
        serde_json::to_value(frame.colors()).unwrap()
    );
    assert_eq!(
        rows[2]["params"]["final_color"],
        serde_json::to_value(Rgb::BLACK).unwrap()
    );
    assert_eq!(device.status(), DeviceStatus::Disconnected);
    device.connect().unwrap();
    device.push_frame(&frame).unwrap();
    device.disconnect(None).unwrap();
    std::fs::remove_file(path).unwrap();
}

#[test]
fn protocol_failures_and_hangs_are_bounded_and_faulted() {
    for fault in [
        "hang",
        "crash",
        "malformed",
        "oversized",
        "wrong-id",
        "wrong-version",
    ] {
        let started = Instant::now();
        let mut device = ProcessDevice::new(manifest(fault)).unwrap();
        let error = device.connect().expect_err(fault);
        if fault == "hang" {
            assert!(error.to_string().contains("timed out"));
        }
        assert!(
            matches!(device.status(), DeviceStatus::Faulted(_)),
            "{fault}"
        );
        // Includes OS process creation (outside the I/O timeout); cold Windows
        // executable startup can take several seconds on a loaded machine.
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "{fault}: {:?}",
            started.elapsed()
        );
        assert!(device.push_frame(&Frame::solid(3, Rgb::RED)).is_err());
    }
}

#[test]
fn frame_crash_and_shutdown_hang_do_not_block_host() {
    for fault in ["crash-frame", "hang-shutdown"] {
        let mut device = ProcessDevice::new(manifest(fault)).unwrap();
        device.connect().unwrap();
        let started = Instant::now();
        if fault == "crash-frame" {
            assert!(device.push_frame(&Frame::solid(3, Rgb::RED)).is_err());
        } else {
            assert!(device.disconnect(None).is_err());
        }
        assert!(matches!(device.status(), DeviceStatus::Faulted(_)));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

#[test]
fn manifests_reject_invalid_versions_layouts_paths_and_limits() {
    let mut m = manifest("");
    m.api_version = 2;
    assert!(m.validate().is_err());
    m = manifest("");
    m.id = "../escape".into();
    assert!(m.validate().is_err());
    m = manifest("");
    m.layout.width = 0;
    assert!(m.validate().is_err());
    m = manifest("");
    m.layout = LedLayout::strip(0);
    assert!(m.validate().is_err());
    m = manifest("");
    m.timeout_ms = 0;
    assert!(m.validate().is_err());
    m = manifest("");
    m.executable = "from-path".into();
    assert!(m.validate().is_err());
    m = manifest("");
    m.layout = LedLayout::strip(4097);
    assert!(m.validate().is_err());
}

#[test]
fn nonreading_child_cannot_block_a_large_request() {
    let mut m = manifest("no-read");
    m.layout = LedLayout::strip(4096);
    let mut device = ProcessDevice::new(m).unwrap();
    let started = Instant::now();
    assert!(device
        .connect()
        .unwrap_err()
        .to_string()
        .contains("timed out"));
    // Bounded, not fast: includes OS process creation, which can take several
    // seconds for a cold executable on a loaded Windows machine.
    assert!(started.elapsed() < Duration::from_secs(30));
}

#[test]
fn manifest_load_rejects_unknown_fields_and_resolves_paths_from_its_directory() {
    let dir = std::env::temp_dir().join(format!("freergb-plugin-manifest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let executable = dir.join("plugin-file");
    std::fs::write(&executable, "not executed").unwrap();
    let path = dir.join("manifest.json");
    let mut value = serde_json::to_value(manifest("")).unwrap();
    value["executable"] = serde_json::json!("plugin-file");
    std::fs::write(&path, value.to_string()).unwrap();
    assert_eq!(
        Manifest::load(&path).unwrap().executable,
        executable.canonicalize().unwrap()
    );
    value["unexpected"] = serde_json::json!(true);
    std::fs::write(&path, value.to_string()).unwrap();
    assert!(Manifest::load(&path).is_err());
    std::fs::write(&path, vec![b' '; freergb_plugins::MAX_MESSAGE + 1]).unwrap();
    assert!(Manifest::load(&path).is_err());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(executable).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
