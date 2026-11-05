#[cfg(target_os = "linux")]
fn main() {
    let fps = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(15);
    if let Err(error) = freergb_screen::run_linux_helper(fps) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
