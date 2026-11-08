//! End to end without hardware: signed schema-3 catalog -> download ->
//! verified install -> the *installed* virtual cooling plugin process ->
//! samples, leased control, pump rule and clean release.
//!
//! Needs the plugin binary first:
//!   cargo build -p freergb-cooling-virtual
//!   cargo test -p freergb-modules --test cooling_e2e -- --ignored
use ed25519_dalek::SigningKey;
use freergb_module_api::cooling::{Policy, API};
use freergb_modules::catalog::*;
use freergb_modules::cooling_host::CoolingPlugin;
use freergb_modules::package::{self, PackageManifest};
use freergb_modules::setup::Manager;
use freergb_modules::source::Source;
use freergb_modules::store::Store;
use freergb_modules::trust::{sign, TrustedKeys};
use sha2::Digest;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn temp(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("freergb-cooling-e2e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn plugin_binary() -> PathBuf {
    let dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    dir.join(format!(
        "freergb-cooling-virtual{}",
        std::env::consts::EXE_SUFFIX
    ))
}

#[test]
#[ignore = "needs the freergb-cooling-virtual binary; see file header"]
fn signed_catalog_to_installed_virtual_plugin() {
    let exe = plugin_binary();
    assert!(exe.is_file(), "build freergb-cooling-virtual first");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/cooling-virtual");
    let policy: Policy =
        serde_json::from_slice(&std::fs::read(root.join("cooling-policy.json")).unwrap()).unwrap();
    let repo = temp("repo");
    let target = host_target();
    let manifest = PackageManifest {
        schema: 3,
        id: "cooling-virtual".into(),
        version: "0.1.0".into(),
        family: "virtual".into(),
        api: vec![API],
        target: target.into(),
        executable: exe.file_name().unwrap().to_string_lossy().into(),
        license: "MIT OR Apache-2.0".into(),
        devices: vec![],
        bus: None,
        cooling: Some(policy),
        probe: None,
        transport: None,
        files: Default::default(),
    };
    let files = vec![
        (manifest.executable.clone(), std::fs::read(&exe).unwrap()),
        (
            "LICENSE".into(),
            std::fs::read(root.join("LICENSE")).unwrap(),
        ),
        (
            "PROVENANCE.md".into(),
            std::fs::read(root.join("PROVENANCE.md")).unwrap(),
        ),
    ];
    package::build(manifest, &files, &repo.join("virtual.zip")).unwrap();
    let bytes = std::fs::read(repo.join("virtual.zip")).unwrap();
    let entry = ModuleEntry {
        id: "cooling-virtual".into(),
        version: "0.1.0".into(),
        family: "virtual".into(),
        name: "Virtual cooling".into(),
        api: vec![API],
        license: "MIT OR Apache-2.0".into(),
        origin: Origin {
            project: "freeRGB".into(),
            url: "https://github.com/leotrax3d/freeRGB".into(),
            commit: "0000000".into(),
        },
        devices: vec![],
        artifacts: vec![Artifact {
            target: target.into(),
            url: "virtual.zip".into(),
            size: bytes.len() as u64,
            sha256: hex::encode(sha2::Sha256::digest(&bytes)),
        }],
        source: Artifact {
            target: "source".into(),
            url: "src.zip".into(),
            size: 1,
            sha256: "0".repeat(64),
        },
        requires: vec![],
    };
    let catalog = serde_json::to_vec(&Catalog {
        schema: COOLING_CATALOG_SCHEMA,
        sequence: 1,
        generated: String::new(),
        modules: vec![entry],
        relevant_vendors: vec![],
    })
    .unwrap();
    let key = SigningKey::from_bytes(&[5u8; 32]);
    std::fs::write(repo.join("catalog.json"), &catalog).unwrap();
    std::fs::write(repo.join("catalog.json.sig"), sign(&key, &catalog)).unwrap();
    let store_root = temp("store");
    let manager = Manager::new(
        Store::open(store_root.clone()).unwrap(),
        TrustedKeys::default().with_development_key(key.verifying_key()),
        Source::Directory(repo),
        temp("cfg"),
    );
    manager.refresh_catalog();
    assert_eq!(manager.cooling_catalog().len(), 1);
    manager.install_cooling("cooling-virtual").unwrap();

    let verified = Store::open(store_root)
        .unwrap()
        .verified_cooling("cooling-virtual")
        .unwrap();
    assert!(
        CoolingPlugin::start(&verified, Some(Box::new(NoDevice))).is_err(),
        "a virtual plugin never receives a device handle"
    );
    let mut plugin = CoolingPlugin::start(&verified, None).unwrap();
    let now = Instant::now();
    plugin.sample(now).unwrap();
    let value = |p: &CoolingPlugin, id: &str, at| {
        p.readings(at)
            .into_iter()
            .find(|r| r.id == id)
            .unwrap()
            .value
    };
    assert_eq!(
        value(&plugin, "fan1_rpm", now),
        Some(1080.0),
        "firmware 60 %"
    );
    plugin.set_duty("fan1", 50, now).unwrap();
    assert!(plugin.set_duty("pump", 80, now).is_err());
    let later = now + Duration::from_millis(1000);
    plugin.sample(later).unwrap();
    assert_eq!(value(&plugin, "fan1_rpm", later), Some(900.0));
    plugin.release("fan1").unwrap();
    plugin.shutdown().unwrap();
    assert!(plugin.status().failsafe.is_none());
}

struct NoDevice;

impl freergb_device_api::HidTransport for NoDevice {
    fn write(&mut self, _: &[u8]) -> Result<usize, freergb_device_api::TransportError> {
        Ok(0)
    }
    fn read_timeout(
        &mut self,
        _: &mut [u8],
        _: i32,
    ) -> Result<usize, freergb_device_api::TransportError> {
        Ok(0)
    }
    fn output_report_len(&self) -> usize {
        1
    }
    fn input_report_len(&self) -> usize {
        1
    }
}
