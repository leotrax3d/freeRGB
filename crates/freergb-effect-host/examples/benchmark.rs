//! First: cargo build --release -p freergb-effect-host --bin freergb-effect-worker
//! Then: cargo run --release -p freergb-effect-host --example benchmark
use freergb_core::Canvas;
use freergb_effect_host::{marketplace::Manager, runtime::WasmEffect};
use freergb_effects::Effect;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
fn main() {
    let root =
        std::env::temp_dir().join(format!("freergb-effect-benchmark-{}", std::process::id()));
    let manager = Manager::new(root.clone()).expect("store");
    for entry in manager.view().catalog {
        manager.install(&entry.id).expect("start installation");
        while manager.busy() {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(manager.view().error.is_none(), "{:?}", manager.view().error);
        let fault = WasmEffect::fault();
        let mut effect = manager
            .load(&entry.id, &entry.version, &BTreeMap::new(), fault.clone())
            .expect("load");
        let mut canvas = Canvas::new(320, 200);
        let start = Instant::now();
        for frame in 0..300 {
            effect.render(
                Duration::from_secs_f64(f64::from(frame) / 30.0),
                &mut canvas,
            );
        }
        assert!(fault.lock().unwrap().is_none());
        println!(
            "{}: {:.3} ms/frame (320x200 canvas, 128x64 guest, 300 frames)",
            entry.id,
            start.elapsed().as_secs_f64() * 1000.0 / 300.0
        );
    }
    drop(manager);
    let _ = std::fs::remove_dir_all(root);
}
