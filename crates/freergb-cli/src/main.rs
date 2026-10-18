//! `freergb` command line tool.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use freergb_core::{Canvas, Region, Rgb};
use freergb_device_api::{RgbDevice, VirtualDevice};
use freergb_effects::EffectSpec;
use freergb_hid::HidapiTransport;
use freergb_protocols::corsair_bragi::{BragiDevice, BragiModel};
use freergb_protocols::registry;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(
    name = "freergb",
    version,
    about = "freeRGB RGB control: discovery and diagnostics"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List HID interfaces and matching adapters (read-only).
    Discover {
        #[arg(long)]
        json: bool,
        /// Show all interfaces, not only RGB vendors / matched ones.
        #[arg(long)]
        all: bool,
    },
    /// Render an effect into a virtual device and print frame checksums.
    DemoVirtual {
        #[arg(long, value_enum, default_value_t = EffectArg::Rainbow)]
        effect: EffectArg,
        #[arg(long, default_value_t = 5)]
        frames: u32,
    },
    /// Corsair Bragi devices (K70 CORE).
    Bragi {
        #[command(subcommand)]
        cmd: BragiCmd,
        /// USB product id (hex).
        #[arg(long, default_value = "1bfd", global = true)]
        pid: String,
    },
    /// Call a method on the running freergbd, e.g. `ctl color.set '{"color":"#ff0000"}'`.
    Ctl {
        method: String,
        /// JSON params object.
        params: Option<String>,
    },
}

#[derive(Subcommand)]
enum BragiCmd {
    /// Read-only identification: ping, VID/PID, firmware, mode, layout.
    Probe,
    /// Show a solid colour, then restore hardware mode.
    Color {
        color: String,
        #[arg(long, default_value_t = 3.0)]
        seconds: f32,
    },
    /// Run an effect for a while, then restore hardware mode.
    Effect {
        #[arg(value_enum)]
        effect: EffectArg,
        #[arg(long, default_value_t = 10.0)]
        seconds: f32,
        #[arg(long, default_value_t = 30)]
        fps: u32,
    },
    /// Light keys one after another (per-key mapping check).
    KeyWalk {
        #[arg(long, default_value_t = 150)]
        ms_per_key: u64,
    },
    /// Light named keys, e.g. `Esc=#ffffff "Num 5=#ff0000"`; others stay dark.
    Keys {
        assignments: Vec<String>,
        #[arg(long, default_value_t = 15.0)]
        seconds: f32,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum EffectArg {
    Rainbow,
    Breathing,
    Red,
}

impl EffectArg {
    fn spec(self) -> EffectSpec {
        match self {
            EffectArg::Rainbow => EffectSpec::RainbowWave {
                speed: 0.3,
                scale: 1.0,
            },
            EffectArg::Breathing => EffectSpec::Breathing {
                color: Rgb::new(0, 155, 222),
                period_ms: 3000,
            },
            EffectArg::Red => EffectSpec::Solid { color: Rgb::RED },
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    match Cli::parse().cmd {
        Cmd::Discover { json, all } => discover(json, all),
        Cmd::DemoVirtual { effect, frames } => demo_virtual(effect, frames),
        Cmd::Bragi { cmd, pid } => {
            let pid =
                u16::from_str_radix(pid.trim_start_matches("0x"), 16).context("invalid --pid")?;
            bragi(cmd, pid)
        }
        Cmd::Ctl { method, params } => ctl(&method, params.as_deref()),
    }
}

/// Sends one JSON-RPC request to freergbd over the local IPC endpoint.
fn ctl(method: &str, params: Option<&str>) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};

    let params: serde_json::Value = match params {
        Some(p) => serde_json::from_str(p).context("params must be JSON")?,
        None => serde_json::Value::Null,
    };
    let req = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});

    #[cfg(windows)]
    let stream = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(r"\\.\pipe\freergb")
        .context("cannot reach freergbd (is it running?)")?;
    #[cfg(unix)]
    let stream = {
        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        std::os::unix::net::UnixStream::connect(dir.join("freergb.sock"))
            .context("cannot reach freergbd (is it running?)")?
    };

