//! `freergbd` – freeRGB background service.
//!
//! Runs the render loop on a dedicated thread (HID I/O is blocking), serves
//! the local API, rescans for hot-plugged devices and hands all devices back
//! to hardware mode on exit (Ctrl+C or `daemon.shutdown`).

use anyhow::Result;
use clap::Parser;
use freergb_core::LedLayout;
use freergb_daemon::{engine::Engine, hardware, ipc, lock, state, SharedEngine};
use freergb_device_api::VirtualDevice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

fn parse_argb_counts(value: &str) -> Result<[u16; 4], String> {
    let counts = value
        .split(',')
        .map(|item| {
            item.parse::<u16>()
                .map_err(|_| "ARGB counts must be integers".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let counts: [u16; 4] = counts
        .try_into()
        .map_err(|_| "exactly four comma-separated ARGB counts are required".to_string())?;
    if counts.iter().any(|count| *count > 512) {
        return Err("ARGB counts must be 0..512".into());
    }
    Ok(counts)
}

#[derive(Parser)]
#[command(name = "freergbd", version, about = "freeRGB background service")]
struct Args {
    /// Explicit local effect release fixture for development; never configured by GUI.
    #[arg(long, hide = true)]
    effect_dev_directory: Option<std::path::PathBuf>,
    /// Isolated local endpoint suffix for integration tests, not normal clients.
    #[cfg(windows)]
    #[arg(long,hide=true,value_parser=ipc::validate_instance,requires_all=["no_hardware","no_state"])]
    ipc_instance: Option<String>,
    /// Allow user-triggered macros to send text/keys to the foreground app.
    #[arg(long)]
    enable_input_macros: bool,
    /// Test override: open ASUS Aura motherboard RGB even when vendor software
    /// runs or management is switched off, with the LED counts below instead of
    /// the saved device settings. Normally motherboard RGB is configured in the GUI.
    #[arg(long)]
    enable_asus_aura: bool,
    /// Override LED counts for four ARGB headers; zero disables that header.
    #[arg(long, default_value="0,0,0,0", value_parser=parse_argb_counts, requires="enable_asus_aura")]
    asus_argb_leds: [u16; 4],
    #[arg(
        long,
        value_name = "DIR",
        help = format!(
            "Opt in to read-only motherboard sensors using a trusted LHM {} directory. Requires separately installed/running PawnIO; never installs a driver.",
            freergb_core::lhm::VERSION
        )
    )]
    mainboard_library: Option<std::path::PathBuf>,
    /// Target frames per second.
    #[arg(long, default_value_t = 30)]
    fps: u32,
    /// Do not open real hardware (API + virtual device only).
    #[arg(long)]
    no_hardware: bool,
    /// Add a virtual 16-LED strip for testing.
    #[arg(long)]
    virtual_device: bool,
    /// Seconds between hot-plug rescans.
    #[arg(long, default_value_t = 5)]
    rescan_secs: u64,
    /// Neither restore nor save effect/brightness.
    #[arg(long)]
    no_state: bool,
    /// Do not read or control Linux hwmon motherboard fan headers.
    #[arg(long)]
    no_hwmon: bool,
    /// Also drive the Corsair iCUE LINK hub (fans/AIO lighting). Keeps the hub in
    /// software mode while running; an RPM watchdog returns it to hardware mode
    /// on any fan or pump slowdown.
    #[arg(long)]
    enable_link_hub: bool,
    /// Also load device descriptions (*.toml) from this directory. They are
    /// always treated as experimental. See docs/PLUGIN_SYSTEM.md.
    #[arg(long, value_name = "DIR")]
    user_devices: Option<std::path::PathBuf>,
    /// Explicitly trust and load a native process plugin manifest (repeatable).
    /// Hardware plugins exclude built-in hardware adapters; --no-hardware blocks them.
    #[arg(long = "plugin", value_name = "JSON")]
    plugins: Vec<std::path::PathBuf>,
    /// Disable downloadable hardware modules and device setup.
    #[arg(long)]
    no_modules: bool,
    /// Where the signed module catalog and packages come from:
    /// github:OWNER/REPO@TAG, an https:// base URL or a local directory.
    #[arg(long, env = "FREERGB_MODULE_SOURCE", default_value = freergb_modules::source::DEFAULT_SOURCE)]
    module_source: String,
    /// Additionally trust this development public key (hex) for the catalog.
    /// For local module development only; logged on every start.
    #[arg(long, value_name = "HEX")]
    module_dev_key: Option<String>,
}

fn data_dir() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let base =
        std::env::var_os("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("freeRGB"));
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })
        .map(|d| d.join("freergb"));
    base
}

