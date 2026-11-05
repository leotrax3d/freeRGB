use crate::catalog::*;
use crate::package::{self, PackageManifest};
use crate::setup::{device_key, Manager, Phase, Seen};
use crate::source::Source;
use crate::store::Store;
use crate::trust::{sign, TrustedKeys};
use ed25519_dalek::SigningKey;
use freergb_module_api::{DeviceKind, InterfaceInfo};
use sha2::Digest;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "freergb-modules-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

#[test]
fn public_distribution_has_a_compiled_release_trust_anchor() {
    assert!(TrustedKeys::builtin().has_release_key());
}

fn keys() -> TrustedKeys {
    TrustedKeys::default().with_development_key(key().verifying_key())
}

fn mouse() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1532,
        pid: 0x00A3,
        interface: 0,
        usage_page: 1,
        usage: 2,
        product: Some("Razer Cobra".into()),
    }
}

fn device_match(pid: u16) -> DeviceMatch {
    DeviceMatch {
        name: format!("Mouse {pid:04X}"),
        kind: DeviceKind::Mouse,
        vid: 0x1532,
        pid,
        interface: Some(0),
        usage_page: Some(1),
        usage: Some(2),
    }
}

fn manifest(version: &str) -> PackageManifest {
    PackageManifest {
        schema: 1,
        id: "razer-chroma".into(),
        version: version.into(),
        family: "razer-chroma".into(),
        api: vec![1],
        target: host_target().into(),
        executable: "module.exe".into(),
        license: "GPL-2.0-or-later".into(),
        devices: vec![device_match(0x00A3), device_match(0x00AF)],
        bus: None,
        cooling: None,
        files: Default::default(),
    }
}

#[test]
fn bus_manifest_is_explicit_v2_and_cannot_be_launched_as_hid() {
    use freergb_module_api::bus::{Operation, PciId, Policy, Rule};
    let mut manifest = manifest("1.0.0");
    manifest.files.insert("module.exe".into(), "0".repeat(64));
    manifest.files.insert("LICENSE".into(), "0".repeat(64));
    manifest
        .files
        .insert("PROVENANCE.md".into(), "0".repeat(64));
    manifest.schema = 2;
    manifest.api = vec![2];
    manifest.devices.clear();
    manifest.bus = Some(Policy {
        backend: "simulation".into(),
        flat_registers: true,
        pci: vec![PciId {
            ven: 1,
            dev: 2,
            subven: 3,
            subdev: 4,
        }],
        rules: vec![Rule {
            address: 0x18,
            first: 0,
            last: 255,
            op: Operation::I2cWriteRegister,
            max_len: 32,
            write_mask: 255,
            write_value: 0,
        }],
    });
    manifest.validate().unwrap();
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let signature = sign(&key(), &bytes);
    keys().verify(&bytes, signature.as_bytes()).unwrap();
    let mut changed = manifest.clone();
    changed.bus.as_mut().unwrap().rules[0].address = 0x19;
    assert!(keys()
        .verify(&serde_json::to_vec(&changed).unwrap(), signature.as_bytes())
        .is_err());
    changed.bus.as_mut().unwrap().rules[0].address = 0x50;
    assert!(changed.validate().is_err());
    changed = manifest.clone();
    changed.api = vec![1, 2];
    assert!(changed.validate().is_err());
    changed = manifest.clone();
    changed.schema = 1;
    assert!(changed.validate().is_err());
    changed = manifest;
    changed.devices.push(device_match(3));
    assert!(changed.validate().is_err());
}

/// Builds a package plus its catalog entry in `dir`.
fn publish(dir: &Path, version: &str, executable: &[u8]) -> ModuleEntry {
    let name = format!("razer-chroma-{version}.zip");
    let files = vec![
        ("module.exe".to_string(), executable.to_vec()),
        ("COPYING".to_string(), b"GPL text".to_vec()),
        ("PROVENANCE.md".to_string(), b"from OpenRGB".to_vec()),
    ];
    package::build(manifest(version), &files, &dir.join(&name)).unwrap();
    let bytes = std::fs::read(dir.join(&name)).unwrap();
    ModuleEntry {
        id: "razer-chroma".into(),
        version: version.into(),
        family: "razer-chroma".into(),
        name: "Razer Chroma".into(),
        api: vec![1],
        license: "GPL-2.0-or-later".into(),
        origin: Origin {
            project: "OpenRGB".into(),
            url: "https://gitlab.com/CalcProgrammer1/OpenRGB".into(),
            commit: "bbc77470a7cba56af60989e54ba910d5ac66a3ac".into(),
        },
        devices: manifest(version).devices,
        artifacts: vec![Artifact {
            target: host_target().into(),
            url: name,
            size: bytes.len() as u64,
            sha256: hex::encode(sha2::Sha256::digest(&bytes)),
        }],
        source: Artifact {
            target: "source".into(),
            url: "razer-chroma-src.zip".into(),
            size: 1,
            sha256: "0".repeat(64),
        },
        requires: vec![],
    }
}

