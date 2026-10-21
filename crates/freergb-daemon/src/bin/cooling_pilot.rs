//! Developer commissioning client, not a public live-control API. The protected
//! service independently restricts authorization to a selected fixed100 channel.
use freergb_control::FanCurve;
use freergb_daemon::{cooling_host::CoolingHost, cooling_runtime::ControlMode, native_bridge};
use std::time::{Duration, Instant};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let helper = args
        .next()
        .ok_or("Usage: freergb-cooling-pilot <bundled-mainboard-helper.exe>")?;
    if args.next().is_some() {
        return Err("Unexpected pilot arguments".into());
    }
    let epoch = Instant::now();
    // The native journal still independently refuses unresolved acquisition.
    let mut host = CoolingHost::new(true, epoch);
    let mut curve = FanCurve {
        minimum_percent: 100.0,
        ..FanCurve::default()
    };
    for point in &mut curve.points {
        point.duty_percent = 100.0;
    }
    host.start_native(
        native_bridge::connect(helper.into()),
        ControlMode::Curve(curve),
    )?;
    // Startup may take up to 12s; the owned task preserves admission throughout.
    for _ in 0..140 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if host.collect_finished().await?.is_some() {
            return Err("Native pilot ended before requested return".into());
        }
    }
    host.prepare_maintenance(20_000).await?;
    println!("PASS: Rust curve host, checked native temperature/RPM, protected session and verified firmware/journal return (100 percent only).");
    Ok(())
}
