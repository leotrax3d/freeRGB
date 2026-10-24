//! freeRGB tray icon: a small process started at sign-in next to the service.
//! It only talks to the daemon's local RPC API (same rights as the GUI); fan
//! changes that need administrator consent still go through UAC in the daemon.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(any(windows, test))]
mod state;

#[cfg(windows)]
mod tray;

#[cfg(windows)]
fn main() {
    tray::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("freergb-tray is Windows-only");
}
