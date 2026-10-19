//! Per-user installation entry point: starts the daemon if needed, then the GUI.
#![cfg_attr(windows, windows_subsystem = "windows")]
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::current_exe()?
        .parent()
        .ok_or("missing executable directory")?
        .to_owned();
    let executable = |name: &str| {
        directory.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.into()
        })
    };
    if freergb_client::call("daemon.version", serde_json::Value::Null).is_err() {
        let mut command = Command::new(executable("freergbd"));
        if std::env::args().any(|a| a == "--demo") {
            command.args(["--no-hardware", "--virtual-device"]);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command.spawn()?;
    }
    Command::new(executable("freergb-gui")).spawn()?;
    Ok(())
}