    let mut writer = stream.try_clone()?;
    writeln!(writer, "{req}")?;
    writer.flush()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    let resp: serde_json::Value = serde_json::from_str(&line)?;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    if resp.get("error").is_some() {
        bail!("daemon returned an error");
    }
    Ok(())
}

fn discover(json: bool, all: bool) -> Result<()> {
    let list = freergb_hid::enumerate()?;
    let rows: Vec<_> = list
        .iter()
        .filter(|i| all || registry::vendor_name(i.vid).is_some())
        .map(|i| {
            let m = registry::match_interface(i.vid, i.pid, i.usage_page, i.usage);
            (i, m)
        })
        .collect();

    if json {
        let v: Vec<_> = rows
            .iter()
            .map(|(i, m)| {
                serde_json::json!({
                    "interface": i,
                    "adapter": m.as_ref().map(|m| m.adapter),
                    "support": m.as_ref().map(|m| format!("{:?}", m.level)),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }

    println!(
        "{:<9} {:>3} {:<11} {:<36} {:<22} SUPPORT",
        "VID:PID", "IF", "UP:U", "PRODUCT", "ADAPTER"
    );
    for (i, m) in rows {
        println!(
            "{:04x}:{:04x} {:>3} {:04x}:{:04x}   {:<36} {:<22} {}",
            i.vid,
            i.pid,
            i.interface,
            i.usage_page,
            i.usage,
            i.product
                .as_deref()
                .unwrap_or("?")
                .chars()
                .take(36)
                .collect::<String>(),
            m.as_ref().map_or("-", |m| m.adapter),
            m.map_or("unknown (not written to)".into(), |m| format!(
                "{:?}",
                m.level
            )),
        );
    }
    Ok(())
}

fn demo_virtual(effect: EffectArg, frames: u32) -> Result<()> {
    let layout = freergb_protocols::corsair_bragi::k70_core::layout();
    let mut dev = VirtualDevice::new("Virtual K70", layout.clone());
    let log = dev.frame_log();
    dev.connect()?;
    let mut fx = effect.spec().build();
    let mut canvas = Canvas::new(
        usize::from(layout.width) * 4,
        usize::from(layout.height) * 4,
    );
    for n in 0..frames {
        let t = Duration::from_millis(u64::from(n) * 33);
        fx.render(t, &mut canvas);
        let frame = canvas.sample_layout(&layout, Region::FULL);
        dev.push_frame(&frame)?;
        println!(
            "frame {n:>3} t={:>5}ms leds={} first={} checksum={:016x}",
            t.as_millis(),
            frame.len(),
            frame.colors()[0],
            frame.checksum()
        );
    }
    dev.disconnect(Some(Rgb::BLACK))?;
    println!("virtual device received {} frames", log.total());
    Ok(())
}

fn open_bragi(pid: u16) -> Result<BragiDevice<HidapiTransport>> {
    let model =
        BragiModel::for_pid(pid).with_context(|| format!("no Bragi model for PID {pid:04x}"))?;
    let info = freergb_hid::find(0x1B1C, pid, 0xFF42, 0x0001)?;
    let transport = HidapiTransport::open(&info)?;
    let sizes = transport.report_sizes();
    eprintln!(
        "opened {} (in {} / out {} bytes, report ids: {})",
        info.product.as_deref().unwrap_or("?"),
        sizes.input,
        sizes.output,
        sizes.uses_report_ids
    );
    if sizes.output < 16 {
        bail!("implausible output report size {}", sizes.output);
    }
    Ok(BragiDevice::new(transport, model, info.serial))
}

/// Runs `body` with a connected device and always tries to restore hardware
/// mode afterwards, also when `body` fails.
fn with_connected(
    pid: u16,
    body: impl FnOnce(&mut BragiDevice<HidapiTransport>) -> Result<()>,
) -> Result<()> {
    let mut dev = open_bragi(pid)?;
    dev.connect()?;
    eprintln!(
        "connected, scheme {:?}, firmware {:?}",
        dev.scheme(),
        dev.info().firmware
    );
    let result = body(&mut dev);
    let restore = dev.disconnect(None);
    eprintln!(
        "restored hardware mode: {}",
        if restore.is_ok() { "ok" } else { "FAILED" }
    );
    result?;
    restore?;
    Ok(())
}

fn bragi(cmd: BragiCmd, pid: u16) -> Result<()> {
    match cmd {
        BragiCmd::Probe => {
            let mut dev = open_bragi(pid)?;
            let r = dev.probe()?;
            println!("vid       {:#06x}", r.vid);
            println!("pid       {:#06x}", r.pid);
            println!("firmware  {}", r.firmware);
            println!(
                "mode      {} ({})",
                r.mode,
                match r.mode {
                    1 => "hardware",
                    2 => "software",
                    _ => "unknown",
                }
            );
            println!("layout    {:?} (1=ANSI, 2=ISO)", r.layout);
            println!("poll rate {:?}", r.polling_rate);
            Ok(())
        }
        BragiCmd::Color { color, seconds } => {
            let c: Rgb = color.parse()?;
            with_connected(pid, |dev| {
                let n = dev.layout().len();
                let deadline = Instant::now() + Duration::from_secs_f32(seconds);
                // Re-send periodically like a live engine would.
                while Instant::now() < deadline {
                    dev.push_frame(&freergb_core::Frame::solid(n, c))?;
                    std::thread::sleep(Duration::from_millis(200));
                }
                Ok(())
            })
        }
        BragiCmd::Effect {
            effect,
            seconds,
            fps,
        } => with_connected(pid, |dev| {
            let layout = dev.layout().clone();
            let mut fx = effect.spec().build();
            let mut canvas = Canvas::new(
                usize::from(layout.width) * 4,
                usize::from(layout.height) * 4,
            );
            let start = Instant::now();
            let period = Duration::from_secs_f64(1.0 / f64::from(fps.max(1)));
            let mut frames = 0u32;
            while start.elapsed().as_secs_f32() < seconds {
                let t0 = Instant::now();
                fx.render(start.elapsed(), &mut canvas);
                dev.push_frame(&canvas.sample_layout(&layout, Region::FULL))?;
                frames += 1;
                if let Some(rest) = period.checked_sub(t0.elapsed()) {
                    std::thread::sleep(rest);
                }
            }
            eprintln!(
                "{frames} frames in {:.1}s ({:.1} fps)",
                start.elapsed().as_secs_f32(),
                f64::from(frames) / start.elapsed().as_secs_f64()
            );
            Ok(())
        }),
        BragiCmd::KeyWalk { ms_per_key } => with_connected(pid, |dev| {
            let layout = dev.layout().clone();
            for (i, led) in layout.leds.iter().enumerate() {
                let mut f = freergb_core::Frame::solid(layout.len(), Rgb::new(0, 0, 40));
                f.colors_mut()[i] = Rgb::WHITE;
                dev.push_frame(&f)?;
                println!("{:>3} {}", i, led.name);
                std::thread::sleep(Duration::from_millis(ms_per_key));
            }
            Ok(())
        }),
        BragiCmd::Keys {
            assignments,
            seconds,
        } => {
            let layout = BragiModel::for_pid(pid)
                .with_context(|| format!("no Bragi model for PID {pid:04x}"))?
                .layout;
            let mut frame = freergb_core::Frame::solid(layout.len(), Rgb::BLACK);
            for a in &assignments {
                let (name, color) = a
                    .rsplit_once('=')
                    .with_context(|| format!("expected NAME=#rrggbb, got {a:?}"))?;
                let idx = layout
                    .index_of(name)
                    .with_context(|| format!("unknown key {name:?}"))?;
                frame.colors_mut()[idx] = color.parse()?;
            }
            with_connected(pid, |dev| {
                let deadline = Instant::now() + Duration::from_secs_f32(seconds);
                while Instant::now() < deadline {
                    dev.push_frame(&frame)?;
                    std::thread::sleep(Duration::from_millis(200));
                }
                Ok(())
            })
        }
    }
}
