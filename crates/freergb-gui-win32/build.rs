mod resource {
    include!("../../packaging/windows/app-resource.rs");
}

/// Embed every locale JSON present in the desktop app's locale folder, so
/// languages arriving from other branches are picked up without edits here.
fn embed_locales() {
    let dir = std::path::Path::new("../../apps/desktop/src/lib/i18n/locales");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let p = e.path();
                    let stem = p.file_stem()?.to_str()?.to_owned();
                    (p.extension()? == "json").then_some((stem, p))
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort();
    let mut out = String::from("pub const LOCALES: &[(&str, &str)] = &[\n");
    for (code, path) in &entries {
        let abs = std::fs::canonicalize(path).expect("locale path");
        let abs = abs.to_string_lossy().replace('\\', "/");
        let abs = abs.strip_prefix("//?/").unwrap_or(&abs);
        out.push_str(&format!("    ({code:?}, include_str!({abs:?})),\n"));
    }
    out.push_str("];\n");
    let dest = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("locales.rs");
    std::fs::write(dest, out).expect("write locales.rs");
}

fn main() {
    embed_locales();
    resource::main();
}
