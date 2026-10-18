//! `freergbd` – freeRGB background service.
//!
//! Runs the render loop on a dedicated thread (HID I/O is blocking), serves
//! the local API, rescans for hot-plugged devices and hands all devices back
//! to hardware mode on exit (Ctrl+C or `daemon.shutdown`).

use anyhow::Result;
use clap::Parser;
use freergb_core::LedLayout;
use freergb_daemon::{engine::Engine, hardware, ipc, lock, SharedEngine};
use freergb_device_api::VirtualDevice;
use freergb_effects::EffectSpec;
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
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();

    let engine: SharedEngine = Arc::new(Mutex::new(Engine::new(
        EffectSpec::RainbowWave {
            speed: 0.2,
            scale: 1.0,
        },
        args.fps,
    )));
    if args.virtual_device {
        lock(&engine).add_device(Box::new(VirtualDevice::new(
            "Virtual Strip",
            LedLayout::strip(16),
        )));
    }
    if !args.no_hardware {
        let n = hardware::scan(&engine);
        tracing::info!(added = n, "initial hardware scan");
    }

    let stop = Arc::new(AtomicBool::new(false));
    let render = {
        let engine = engine.clone();
        let stop = stop.clone();
        let rescan = (!args.no_hardware).then(|| Duration::from_secs(args.rescan_secs.max(1)));
        std::thread::Builder::new()
            .name("render".into())
            .spawn(move || render_loop(engine, stop, rescan))?
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

fn render_loop(engine: SharedEngine, stop: Arc<AtomicBool>, rescan: Option<Duration>) {
    let period = Duration::from_secs_f64(1.0 / f64::from(lock(&engine).target_fps()));
    let mut last_scan = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        let t0 = Instant::now();
        lock(&engine).tick(t0);
        if let Some(every) = rescan {
            if last_scan.elapsed() >= every {
                last_scan = Instant::now();
                let n = hardware::scan(&engine);
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
