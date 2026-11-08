pub fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/freergb.ico");
    println!("cargo:rerun-if-changed=../../packaging/windows/app-resource.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let compiled = winresource::WindowsResource::new()
            .set_icon("../../assets/icons/freergb.ico")
            .set("ProductName", "freeRGB")
            .set("FileDescription", "freeRGB")
            .compile();
        // rc.exe (MSVC) or windres (GNU) may be absent when only checking
        // code without linking; the icon is cosmetic, so stay buildable.
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            compiled.expect("compile freeRGB Windows icon and version resources");
        } else if let Err(error) = compiled {
            println!("cargo:warning=skipping freeRGB icon resources: {error}");
        }
    }
}
