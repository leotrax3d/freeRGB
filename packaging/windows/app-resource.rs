fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/freergb.ico");
    println!("cargo:rerun-if-changed=../../packaging/windows/app-resource.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("../../assets/icons/freergb.ico")
            .set("ProductName", "freeRGB")
            .set("FileDescription", "freeRGB")
            .compile()
            .expect("compile freeRGB Windows icon and version resources");
    }
}
