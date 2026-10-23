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
    let mode = args.next().unwrap_or_else(|| "100".into());
    let mode = mode.to_str().ok_or("Invalid pilot mode")?;
    if !["100", "80", "40", "calibrate"].contains(&mode) || args.next().is_some() {
        return Err("Unexpected pilot arguments".into());
    }
    let epoch = Instant::now();
    // The native journal still independently refuses unresolved acquisition.
    let mut host = CoolingHost::new(true, epoch);
    let mut curve = FanCurve {
        minimum_percent: if mode == "calibrate" {
            40.0
        } else {
            mode.parse()?
        },
        ..FanCurve::default()
    };
    for point in &mut curve.points {
        point.duty_percent = point.duty_percent.max(curve.minimum_percent);
    }
    host.start_native(
        native_bridge::connect(helper.into()),
        if mode == "calibrate" {
            ControlMode::Calibration { critical_c: 80.0 }
        } else {
            ControlMode::Curve(curve)
        },
        freergb_daemon::cooling_runtime::RuntimeLimits::default(),
    )?;
    // Startup may take up to 12s; the owned task preserves admission throughout.
    for _ in 0..if mode == "calibrate" { 450 } else { 140 } {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(report) = host.collect_finished().await? {
            println!("{}", serde_json::to_string(&report)?);
            if mode != "calibrate" {
                return Err("Native pilot ended before requested return".into());
            }
            break;
        }
    }
    host.prepare_maintenance(20_000).await?;
    if let Some(report) = host.last_report() {
        println!("{}", serde_json::to_string(report)?);
    }
    println!("PASS: Rust host, checked native telemetry and verified firmware/journal return (commissioning mode {mode}).");
    Ok(())
}
