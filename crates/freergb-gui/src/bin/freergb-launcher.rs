//! Per-user installation entry point: starts the daemon if needed, then the GUI.
#![cfg_attr(windows, windows_subsystem = "windows")]
use std::process::Command;

#[derive(Default)]
struct Options {
    demo: bool,
    mainboard_library: Option<std::ffi::OsString>,
}

fn options(args: impl Iterator<Item = std::ffi::OsString>) -> Result<Options, &'static str> {
    let mut options = Options::default();
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--demo" {
            options.demo = true;
        }
        if arg == "--mainboard-library" {
            options.mainboard_library = Some(
                args.next()
                    .ok_or("--mainboard-library requires a directory")?,
            );
        }
    }
    if options.demo && options.mainboard_library.is_some() {
        return Err("demo mode cannot enable motherboard access");
    }
    Ok(options)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = options(std::env::args_os().skip(1))?;
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
        if options.demo {
            command.args(["--no-hardware", "--virtual-device"]);
        }
        if let Some(library) = options.mainboard_library {
            command.arg("--mainboard-library").arg(library);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motherboard_option_is_explicit_and_cannot_enable_hardware_in_demo() {
        let parse = |args: &[&str]| options(args.iter().map(std::ffi::OsString::from));
        assert!(parse(&[]).unwrap().mainboard_library.is_none());
        assert!(parse(&["--mainboard-library"]).is_err());
        assert!(parse(&["--demo", "--mainboard-library", "C:\\LHM"]).is_err());
        assert_eq!(
            parse(&["--mainboard-library", "C:\\LHM files"])
                .unwrap()
                .mainboard_library
                .unwrap(),
            "C:\\LHM files"
        );
    }
}
