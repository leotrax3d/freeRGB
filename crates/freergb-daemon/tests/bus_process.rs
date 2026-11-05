//! Signed-package -> real child -> daemon register model; never opens hardware.
#![cfg(feature = "bus-fixture")]
use ed25519_dalek::SigningKey;
use freergb_core::Rgb;
use freergb_daemon::bus_simulation::{RegisterBus, SimulatedBus};
use freergb_module_api::bus::{Operation, PciId, Policy, Rule};
use freergb_modules::catalog::{host_target, Artifact, Catalog, ModuleEntry, Origin};
use freergb_modules::package::{self, PackageManifest};
use freergb_modules::store::Store;
use freergb_modules::trust::{sign, TrustedKeys};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn pci() -> PciId {
    PciId {
        ven: 0x10de,
        dev: 1,
        subven: 0x1458,
        subdev: 2,
    }
}
struct Harness {
    root: PathBuf,
    store: Store,
    bus: Arc<SimulatedBus>,
}
impl Harness {
    fn new(mode: &str) -> Self {
        // Windows clocks are coarse: parallel tests need a counter, not only a timestamp.
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "freergb-bus-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let policy = Policy {
            pci: vec![pci()],
            backend: "simulation".into(),
            flat_registers: true,
            rules: vec![
                Rule {
                    address: 0x18,
                    first: 0,
                    last: 0,
                    op: Operation::SmbusReadByteData,
                    max_len: 1,
                    write_mask: 0,
                    write_value: 0,
                },
                Rule {
                    address: 0x18,
                    first: 0x20,
                    last: 0x25,
                    op: Operation::I2cWriteRegister,
                    max_len: 6,
                    write_mask: 255,
                    write_value: 0,
                },
                Rule {
                    address: 0x18,
                    first: 0x10,
                    last: 0x10,
                    op: Operation::I2cWriteRegister,
                    max_len: 1,
                    write_mask: 0,
                    write_value: 0,
                },
            ],
        };
        let manifest = PackageManifest {
            schema: 2,
            id: "bus-fixture".into(),
            version: "0.0.1".into(),
            family: "bus-fixture".into(),
            api: vec![2],
            target: host_target().into(),
            executable: format!("fixture{}", std::env::consts::EXE_SUFFIX),
            license: "MIT".into(),
            devices: vec![],
            bus: Some(policy),
            cooling: None,
            files: Default::default(),
        };
        let archive = root.join("fixture.zip");
        let executable = std::fs::read(env!("CARGO_BIN_EXE_freergb-bus-fixture")).unwrap();
        package::build(
            manifest.clone(),
            &[
                (manifest.executable.clone(), executable),
                ("LICENSE".into(), b"Synthetic test fixture; MIT".to_vec()),
                (
                    "PROVENANCE.md".into(),
                    b"Independent test fixture, not a controller port".to_vec(),
                ),
                ("fixture-mode".into(), mode.as_bytes().to_vec()),
            ],
            &archive,
        )
        .unwrap();
        let artifact = Artifact {
            target: host_target().into(),
            url: "fixture.zip".into(),
            size: std::fs::metadata(&archive).unwrap().len(),
            sha256: package::sha256_file(&archive).unwrap(),
        };
        let entry = ModuleEntry {
            id: manifest.id,
            version: manifest.version,
            family: manifest.family,
            name: "Synthetic fixture".into(),
            api: vec![2],
            license: "MIT".into(),
            origin: Origin {
                project: "freeRGB synthetic test".into(),
                url: "https://example.invalid/fixture".into(),
                commit: "0".repeat(40),
            },
            devices: vec![],
            artifacts: vec![artifact],
            source: Artifact {
                target: "source".into(),
                url: "fixture-source.zip".into(),
                size: 1,
                sha256: "0".repeat(64),
            },
            requires: vec![],
        };
        assert!(
            entry.compatible().is_err(),
            "HID routing must refuse bus packages"
        );
        let catalog = Catalog {
            schema: 2,
            sequence: 1,
            generated: "2026-10-24".into(),
            modules: vec![entry],
            relevant_vendors: vec![],
        };
        let bytes = serde_json::to_vec(&catalog).unwrap();
        let key = SigningKey::from_bytes(&[71; 32]);
        let keys = TrustedKeys::default().with_development_key(key.verifying_key());
        let signed = Catalog::verify(&bytes, sign(&key, &bytes).as_bytes(), &keys).unwrap();
        let store = Store::open(root.join("installed")).unwrap();
        store.install_bus(&signed.modules[0], &archive).unwrap();
        assert!(
            store.verified("bus-fixture").is_err(),
            "HID verification must refuse bus packages"
        );
        let mut registers = RegisterBus::default();
        let mut values = [0; 256];
        values[0] = 0xab;
        registers.attach(0x18, values);
        Self {
            root,
            store,
            bus: SimulatedBus::new(registers),
        }
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn colors() -> [Rgb; 2] {
    [Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }]
}