fn module_manager(args: &Args) -> Result<Option<Arc<freergb_modules::setup::Manager>>> {
    use freergb_modules::{setup::Manager, source::Source, store::Store, trust::TrustedKeys};
    if args.no_modules {
        return Ok(None);
    }
    let (data, config) = match (args.no_state, data_dir(), state::default_path()) {
        (false, Some(data), Some(state)) => {
            (data.join("modules"), state.parent().unwrap().to_path_buf())
        }
        _ => {
            let temporary =
                std::env::temp_dir().join(format!("freergb-modules-{}", std::process::id()));
            (temporary.join("modules"), temporary)
        }
    };
    let mut keys = TrustedKeys::builtin();
    if let Some(hex) = &args.module_dev_key {
        tracing::warn!("trusting a development module signing key");
        keys = keys.with_development_key(freergb_modules::trust::parse_key(hex)?);
    }
    if keys.is_empty() {
        tracing::warn!(
            "no module signing key in this build: only already installed modules can run"
        );
    }
    let source = Source::parse(&args.module_source)?;
    tracing::info!(source = %source.describe(), store = %data.display(), "hardware modules");
    Ok(Some(Arc::new(Manager::new(
        Store::open(data)?,
        keys,
        source,
        config,
    ))))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();

    let mut engine = Engine::new(state::State::default().effect, args.fps);
    let effect_root = if args.no_state {
        std::env::temp_dir().join(format!("freergb-effects-{}", std::process::id()))
    } else {
        data_dir()
            .ok_or_else(|| anyhow::anyhow!("No user data directory for effects"))?
            .join("effect-marketplace")
    };
    let marketplace = match &args.effect_dev_directory {
        Some(directory) => {
            tracing::warn!(path = %directory.display(), "using local development effect release");
            freergb_effect_host::marketplace::Manager::development(effect_root, directory.clone())
        }
        None => freergb_effect_host::marketplace::Manager::new(effect_root),
    };
    engine.marketplace = match marketplace {
        Ok(manager) => Some(manager),
        Err(error) => {
            tracing::error!(%error, "marketplace unavailable; continuing with built-in effects");
            None
        }
    };
    engine.keyboard.capture_allowed = !args.no_hardware;
    if let Some(library) = &args.mainboard_library {
        let executable = std::env::current_exe()?;
        let helper = executable
            .parent()
            .ok_or_else(|| anyhow::anyhow!("daemon directory unavailable"))?
            .join("freergb-mainboard.exe");
        engine
            .mainboard
            .start(helper.clone(), library.clone(), !args.no_hardware)
            .map_err(anyhow::Error::msg)?;
        if !args.no_hardware && cfg!(windows) {
            engine.background = std::mem::take(&mut engine.background).enable(helper.clone());
            engine.cooling_sessions =
                freergb_daemon::cooling_sessions::CoolingSessions::enable(helper)
                    .map_err(anyhow::Error::msg)?;
        }
    }
    if !args.no_state {
        if let Some(path) = state::default_path() {
            tracing::info!(path = %path.display(), "state file");
            engine = engine.with_state_file(path);
        }
    }
    if !args.no_hardware {
        // GPU fans: discovery is read-only; a fan is written only after
        // cooling.gpu.set. Journaled fans from a crash return to driver auto.
        let state_dir = (!args.no_state)
            .then(state::default_path)
            .flatten()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        engine.gpu = freergb_daemon::gpu_cooling::GpuCooling::new(
            freergb_daemon::gpu_cooling::GpuCooling::platform_backends(),
            state_dir,
        );
        engine.gpu.start();
    }
    #[cfg(target_os = "linux")]
    if !args.no_hardware && !args.no_hwmon {
        // Kernel hwmon: reading is passive; headers are written only after an
        // explicit assignment (cooling.hwmon.set) or a saved one for this board.
        let root = std::env::var_os("FREERGB_HWMON_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "/sys/class/hwmon".into());
        let state_dir = (!args.no_state)
            .then(state::default_path)
            .flatten()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        engine.hwmon =
            freergb_daemon::hwmon::HwmonProvider::new(root, "/sys/class/dmi/id".into(), state_dir);
        engine
            .hwmon
            .set_unrestricted(engine.fan_support.config.unrestricted_enabled);
        if let Err(error) = engine.hwmon.start(true) {
            tracing::warn!(%error, "Linux hwmon fan provider unavailable");
        }
    }
    let manifests = args
        .plugins
        .iter()
        .map(|path| freergb_plugins::Manifest::load(path))
        .collect::<std::io::Result<Vec<_>>>()?;
    engine.controls.input_enabled = args.enable_input_macros;
    engine
        .configure_plugins(manifests, !args.no_hardware)
        .map_err(anyhow::Error::msg)?;
    if engine.has_hardware_plugins() {
        tracing::warn!("hardware process plugins configured: built-in hardware adapters disabled");
    }
    engine.modules = module_manager(&args)?;
    if let Some(manager) = engine.modules.clone() {
        // One catalog file, no device data is sent. Failure keeps the cache.
        std::thread::Builder::new()
            .name("module-catalog".into())
            .spawn(move || manager.refresh_catalog())?;
    }
    let engine: SharedEngine = Arc::new(Mutex::new(engine));
    if args.virtual_device {
        lock(&engine).add_device(Box::new(VirtualDevice::new(
            "Virtual Strip",
            LedLayout::strip(16),
        )));
    }
    let user_devices = match &args.user_devices {
        Some(dir) => {
            let (ok, errors) = freergb_protocols::descriptions::load_user_dir(dir);
            for e in &errors {
                tracing::warn!(error = %e, "skipping device description");
            }
            for d in &ok {
                tracing::warn!(name = %d.name, source = %d.source, "using user device description (experimental)");
            }
            ok
        }
        None => Vec::new(),
    };
    let scan_opts = hardware::ScanOptions {
        enable_asus_aura: args.enable_asus_aura,
        asus_argb_leds: args.asus_argb_leds,
        enable_link_hub: args.enable_link_hub,
        user_devices: Arc::new(user_devices),
    };
    if !args.no_hardware {
        let n = hardware::scan(&engine, &scan_opts);
        tracing::info!(added = n, "initial hardware scan");
    }

    let stop = Arc::new(AtomicBool::new(false));
    let render = {
        let engine = engine.clone();
        let stop = stop.clone();
        let rescan = (!args.no_hardware).then(|| Duration::from_secs(args.rescan_secs.max(1)));
        std::thread::Builder::new()
            .name("render".into())
            .spawn(move || render_loop(engine, stop, rescan, scan_opts))?
    };

    let rt = tokio::runtime::Runtime::new()?;
    let shutdown = Arc::new(Notify::new());
    let api_result = rt.block_on(async {
        tokio::select! {
            r = async {
                #[cfg(windows)]
                if let Some(instance) = &args.ipc_instance { return ipc::serve_instance(engine.clone(),shutdown.clone(),instance).await; }
                ipc::serve(engine.clone(),shutdown.clone()).await
            } => {
                r
            }
            _ = shutdown.notified() => { tracing::info!("shutdown requested via API"); Ok(()) },
            _ = tokio::signal::ctrl_c() => { tracing::info!("Ctrl+C"); Ok(()) },
        }
    });

    stop.store(true, Ordering::SeqCst);
    let _ = render.join();
    lock(&engine).shutdown();
    tracing::info!("devices released, bye");
    api_result?;
    Ok(())
}

fn render_loop(
    engine: SharedEngine,
    stop: Arc<AtomicBool>,
    rescan: Option<Duration>,
    scan_opts: hardware::ScanOptions,
) {
    let period = Duration::from_secs_f64(1.0 / f64::from(lock(&engine).target_fps()));
    let mut last_scan = Instant::now();
    let mut last_resume = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        let t0 = Instant::now();
        lock(&engine).tick(t0);
        if last_resume.elapsed() >= Duration::from_secs(1) {
            last_resume = Instant::now();
            lock(&engine).resume_cooling();
        }
        if let Some(every) = rescan {
            let requested = lock(&engine)
                .modules
                .as_ref()
                .is_some_and(|m| m.rescan.swap(false, Ordering::SeqCst));
            if requested || last_scan.elapsed() >= every {
                last_scan = Instant::now();
                let n = hardware::scan(&engine, &scan_opts);
                if n > 0 {
                    tracing::info!(added = n, "hot-plug scan");
                }
            }
        }
        // A guest may use the whole frame interval, especially in debug builds.
        // Always release CPU time between frames so pending API workers can
        // acquire the engine mutex even when rendering misses its target FPS.
        std::thread::sleep(
            period
                .saturating_sub(t0.elapsed())
                .max(Duration::from_millis(1)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aura_header_counts_are_explicit_and_bounded_without_opening_hardware() {
        let defaults = Args::try_parse_from(["freergbd", "--no-hardware"]).unwrap();
        assert!(!defaults.enable_asus_aura);
        assert_eq!(defaults.asus_argb_leds, [0; 4]);
        assert!(Args::try_parse_from(["freergbd", "--asus-argb-leds", "1,2,3,4"]).is_err());
        for counts in ["513,0,0,0", "-1,0,0,0", "1,2,3", "1,2,3,4,5"] {
            assert!(
                Args::try_parse_from([
                    "freergbd",
                    "--enable-asus-aura",
                    "--asus-argb-leds",
                    counts
                ])
                .is_err(),
                "{counts}"
            );
        }
        assert_eq!(
            Args::try_parse_from([
                "freergbd",
                "--enable-asus-aura",
                "--asus-argb-leds",
                "512,12,0,0"
            ])
            .unwrap()
            .asus_argb_leds,
            [512, 12, 0, 0]
        );
    }
    #[cfg(windows)]
    #[test]
    fn isolated_endpoint_requires_no_hardware_and_no_user_state() {
        assert!(Args::try_parse_from(["freergbd", "--ipc-instance", "test"]).is_err());
        assert!(
            Args::try_parse_from(["freergbd", "--no-hardware", "--ipc-instance", "test"]).is_err()
        );
        assert!(Args::try_parse_from([
            "freergbd",
            "--no-hardware",
            "--no-state",
            "--ipc-instance",
            "test-1"
        ])
        .is_ok());
        for instance in ["", r"..\elsewhere", r"\\remote\pipe", "a b"] {
            assert!(Args::try_parse_from([
                "freergbd",
                "--no-hardware",
                "--no-state",
                "--ipc-instance",
                instance
            ])
            .is_err());
        }
    }
}
