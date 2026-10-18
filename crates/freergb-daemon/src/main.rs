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

#[derive(Parser)]
#[command(name = "freergbd", version, about = "freeRGB background service")]
struct Args {
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
    /// Also drive the Corsair iCUE LINK hub (fans/AIO lighting). Keeps the hub in
    /// software mode while running; an RPM watchdog returns it to hardware mode
    /// on any fan or pump slowdown.
    #[arg(long)]
    enable_link_hub: bool,
    /// EXPERIMENTAL: pipelined stream packets for Bragi keyboards (K70 CORE).
    #[arg(long)]
    experimental_pipelining: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();

    let mut engine = Engine::new(state::State::default().effect, args.fps);
    if !args.no_state {
        if let Some(path) = state::default_path() {
            tracing::info!(path = %path.display(), "state file");
            engine = engine.with_state_file(path);
        }
    }
    let engine: SharedEngine = Arc::new(Mutex::new(engine));
    if args.virtual_device {
        lock(&engine).add_device(Box::new(VirtualDevice::new(
            "Virtual Strip",
            LedLayout::strip(16),
        )));
    }
    let scan_opts = hardware::ScanOptions {
        enable_link_hub: args.enable_link_hub,
        bragi_pipelined: args.experimental_pipelining,
    };
    if !args.no_hardware {
        let n = hardware::scan(&engine, scan_opts);
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
    rt.block_on(async {
        tokio::select! {
            r = ipc::serve(engine.clone(), shutdown.clone()) => {
                if let Err(e) = r { tracing::error!(error = %e, "API server stopped"); }
            }
            _ = shutdown.notified() => tracing::info!("shutdown requested via API"),
            _ = tokio::signal::ctrl_c() => tracing::info!("Ctrl+C"),
        }
    });

    stop.store(true, Ordering::SeqCst);
    let _ = render.join();
    lock(&engine).shutdown();
    tracing::info!("devices released, bye");
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
    while !stop.load(Ordering::SeqCst) {
        let t0 = Instant::now();
        lock(&engine).tick(t0);
        if let Some(every) = rescan {
            if last_scan.elapsed() >= every {
                last_scan = Instant::now();
                let n = hardware::scan(&engine, scan_opts);
                if n > 0 {
                    tracing::info!(added = n, "hot-plug scan");
                }
            }
        }
        if let Some(rest) = period.checked_sub(t0.elapsed()) {
            std::thread::sleep(rest);
        }
    }
}
