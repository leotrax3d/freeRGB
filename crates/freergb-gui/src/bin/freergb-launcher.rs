//! Per-user installation entry point: starts the daemon if needed, then the GUI.
#![cfg_attr(windows, windows_subsystem = "windows")]
use std::process::Command;

fn installed_library(directory: &std::path::Path) -> Option<std::ffi::OsString> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(directory.join("mainboard-provider.json"))
        .ok()?
        .take(4097)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let state: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if state["version"] != 1
        || state["enabled"] != true
        || state["library"] != "providers/lhm-0.9.6"
    {
        return None;
    }
    let library = directory.join("providers").join("lhm-0.9.6");
    library
        .join("LibreHardwareMonitorLib.dll")
        .is_file()
        .then(|| library.into_os_string())
}

#[derive(Default)]
struct Options {
    demo: bool,
    daemon_only: bool,
    classic: bool,
    mainboard_library: Option<std::ffi::OsString>,
    asus_aura: bool,
    asus_argb_leds: Option<std::ffi::OsString>,
}

fn options(args: impl Iterator<Item = std::ffi::OsString>) -> Result<Options, &'static str> {
    let mut options = Options::default();
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--demo" {
            options.demo = true;
        }
        if arg == "--daemon-only" {
            options.daemon_only = true;
        }
        if arg == "--classic" {
            options.classic = true;
        }
        if arg == "--mainboard-library" {
            options.mainboard_library = Some(
                args.next()
                    .ok_or("--mainboard-library requires a directory")?,
            );
        }
        if arg == "--enable-asus-aura" {
            options.asus_aura = true;
        }
        if arg == "--asus-argb-leds" {
            options.asus_argb_leds = Some(
                args.next()
                    .ok_or("--asus-argb-leds requires four comma-separated counts")?,
            );
        }
    }
    if options.demo
        && (options.mainboard_library.is_some()
            || options.asus_aura
            || options.asus_argb_leds.is_some())
    {
        return Err("demo mode cannot enable motherboard access");
    }
    if options.asus_argb_leds.is_some() && !options.asus_aura {
        return Err("ARGB counts require --enable-asus-aura");
    }
    Ok(options)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut options = options(std::env::args_os().skip(1))?;
    let directory = std::env::current_exe()?
        .parent()
        .ok_or("missing executable directory")?
        .to_owned();
    if !options.demo && options.mainboard_library.is_none() {
        options.mainboard_library = installed_library(&directory);
    }
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
        if options.asus_aura {
            command.arg("--enable-asus-aura");
        }
        if let Some(counts) = options.asus_argb_leds {
            command.arg("--asus-argb-leds").arg(counts);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command.spawn()?;
    }
    if !options.daemon_only {
        Command::new(executable(if options.classic {
            "freergb-gui"
        } else {
            "freergb-desktop"
        }))
        .spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_modes_are_explicit() {
        let parse = |args: &[&str]| options(args.iter().map(std::ffi::OsString::from)).unwrap();
        assert!(!parse(&[]).classic);
        assert!(!parse(&[]).daemon_only);
        assert!(parse(&["--classic"]).classic);
        assert!(parse(&["--daemon-only"]).daemon_only);
    }
    #[test]
    fn installed_provider_requires_version_consent_fixed_path_and_runtime() {
        let directory =
            std::env::temp_dir().join(format!("freergb-launcher-provider-{}", std::process::id()));
        std::fs::create_dir_all(directory.join("providers/lhm-0.9.6")).unwrap();
        assert!(installed_library(&directory).is_none());
        std::fs::write(
            directory.join("providers/lhm-0.9.6/LibreHardwareMonitorLib.dll"),
            b"test fixture, never loaded",
        )
        .unwrap();
        for state in [
            r#"{"version":2,"enabled":true,"library":"providers/lhm-0.9.6"}"#,
            r#"{"version":1,"enabled":false,"library":"providers/lhm-0.9.6"}"#,
            r#"{"version":1,"enabled":true,"library":"../../elsewhere"}"#,
        ] {
            std::fs::write(directory.join("mainboard-provider.json"), state).unwrap();
            assert!(installed_library(&directory).is_none());
        }
        std::fs::write(
            directory.join("mainboard-provider.json"),
            r#"{"version":1,"enabled":true,"library":"providers/lhm-0.9.6"}"#,
        )
        .unwrap();
        assert!(installed_library(&directory).is_some());
        std::fs::remove_file(directory.join("mainboard-provider.json")).unwrap();
        std::fs::remove_file(directory.join("providers/lhm-0.9.6/LibreHardwareMonitorLib.dll"))
            .unwrap();
        std::fs::remove_dir(directory.join("providers/lhm-0.9.6")).unwrap();
        std::fs::remove_dir(directory.join("providers")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn motherboard_option_is_explicit_and_cannot_enable_hardware_in_demo() {
        let parse = |args: &[&str]| options(args.iter().map(std::ffi::OsString::from));
        assert!(parse(&[]).unwrap().mainboard_library.is_none());
        assert!(parse(&["--mainboard-library"]).is_err());
        assert!(parse(&["--demo", "--enable-asus-aura"]).is_err());
        assert!(parse(&["--asus-argb-leds", "1,2,3,4"]).is_err());
        assert!(
            parse(&["--enable-asus-aura", "--asus-argb-leds", "1,2,3,4"])
                .unwrap()
                .asus_aura
        );
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
