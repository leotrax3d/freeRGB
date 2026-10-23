//! Real guests and sandbox, no live services.
use freergb_core::Canvas;
use freergb_effect_host::{
    marketplace::{Catalog, Manager},
    runtime::WasmEffect,
};
use freergb_effects::Effect;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};
fn catalog() -> Catalog {
    Catalog::parse(include_bytes!("../../../effects/development/catalog.json")).unwrap()
}
fn bytes(id: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../effects/development")
            .join(format!("{id}-1.0.0.wasm")),
    )
    .unwrap()
}
fn frame(bytes: &[u8], params: &[f64], time: f64, w: usize, h: usize) -> Canvas {
    let fault = WasmEffect::fault();
    let mut e = WasmEffect::load(bytes, params, fault.clone()).unwrap();
    let mut c = Canvas::new(w, h);
    e.render(Duration::from_secs_f64(time), &mut c);
    assert!(
        fault.lock().unwrap().is_none(),
        "{w}x{h} at {time}: {:?}",
        fault.lock().unwrap()
    );
    c
}
#[test]
fn development_install_is_offline_and_has_no_community_client() {
    let root = std::env::temp_dir().join(format!("freergb-collection-{}", std::process::id()));
    let m = Manager::development(
        root.clone(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../effects/development"),
    )
    .unwrap();
    assert!(!m.view().ratings.configured);
    m.refresh().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while m.busy() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(m.view().error.is_none(), "{:?}", m.view().error);
    assert_eq!(m.view().catalog.len(), 28);
    for e in m.view().catalog {
        m.install(&e.id).unwrap();
        while m.busy() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(m.view().error.is_none(), "{}: {:?}", e.id, m.view().error);
        m.load(&e.id, &e.version, &BTreeMap::new(), WasmEffect::fault())
            .unwrap();
    }
    assert_eq!(m.view().installed.len(), 28);
    assert!(!m.view().ratings.configured);
    assert!(m.view().ratings.error.is_none());
    drop(m);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn collection_frames_layouts_limits_and_parameter_endpoints() {
    for e in catalog().effects.iter().skip(3) {
        eprintln!("Checking {}", e.id);
        let b = bytes(&e.id);
        let params: Vec<_> = e.parameters.iter().map(|p| p.default).collect();
        for (w, h) in [
            (1, 1),
            (2, 1),
            (8, 1),
            (1, 8),
            (7, 5),
            (128, 64),
            (320, 200),
        ] {
            for t in [0., 0.125, 0.7, 1.9, 3.875, 4.0, 12345.678] {
                frame(&b, &params, t, w, h);
            }
        }
        let times = [0., 0.7, 1.9, 3.875, 5.3];
        let frames: Vec<_> = times.map(|t| frame(&b, &params, t, 64, 32)).into();
        assert!(
            frames.windows(2).any(|f| f[0] != f[1]),
            "{} does not animate",
            e.id
        );
        for (i, p) in e.parameters.iter().enumerate() {
            let mut changed = false;
            for v in [p.min, p.max, (p.min + p.max) * 0.5] {
                let mut ps = params.clone();
                ps[i] = v;
                for (j, t) in times.into_iter().enumerate() {
                    changed |= frame(&b, &ps, t, 64, 32) != frames[j];
                    frame(&b, &ps, t, 128, 64);
                }
            }
            if !changed {
                // Rare lightning/glitch events require a longer observation window.
                for tick in 0..80 {
                    let t = tick as f64 * 0.123;
                    let baseline = frame(&b, &params, t, 16, 8);
                    let mut ps = params.clone();
                    ps[i] = p.max;
                    if frame(&b, &ps, t, 16, 8) != baseline {
                        changed = true;
                        break;
                    }
                    ps[i] = p.min;
                    if frame(&b, &ps, t, 16, 8) != baseline {
                        changed = true;
                        break;
                    }
                }
            }
            assert!(
                changed,
                "{} parameter {} has no visible effect",
                e.id, p.key
            );
        }
    }
}
#[test]
fn output_is_seeded_and_independent_of_frame_schedule() {
    for e in catalog().effects.iter().skip(3) {
        let b = bytes(&e.id);
        let params: Vec<_> = e.parameters.iter().map(|p| p.default).collect();
        let expected = frame(&b, &params, 3.875, 16, 8);
        for fps in [8, 30, 60] {
            let fault = WasmEffect::fault();
            let mut effect = WasmEffect::load(&b, &params, fault.clone()).unwrap();
            let mut canvas = Canvas::new(16, 8);
            for i in 0..fps * 4 {
                effect.render(Duration::from_secs_f64(i as f64 / fps as f64), &mut canvas);
            }
            effect.render(Duration::from_secs_f64(3.875), &mut canvas);
            assert!(
                fault.lock().unwrap().is_none(),
                "{}: {:?}",
                e.id,
                fault.lock().unwrap()
            );
            assert_eq!(canvas, expected, "{} at {fps} fps", e.id);
        }
    }
}

#[test]
fn single_led_and_short_strips_animate_and_brightness_zero_is_black() {
    for e in catalog().effects.iter().skip(3) {
        let b = bytes(&e.id);
        let ps: Vec<_> = e.parameters.iter().map(|p| p.default).collect();
        for (w, h) in [(1, 1), (2, 1), (1, 2)] {
            let first = frame(&b, &ps, 0., w, h);
            assert!(
                (1..60).any(|i| frame(&b, &ps, i as f64 * 0.137, w, h) != first),
                "{} is static on {w}x{h}",
                e.id
            );
            let mut dark = ps.clone();
            dark[1] = 0.;
            assert_eq!(frame(&b, &dark, 1.9, w, h), Canvas::new(w, h));
        }
        for boundary in [false, true] {
            let ps: Vec<_> = e
                .parameters
                .iter()
                .map(|p| if boundary { p.max } else { p.min })
                .collect();
            frame(&b, &ps, 3.875, 128, 64);
        }
    }
}

#[test]
fn guest_abi_rejects_invalid_dimensions_and_time_without_imports_or_memory_growth() {
    use wasmi::{Config, Engine, Linker, Module, Store};
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    for e in catalog().effects.iter().skip(3) {
        let module = Module::new(&engine, bytes(&e.id).as_slice()).unwrap();
        assert!(module.imports().next().is_none());
        let mut store = Store::new(&engine, ());
        store.set_fuel(2_000_000).unwrap();
        let instance = Linker::new(&engine)
            .instantiate(&mut store, &module)
            .unwrap()
            .ensure_no_start(&mut store)
            .unwrap();
        let memory = instance.get_memory(&store, "memory").unwrap();
        assert_eq!(memory.data(&store).len(), 256 * 1024);
        assert!(memory.grow(&mut store, 1).is_err());
        let set = instance
            .get_typed_func::<(i32, f64), ()>(&store, "set_param")
            .unwrap();
        for (i, p) in e.parameters.iter().enumerate() {
            set.call(&mut store, (i as i32, p.default)).unwrap();
        }
        for i in [-1, 16, i32::MAX] {
            set.call(&mut store, (i, f64::NAN)).unwrap();
        }
        let render = instance
            .get_typed_func::<(f64, i32, i32), i32>(&store, "render")
            .unwrap();
        for args in [
            (0., 0, 1),
            (0., 1, 0),
            (0., 129, 1),
            (0., 1, 65),
            (0., -1, 1),
            (f64::NAN, 1, 1),
            (f64::INFINITY, 1, 1),
            (-1., 1, 1),
        ] {
            store.set_fuel(2_000_000).unwrap();
            assert_eq!(
                render.call(&mut store, args).unwrap(),
                -1,
                "{}: {args:?}",
                e.id
            );
        }
    }
}