fn write_catalog(dir: &Path, sequence: u64, modules: Vec<ModuleEntry>) {
    let catalog = Catalog {
        schema: 1,
        sequence,
        generated: "2026-10-22".into(),
        modules,
        relevant_vendors: vec![Vendor {
            vid: 0x1532,
            name: "Razer".into(),
        }],
    };
    let bytes = serde_json::to_vec(&catalog).unwrap();
    std::fs::write(dir.join("catalog.json"), &bytes).unwrap();
    std::fs::write(dir.join("catalog.json.sig"), sign(&key(), &bytes)).unwrap();
}

#[test]
fn bus_catalog_schema_is_explicit_and_legacy_api_advertisements_stay_valid() {
    let root = temp("bus-catalog-schema");
    let mut entry = publish(&root, "0.0.1", b"synthetic test executable");
    entry.api = vec![1, 2];
    let mut catalog = Catalog {
        schema: 1,
        sequence: 1,
        generated: "2026-10-24".into(),
        modules: vec![entry],
        relevant_vendors: vec![],
    };
    catalog.validate().unwrap();
    catalog.modules[0].api = vec![2];
    catalog.modules[0].devices.clear();
    assert!(catalog.validate().is_err());
    catalog.schema = 2;
    catalog.validate().unwrap();
    catalog.modules[0].devices.push(device_match(3));
    assert!(catalog.validate().is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn catalog_requires_a_trusted_signature() {
    let dir = temp("sig");
    write_catalog(&dir, 1, vec![publish(&dir, "0.1.0", b"exe")]);
    let bytes = std::fs::read(dir.join("catalog.json")).unwrap();
    let sig = std::fs::read(dir.join("catalog.json.sig")).unwrap();
    let catalog = Catalog::verify(&bytes, &sig, &keys()).unwrap();
    assert_eq!(catalog.match_interface(&mouse()).unwrap().1.pid, 0x00A3);
    let mut tampered = bytes.clone();
    let at = tampered.iter().position(|&b| b == b'1').unwrap();
    tampered[at] = b'2';
    assert!(Catalog::verify(&tampered, &sig, &keys()).is_err());
    let other = TrustedKeys::default()
        .with_development_key(SigningKey::from_bytes(&[9u8; 32]).verifying_key());
    assert!(Catalog::verify(&bytes, &sig, &other).is_err());
    assert!(
        Catalog::verify(&bytes, &sig, &TrustedKeys::default()).is_err(),
        "no key, no trust"
    );
    // A signature without the domain prefix is not accepted either.
    use ed25519_dalek::Signer;
    let raw = hex::encode(key().sign(&bytes).to_bytes());
    assert!(Catalog::verify(&bytes, raw.as_bytes(), &keys()).is_err());
}

#[test]
fn matching_respects_interface_usage_and_variants() {
    let m = device_match(0x00A3);
    assert!(m.matches(&mouse()));
    for change in [
        |i: &mut InterfaceInfo| i.interface = 1,
        |i: &mut InterfaceInfo| i.usage = 6,
        |i: &mut InterfaceInfo| i.pid = 0x00A4,
    ] {
        let mut other = mouse();
        change(&mut other);
        assert!(!m.matches(&other));
    }
}

#[test]
fn package_extraction_rejects_unsafe_and_tampered_content() {
    let dir = temp("pkg");
    publish(&dir, "0.1.0", b"exe");
    let out = temp("pkg-out");
    let (m, _) = package::extract(&dir.join("razer-chroma-0.1.0.zip"), &out).unwrap();
    assert_eq!(m.version, "0.1.0");
    // Parent path entry.
    let evil = dir.join("evil.zip");
    {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
        zip.start_file("../module.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"{}").unwrap();
        zip.finish().unwrap();
    }
    assert!(package::extract(&evil, &temp("pkg-evil")).is_err());
    // A file whose hash differs from module.json.
    let bad = dir.join("bad.zip");
    {
        use std::io::Write;
        let mut m = manifest("0.1.0");
        for (n, b) in [
            ("module.exe", &b"exe"[..]),
            ("COPYING", b"x"),
            ("PROVENANCE.md", b"y"),
        ] {
            m.files
                .insert(n.into(), hex::encode(sha2::Sha256::digest(b)));
        }
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&bad).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        zip.start_file("module.json", o).unwrap();
        zip.write_all(&serde_json::to_vec(&m).unwrap()).unwrap();
        for (n, b) in [
            ("module.exe", &b"EVIL"[..]),
            ("COPYING", b"x"),
            ("PROVENANCE.md", b"y"),
        ] {
            zip.start_file(n, o).unwrap();
            zip.write_all(b).unwrap();
        }
        zip.finish().unwrap();
    }
    assert!(package::extract(&bad, &temp("pkg-bad")).is_err());
}

#[test]
fn extraction_never_overwrites_existing_files_or_follows_directory_links() {
    let repo = temp("pkg-existing");
    publish(&repo, "0.1.0", b"exe");
    let package = repo.join("razer-chroma-0.1.0.zip");
    let destination = temp("pkg-existing-out");
    let existing = destination.join("module.json");
    std::fs::write(&existing, b"preserve user data").unwrap();
    assert!(package::extract(&package, &destination).is_err());
    assert_eq!(std::fs::read(existing).unwrap(), b"preserve user data");
    #[cfg(unix)]
    {
        let target = temp("pkg-link-target");
        let linked = repo.join("linked");
        std::os::unix::fs::symlink(&target, &linked).unwrap();
        assert!(package::extract(&package, &linked).is_err());
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 0);
        std::fs::remove_dir_all(target).unwrap();
    }
    let malicious = repo.join("linked-entry.zip");
    let mut archive = zip::ZipWriter::new(std::fs::File::create(&malicious).unwrap());
    archive
        .add_symlink(
            "module.json",
            "../outside",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    archive.finish().unwrap();
    let empty = temp("pkg-reject-link");
    assert!(package::extract(&malicious, &empty).is_err());
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    for path in [repo, destination, empty] {
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn install_is_atomic_verified_before_use_and_can_roll_back() {
    let repo = temp("repo");
    let store = Store::open(temp("store")).unwrap();
    let v1 = publish(&repo, "0.1.0", b"one");
    store
        .install(&v1, &repo.join("razer-chroma-0.1.0.zip"))
        .unwrap();
    store.mark_good("razer-chroma").unwrap();
    // A package that does not match its entry leaves 0.1.0 active.
    let mut v2 = publish(&repo, "0.2.0", b"two");
    let mut wrong = v2.clone();
    wrong.artifacts[0].sha256 = "1".repeat(64);
    assert!(store
        .install(&wrong, &repo.join("razer-chroma-0.2.0.zip"))
        .is_err());
    assert_eq!(
        store.verified("razer-chroma").unwrap().manifest.version,
        "0.1.0"
    );
    // Entry/package mismatch (catalog says other devices) is refused too.
    let mut lying = v2.clone();
    lying.devices.pop();
    assert!(store
        .install(&lying, &repo.join("razer-chroma-0.2.0.zip"))
        .is_err());
    store
        .install(&v2, &repo.join("razer-chroma-0.2.0.zip"))
        .unwrap();
    let active = store.verified("razer-chroma").unwrap();
    assert_eq!(active.manifest.version, "0.2.0");
    assert_eq!(std::fs::read(&active.executable).unwrap(), b"two");
    // Modified executable is detected before execution.
    std::fs::write(&active.executable, b"tampered").unwrap();
    assert!(store.verified("razer-chroma").is_err());
    assert_eq!(store.rollback("razer-chroma").unwrap(), "0.1.0");
    assert_eq!(
        std::fs::read(store.verified("razer-chroma").unwrap().executable).unwrap(),
        b"one"
    );
    // Incompatible protocol version is refused at install time.
    v2.api = vec![99];
    assert!(store
        .install(&v2, &repo.join("razer-chroma-0.2.0.zip"))
        .is_err());
    // No staging leftovers.
    assert!(std::fs::read_dir(store.root())
        .unwrap()
        .flatten()
        .all(|e| !e.file_name().to_string_lossy().starts_with(".staging-")));
}

fn seen(i: InterfaceInfo) -> Seen {
    Seen {
        interface: i,
        manufacturer: Some("Razer".into()),
        builtin: None,
    }
}

#[test]
fn shared_module_downloads_once_and_counts_bytes_once() {
    let repo = temp("shared");
    let entry = publish(&repo, "0.1.0", b"exe");
    let size = entry.artifacts[0].size;
    write_catalog(&repo, 3, vec![entry]);
    let manager = Manager::new(
        Store::open(temp("shared-store")).unwrap(),
        keys(),
        Source::Directory(repo.clone()),
        temp("shared-cfg"),
    );
    manager.refresh_catalog();
    assert_eq!(manager.status().catalog.state, "ready");
    let mut pro = mouse();
    pro.pid = 0x00AF;
    let mut unknown = mouse();
    unknown.pid = 0x0BAD;
    let mut other_vendor = mouse();
    other_vendor.vid = 0x046D;
    let list = [
        seen(mouse()),
        seen(pro.clone()),
        seen(unknown),
        seen(other_vendor),
    ];
    manager.observe(&list, &BTreeMap::new());
    let status = manager.status();
    assert_eq!(status.cards.len(), 3, "irrelevant vendor is hidden");
    let unsupported = status
        .cards
        .iter()
        .find(|c| c.phase == Phase::Unsupported)
        .unwrap();
    assert!(
        unsupported.technical.contains("0x0BAD")
            && !unsupported.technical.to_lowercase().contains("serial")
    );
    let keys = [device_key(&mouse()), device_key(&pro)];
    manager.run(&keys);
    let status = manager.status();
    assert_eq!(status.job.bytes_total, size, "one package, counted once");
    assert_eq!(status.job.modules, vec!["razer-chroma".to_string()]);
    for key in &keys {
        let card = status.cards.iter().find(|c| &c.key == key).unwrap();
        assert_eq!(
            card.phase,
            Phase::Initializing,
            "download done is not ready"
        );
        assert_eq!(card.download_size, Some(size));
    }
    assert_eq!(manager.configured().len(), 2);
    // Running again reuses the installed module: nothing to download.
    manager.run(&keys);
    assert_eq!(manager.status().job.bytes_total, 0);
    // Engine reports connected -> Ready; unplugged -> NotConnected, no new setup.
    let runtime: BTreeMap<_, _> = [(
        keys[0].clone(),
        crate::setup::Runtime {
            engine_id: "module:x".into(),
            connected: true,
            error: None,
        },
    )]
    .into();
    manager.observe(&list, &runtime);
    assert_eq!(
        manager
            .status()
            .cards
            .iter()
            .find(|c| c.key == keys[0])
            .unwrap()
            .phase,
        Phase::Ready
    );
    manager.observe(&list[1..], &BTreeMap::new());
    assert_eq!(
        manager
            .status()
            .cards
            .iter()
            .find(|c| c.key == keys[0])
            .unwrap()
            .phase,
        Phase::NotConnected
    );
    manager.observe(&list, &runtime);
    assert_eq!(
        manager
            .status()
            .cards
            .iter()
            .find(|c| c.key == keys[0])
            .unwrap()
            .phase,
        Phase::Ready
    );
}

#[test]
fn offline_and_rollback_catalogs_keep_installed_state() {
    let repo = temp("offline");
    write_catalog(&repo, 5, vec![publish(&repo, "0.1.0", b"exe")]);
    let config = temp("offline-cfg");
    let store_dir = temp("offline-store");
    let manager = Manager::new(
        Store::open(store_dir.clone()).unwrap(),
        keys(),
        Source::Directory(repo.clone()),
        config.clone(),
    );
    manager.refresh_catalog();
    manager.observe(&[seen(mouse())], &BTreeMap::new());
    manager.run(&[device_key(&mouse())]);
    // Older catalog (sequence 4) is refused, cached one stays.
    write_catalog(&repo, 4, vec![publish(&repo, "0.1.0", b"exe")]);
    manager.refresh_catalog();
    let status = manager.status();
    assert_eq!(status.catalog.state, "cached");
    assert!(status.catalog.message.unwrap().contains("rollback"));
    // Source gone (offline): a new manager still matches via installed module.
    let gone = Manager::new(
        Store::open(store_dir).unwrap(),
        keys(),
        Source::Directory(temp("nothing")),
        config,
    );
    gone.refresh_catalog();
    gone.observe(&[seen(mouse())], &BTreeMap::new());
    let card = &gone.status().cards[0];
    assert!(card.installed && card.configured, "{card:?}");
    assert_eq!(card.phase, Phase::Initializing);
}

#[test]
fn failed_download_reports_cause_and_can_be_cancelled() {
    let repo = temp("broken");
    let mut entry = publish(&repo, "0.1.0", b"exe");
    entry.artifacts[0].sha256 = "2".repeat(64);
    write_catalog(&repo, 1, vec![entry]);
    let manager = Manager::new(
        Store::open(temp("broken-store")).unwrap(),
        keys(),
        Source::Directory(repo),
        temp("broken-cfg"),
    );
    manager.refresh_catalog();
    manager.observe(&[seen(mouse())], &BTreeMap::new());
    manager.run(&[device_key(&mouse())]);
    let card = manager.status().cards[0].clone();
    assert_eq!(card.phase, Phase::Failed);
    assert_eq!(card.error.unwrap().code, "invalid_package");
    assert!(manager.configured().is_empty());
    assert!(manager.store().all_verified().is_empty());
}
