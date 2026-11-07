//! End to end without hardware: signed catalog -> download -> verified
//! install -> the *installed* razer-chroma module process -> brokered feature
//! reports to a simulated Razer Cobra.
//!
//! Needs the module binary first (it is a separate GPL program):
//!   cargo build -p freergb-module-razer-chroma
//!   cargo test -p freergb-modules --test e2e -- --ignored
use ed25519_dalek::SigningKey;
use freergb_core::{Frame, Rgb};
use freergb_device_api::{DeviceId, HidTransport, RgbDevice, TransportError};
use freergb_module_api::InterfaceInfo;
use freergb_modules::catalog::*;
use freergb_modules::host::{Launch, ModuleDevice};
use freergb_modules::package::{self, PackageManifest};
use freergb_modules::setup::{device_key, Manager, Phase, Seen};
use freergb_modules::source::Source;
use freergb_modules::store::Store;
use freergb_modules::trust::{sign, TrustedKeys};
use sha2::Digest;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn module_binary() -> PathBuf {
    // target/<profile>/deps/e2e-xxxx -> target/<profile>/
    let dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    dir.join(format!(
        "freergb-module-razer-chroma{}",
        std::env::consts::EXE_SUFFIX
    ))
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("freergb-e2e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Simulated Cobra: records feature reports; answers queries like firmware.
struct Cobra(Arc<Mutex<Vec<Vec<u8>>>>);

impl HidTransport for Cobra {
    fn write(&mut self, _: &[u8]) -> Result<usize, TransportError> {
        Err(TransportError::Io("unexpected output report".into()))
    }
    fn read_timeout(&mut self, _: &mut [u8], _: i32) -> Result<usize, TransportError> {
        Ok(0)
    }
    fn output_report_len(&self) -> usize {
        1
    }
    fn input_report_len(&self) -> usize {
        1
    }
    fn send_feature(&mut self, report: &[u8]) -> Result<usize, TransportError> {
        self.0.lock().unwrap().push(report.to_vec());
        Ok(report.len())
    }
    fn get_feature(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        let mut reply = self.0.lock().unwrap().last().cloned().unwrap_or_default();
        reply.resize(buf.len(), 0);
        reply[1] = 0x02; // success
        reply[9] = 1;
        reply[10] = 7;
        buf.copy_from_slice(&reply);
        Ok(buf.len())
    }
}

#[test]
#[ignore = "needs the razer-chroma module binary; see file header"]
fn signed_catalog_to_installed_module_driving_a_simulated_cobra() {
    let exe = module_binary();
    assert!(
        exe.is_file(),
        "build freergb-module-razer-chroma first: {}",
        exe.display()
    );
    let repo = temp("repo");
    let target = host_target();
    let cobra = DeviceMatch {
        name: "Razer Cobra".into(),
        kind: freergb_module_api::DeviceKind::Mouse,
        vid: 0x1532,
        pid: 0x00A3,
        interface: Some(0),
        usage_page: Some(1),
        usage: Some(2),
    };
    let manifest = PackageManifest {
        schema: 1,
        id: "razer-chroma".into(),
        version: "0.1.0".into(),
        family: "razer-chroma".into(),
        api: vec![1],
        target: target.into(),
        executable: exe.file_name().unwrap().to_string_lossy().into(),
        license: "GPL-2.0-or-later".into(),
        devices: vec![cobra],
        bus: None,
        cooling: None,
        probe: None,
        files: Default::default(),
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/razer-chroma");
    let files = vec![
        (manifest.executable.clone(), std::fs::read(&exe).unwrap()),
        (
            "COPYING".into(),
            std::fs::read(root.join("COPYING")).unwrap(),
        ),
        (
            "PROVENANCE.md".into(),
            std::fs::read(root.join("PROVENANCE.md")).unwrap(),
        ),
    ];
    package::build(manifest.clone(), &files, &repo.join("razer.zip")).unwrap();
    let bytes = std::fs::read(repo.join("razer.zip")).unwrap();
    let entry = ModuleEntry {
        id: "razer-chroma".into(),
        version: "0.1.0".into(),
        family: "razer-chroma".into(),
        name: "Razer Chroma".into(),
        api: vec![1],
        license: "GPL-2.0-or-later".into(),
        origin: Origin {
            project: "OpenRGB".into(),
            url: "https://gitlab.com/CalcProgrammer1/OpenRGB".into(),
            commit: "bbc77470a7cba56af60989e54ba910d5ac66a3ac".into(),
        },
        devices: manifest.devices.clone(),
        artifacts: vec![Artifact {
            target: target.into(),
            url: "razer.zip".into(),
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
        schema: 1,
        sequence: 1,
        generated: String::new(),
        modules: vec![entry],
        relevant_vendors: vec![],
    })
    .unwrap();
    let key = SigningKey::from_bytes(&[3u8; 32]);
    std::fs::write(repo.join("catalog.json"), &catalog).unwrap();
    std::fs::write(repo.join("catalog.json.sig"), sign(&key, &catalog)).unwrap();

    let manager = Manager::new(
        Store::open(temp("store")).unwrap(),
        TrustedKeys::default().with_development_key(key.verifying_key()),
        Source::Directory(repo),
        temp("cfg"),
    );
    manager.refresh_catalog();
    let interface = InterfaceInfo {
        vid: 0x1532,
        pid: 0x00A3,
        interface: 0,
        usage_page: 1,
        usage: 2,
        product: Some("Razer Cobra".into()),
    };
    manager.observe(
        &[Seen {
            interface: interface.clone(),
            manufacturer: None,
            builtin: None,
        }],
        &Default::default(),
    );
    manager.run(&[device_key(&interface)]);
    assert_eq!(manager.status().cards[0].phase, Phase::Initializing);

    // What the daemon does next: verify, then start the installed module.
    let verified = manager.store().verified("razer-chroma").unwrap();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let wire = sent.clone();
    let launch = Launch {
        module_id: "razer-chroma".into(),
        executable: verified.executable,
        directory: verified.directory,
    };
    let mut device = ModuleDevice::new(
        DeviceId("module:razer-chroma:test".into()),
        "Razer Cobra".into(),
        launch,
        interface,
        Box::new(move || Ok(Box::new(Cobra(wire.clone())) as Box<dyn HidTransport>)),
    );
    device.connect().unwrap();
    assert_eq!(device.info().name, "Razer Cobra");
    assert_eq!(device.info().firmware.as_deref(), Some("v1.7"));
    sent.lock().unwrap().clear();
    device
        .push_frame(&Frame::from_colors(vec![Rgb { r: 0, g: 255, b: 0 }]))
        .unwrap();
    let reports = sent.lock().unwrap().clone();
    assert_eq!(reports.len(), 2, "row report + custom-frame mode report");
    assert_eq!(reports[0][7..17], [0x0F, 0x03, 0, 0, 0, 0, 0, 0, 255, 0]);
    device.disconnect(None).unwrap();
}