#[test]
fn signed_bus_module_initializes_frames_and_shuts_down_through_daemon() {
    let harness = Harness::new("ok");
    assert!(harness
        .bus
        .start_module(&harness.store, "bus-fixture", pci(), false)
        .is_err());
    assert!(harness.bus.trace().is_empty());
    let (mut process, caps) = harness
        .bus
        .start_module(&harness.store, "bus-fixture", pci(), true)
        .unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    process.frame(&colors()).unwrap();
    process.shutdown().unwrap();
    assert!(process.frame(&colors()).is_err());
    assert_eq!(
        harness.bus.trace(),
        vec![
            vec![0x30, 0, 0x31, 0xab],
            vec![0x30, 0x20, 1, 2, 3, 4, 5, 6],
            vec![0x30, 0x10, 0]
        ]
    );
}

#[test]
fn installed_policy_tampering_is_rejected_before_child_or_bus_io() {
    let harness = Harness::new("ok");
    let path = harness.root.join("installed/bus-fixture/0.0.1/module.json");
    let mut manifest: PackageManifest =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    manifest.bus.as_mut().unwrap().rules[1].last = 0x30;
    std::fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(harness
        .bus
        .start_module(&harness.store, "bus-fixture", pci(), true)
        .is_err());
    assert!(harness.bus.trace().is_empty());
}

#[test]
fn failing_children_cannot_bypass_policy_or_hang_the_host() {
    for mode in [
        "forbidden",
        "hide-denial",
        "crash",
        "hang",
        "oversize",
        "malformed",
        "wrong-request",
        "wrong-version",
    ] {
        let harness = Harness::new(mode);
        let (mut process, _) = harness
            .bus
            .start_module(&harness.store, "bus-fixture", pci(), true)
            .unwrap();
        let start = Instant::now();
        assert!(process.frame(&colors()).is_err(), "mode {mode}");
        assert!(start.elapsed() < Duration::from_secs(5), "mode {mode}");
        assert!(process.frame(&colors()).is_err());
        assert_eq!(
            harness.bus.trace(),
            vec![vec![0x30, 0, 0x31, 0xab]],
            "mode {mode}"
        );
    }
}

#[test]
fn handshake_rejects_wrong_identity_api_and_early_bus_io() {
    for mode in ["wrong-identity", "old-api", "io-hello", "refuse"] {
        let harness = Harness::new(mode);
        assert!(
            harness
                .bus
                .start_module(&harness.store, "bus-fixture", pci(), true)
                .is_err(),
            "{mode}"
        );
        assert!(harness.bus.trace().is_empty(), "{mode}");
    }
}

#[test]
fn process_crash_after_a_completed_write_quarantines_the_shared_bus() {
    use freergb_module_api::bus::{Call, Error, Hello, Transfer};
    use std::sync::atomic::AtomicBool;
    let harness = Harness::new("crash-after-write");
    let (mut process, _) = harness
        .bus
        .start_module(&harness.store, "bus-fixture", pci(), true)
        .unwrap();
    assert!(process.frame(&colors()).is_err());
    let policy = harness
        .store
        .verified_bus("bus-fixture")
        .unwrap()
        .policy()
        .clone();
    let session = harness
        .bus
        .session(
            &Hello {
                v: 2,
                api: vec![2],
                transport: "simulation".into(),
            },
            policy,
            pci(),
            true,
        )
        .unwrap();
    let call = Call {
        v: 2,
        call: 1,
        capability: session.capability(),
        transfers: vec![Transfer {
            op: Operation::SmbusReadByteData,
            address: 0x18,
            register: 0,
            data: vec![],
            read_len: 1,
        }],
    };
    assert_eq!(
        session.call(&call, &AtomicBool::new(false)).result,
        Err(Error::Faulted)
    );
    assert_eq!(
        harness.bus.trace(),
        vec![
            vec![0x30, 0, 0x31, 0xab],
            vec![0x30, 0x20, 1, 2, 3, 4, 5, 6]
        ]
    );
}
