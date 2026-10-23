//! Render the development collection to portable pixmaps for offline visual QA.
use freergb_core::Canvas;
use freergb_effect_host::{marketplace::Catalog, runtime::WasmEffect};
use freergb_effects::Effect;
use std::{io::Write, path::PathBuf, time::Duration};
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../effects/development");
    let catalog = Catalog::parse(&std::fs::read(root.join("catalog.json")).unwrap()).unwrap();
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/effect-previews"));
    std::fs::create_dir_all(&out).unwrap();
    for entry in catalog.effects.iter().skip(3) {
        let bytes = std::fs::read(root.join(&entry.artifact.url)).unwrap();
        let params: Vec<_> = entry.parameters.iter().map(|p| p.default).collect();
        let fault = WasmEffect::fault();
        let mut effect = WasmEffect::load(&bytes, &params, fault.clone()).unwrap();
        for (i, t) in [0.3, 1.1, 2.3, 3.7].iter().enumerate() {
            let mut canvas = Canvas::new(128, 64);
            effect.render(Duration::from_secs_f64(*t), &mut canvas);
            assert!(
                fault.lock().unwrap().is_none(),
                "{}: {:?}",
                entry.id,
                fault.lock().unwrap()
            );
            let mut f = std::fs::File::create(out.join(format!("{}-{i}.ppm", entry.id))).unwrap();
            write!(f, "P6\n128 64\n255\n").unwrap();
            for y in 0..64 {
                for x in 0..128 {
                    let c = canvas.get(x, y).unwrap();
                    f.write_all(&[c.r, c.g, c.b]).unwrap();
                }
            }
        }
    }
    println!("Previews: {}", out.display());
}
