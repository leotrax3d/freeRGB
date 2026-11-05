use freergb_core::{Canvas, Rgb};
use freergb_effect_host::{
    lifecycle::ManagedEffect,
    marketplace::{Catalog, Manager},
    process::{HostCommand, ProcessEffect},
    runtime::WasmEffect,
};
use freergb_effects::Effect;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};
static PROCESS_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn host() -> HostCommand {
    HostCommand {
        executable: env!("CARGO_BIN_EXE_freergb-effect-worker").into(),
        args: vec![],
    }
}
fn module(body: &str) -> Vec<u8> {
    wat::parse_str(format!(r#"(module (memory (export "memory") 1) (func (export "set_param") (param i32 f64)) (func (export "render") (param f64 i32 i32) (result i32) {body}))"#)).unwrap()
}
#[test]
fn real_worker_bounds_guest_and_isolates_runtime_faults() {
    let _serial = PROCESS_TEST.lock().unwrap_or_else(|e| e.into_inner());
    for bytes in [
        b"invalid".to_vec(),
        module("unreachable"),
        module("(loop br 0) i32.const 0"),
        module("i32.const -1"),
        module("i32.const 65535"),
    ] {
        assert!(ProcessEffect::load(&host(), &bytes, &[]).is_err());
    }
    let bytes = module("local.get 0 f64.const 0 f64.gt if unreachable end i32.const 0");
    let mut effect = ProcessEffect::load(&host(), &bytes, &[]).unwrap();
    assert!(effect
        .frame(Duration::from_secs(1), &mut Canvas::new(2, 2))
        .is_err());
    let mut healthy = ProcessEffect::load(&host(), &module("i32.const 0"), &[]).unwrap();
    healthy
        .frame(Duration::ZERO, &mut Canvas::new(300, 200))
        .unwrap();
}
#[test]
fn activation_from_async_context_does_not_nest_runtimes() {
    let _serial = PROCESS_TEST.lock().unwrap_or_else(|e| e.into_inner());
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _effect = ProcessEffect::load(&host(), &module("i32.const 0"), &[]).unwrap();
        });
}
#[cfg(feature = "effect-fixture")]
#[test]
fn broken_native_peers_have_bounded_deadlines_and_cleanup() {
    let _serial = PROCESS_TEST.lock().unwrap_or_else(|e| e.into_inner());
    for mode in [
        "crash",
        "hang",
        "no-read",
        "malformed",
        "oversized",
        "wrong-id",
        "wrong-version",
    ] {
        let host = HostCommand {
            executable: env!("CARGO_BIN_EXE_freergb-effect-fixture").into(),
            args: vec![mode.into()],
        };
        let start = Instant::now();
        assert!(
            ProcessEffect::load(&host, &module("i32.const 0"), &[]).is_err(),
            "{mode}"
        );
        assert!(start.elapsed() < Duration::from_secs(30), "{mode}");
    }
}
struct Fixture {
    root: PathBuf,
    release: PathBuf,
    manager: std::sync::Arc<Manager>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "freergb-lifecycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let release = root.join("release");
        std::fs::create_dir_all(&release).unwrap();
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../effects/catalog.json"),
            release.join("catalog.json"),
        )
        .unwrap();
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../effects/packages/aurora-1.0.0.wasm"),
            release.join("aurora-1.0.0.wasm"),
        )
        .unwrap();
        let manager = Manager::development(root.join("store"), release.clone()).unwrap();
        manager.refresh().unwrap();
        wait(&manager);
        manager.install("aurora").unwrap();
        wait(&manager);
        assert!(manager.view().error.is_none(), "{:?}", manager.view().error);
        Self {
            root,
            release,
            manager,
        }
    }
    fn replace(&self, bytes: &[u8]) {
        let mut catalog =
            Catalog::parse(&std::fs::read(self.release.join("catalog.json")).unwrap()).unwrap();
        let entry = &mut catalog.effects[0];
        entry.artifact.size = bytes.len() as u64;
        entry.artifact.sha256 = hex::encode(Sha256::digest(bytes));
        std::fs::write(self.root.join("store/packages/aurora/1.0.0.wasm"), bytes).unwrap();
        std::fs::write(
            self.root.join("store/packages/aurora/1.0.0.json"),
            serde_json::to_vec(entry).unwrap(),
        )
        .unwrap();
    }
    fn effect(&self) -> (ManagedEffect, freergb_effect_host::runtime::Fault) {
        let fault = WasmEffect::fault();
        (
            ManagedEffect::load_with_host(
                self.manager.clone(),
                "aurora",
                "1.0.0",
                &BTreeMap::new(),
                fault.clone(),
                host(),
            )
            .unwrap(),
            fault,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn wait(m: &Manager) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while m.busy() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !condition() {
        assert!(Instant::now() < deadline, "condition timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn reload_during_render_rejects_bad_candidates_and_rolls_back_later_trap() {
    let _serial = PROCESS_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let f = Fixture::new();
    let (mut effect, fault) = f.effect();
    let mut canvas = Canvas::new(1, 1);
    f.replace(&module("i32.const 0 i32.const 255 i32.store8 i32.const 0"));
    until(|| {
        effect.render(Duration::ZERO, &mut canvas);
        canvas.get(0, 0) == Some(Rgb::new(255, 0, 0))
    });
    f.replace(b"invalid");
    until(|| {
        effect.render(Duration::ZERO, &mut canvas);
        f.manager.view().error.is_some()
    });
    assert_eq!(canvas.get(0, 0), Some(Rgb::new(255, 0, 0)));
    assert!(fault.lock().unwrap().is_none());
    f.replace(&module("local.get 0 f64.const 1 f64.gt if unreachable end i32.const 0 i32.const 128 i32.store8 i32.const 0"));
    until(|| {
        effect.render(Duration::ZERO, &mut canvas);
        canvas.get(0, 0) == Some(Rgb::new(128, 0, 0))
    });
    effect.render(Duration::from_secs(2), &mut canvas);
    assert_eq!(canvas.get(0, 0), Some(Rgb::new(255, 0, 0)));
    assert!(fault.lock().unwrap().is_none());
}
#[test]
fn disable_is_persistent_and_uninstall_faults_live_selection() {
    let _serial = PROCESS_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let f = Fixture::new();
    let (mut effect, fault) = f.effect();
    f.manager.set_enabled("aurora", false).unwrap();
    effect.render(Duration::ZERO, &mut Canvas::new(1, 1));
    assert!(fault.lock().unwrap().is_some());
    assert!(
        !Manager::development(f.root.join("store"), f.release.clone())
            .unwrap()
            .enabled("aurora")
    );
    assert!(f
        .manager
        .load("aurora", "1.0.0", &BTreeMap::new(), WasmEffect::fault())
        .is_err());
    f.manager.set_enabled("aurora", true).unwrap();
    let (mut effect, fault) = f.effect();
    f.manager.uninstall("aurora").unwrap();
    effect.render(Duration::ZERO, &mut Canvas::new(1, 1));
    assert!(fault.lock().unwrap().is_some());
}
#[test]
fn signatures_bind_exact_bytes_and_effect_domain() {
    use ed25519_dalek::{Signer, SigningKey};
    let key = SigningKey::from_bytes(&[7; 32]);
    let bytes = b"catalog";
    let mut message = freergb_effect_host::trust::CONTEXT.to_vec();
    message.extend_from_slice(bytes);
    let sig = hex::encode(key.sign(&message).to_bytes());
    let verify = |data: &[u8], signature: &[u8]| {
        freergb_effect_host::trust::verify_with_keys(data, signature, &[key.verifying_key()])
    };
    verify(bytes, sig.as_bytes()).unwrap();
    assert!(verify(b"edited", sig.as_bytes()).is_err());
    let module_sig = freergb_modules::trust::sign(&key, bytes);
    assert!(verify(bytes, module_sig.as_bytes()).is_err());
    assert!(verify(bytes, b"bad").is_err());
    assert!(freergb_effect_host::trust::verify(bytes, sig.as_bytes()).is_err());
}
#[test]
fn installed_metadata_cannot_replace_compiled_trust_with_a_new_hash() {
    let _serial = PROCESS_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let root = std::env::temp_dir().join(format!("freergb-effect-proof-{}", std::process::id()));
    let manager = Manager::new(root.clone()).unwrap();
    manager.install("aurora").unwrap();
    wait(&manager);
    assert!(manager.view().error.is_none(), "{:?}", manager.view().error);
    let path = root.join("packages/aurora/1.0.0.json");
    let mut entry: freergb_effect_host::marketplace::Entry =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let replacement = module("i32.const 0");
    entry.artifact.size = replacement.len() as u64;
    entry.artifact.sha256 = hex::encode(Sha256::digest(&replacement));
    std::fs::write(path, serde_json::to_vec(&entry).unwrap()).unwrap();
    std::fs::write(root.join("packages/aurora/1.0.0.wasm"), replacement).unwrap();
    assert!(manager
        .load("aurora", "1.0.0", &BTreeMap::new(), WasmEffect::fault())
        .is_err());
    drop(manager);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn worker_rejects_unknown_versions_fields_and_replayed_ids() {
    let _serial = PROCESS_TEST.lock().unwrap_or_else(|e| e.into_inner());
    use std::io::Write;
    use std::process::{Command, Stdio};
    let wasm = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        module("i32.const 0"),
    );
    let load =
        serde_json::json!({"api":1,"id":1,"command":{"method":"Load","wasm":wasm,"parameters":[]}})
            .to_string();
    for (input, expected_replies) in [
        ("{\"api\":999,\"id\":1,\"command\":{\"method\":\"Render\",\"seconds\":0,\"width\":1,\"height\":1}}\n".to_string(),0),
        ("{\"api\":1,\"id\":1,\"extra\":true,\"command\":{\"method\":\"Render\",\"seconds\":0,\"width\":1,\"height\":1}}\n".to_string(),0),
        (format!("{load}\n{load}\n"),1),
    ] {
        let mut child=Command::new(&host().executable).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let deadline=Instant::now()+Duration::from_secs(30);
        while child.try_wait().unwrap().is_none() {
            if Instant::now()>=deadline { let _=child.kill();let _=child.wait();panic!("worker did not reject invalid protocol"); }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output=child.wait_with_output().unwrap();
        assert_eq!(String::from_utf8(output.stdout).unwrap().lines().count(),expected_replies);
    }
}
