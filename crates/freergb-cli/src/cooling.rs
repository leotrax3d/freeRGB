//! `freergb cooling ...`: scriptable front end for the daemon's cooling API
//! (docs/LOCAL_API.md "Cooling CLI"). Every action goes through the same
//! daemon policy as the GUIs; the CLI never talks to hardware itself.
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::{json, Value};

#[derive(Args)]
pub struct CoolingArgs {
    #[command(subcommand)]
    cmd: CoolingCmd,
    /// Print the daemon's JSON result instead of text.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum CoolingCmd {
    /// Fan/pump channels with tier, reading and control state.
    List,
    /// Running channels, remembered assignments, boot service and active profile.
    Status,
    /// Fixed duty on one channel (floor, critical temperature and supervision apply).
    SetManual {
        control_id: String,
        percent: u8,
        /// Temperature source(s) for critical-temperature protection.
        #[arg(long)]
        temperature: String,
        /// Tachometer of the same fan.
        #[arg(long)]
        rpm: String,
        /// You physically confirmed this header drives a normal fan.
        #[arg(long)]
        confirmed_fan: bool,
    },
    /// Return one channel to firmware control.
    Stop { control_id: String },
    /// Return every channel to firmware control.
    StopAll,
    /// Saved profiles.
    Profiles,
    /// Save the current complete configuration as a profile.
    Save { name: String },
    /// Switch to a saved profile.
    Apply { name: String },
    /// Delete a saved profile.
    Delete { name: String },
    /// Write a profile as a versioned JSON document (stdout without --out).
    Export {
        name: String,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// Import a profile document; identities are remapped to this board.
    Import {
        file: std::path::PathBuf,
        /// Store under this name instead of the document's.
        #[arg(long)]
        name: Option<String>,
        /// Map a source device to a local one, e.g. /lpc/it8689e/0=/lpc/nct6701d/0.
        #[arg(long = "remap", value_name = "FROM=TO")]
        remap: Vec<String>,
    },
    /// Show or change how the boot-time cooling service runs.
    Service {
        #[arg(long, value_enum)]
        mode: Option<Mode>,
        /// Profile applied at boot (empty string clears it).
        #[arg(long)]
        boot_profile: Option<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Login,
    Service,
}

fn call(method: &str, params: Value) -> Result<Value> {
    freergb_client::call(method, params).with_context(|| format!("{method} failed"))
}

pub fn run(args: CoolingArgs) -> Result<()> {
    let json_out = args.json;
    let print = |value: &Value, text: &dyn Fn(&Value) -> String| -> Result<()> {
        if json_out {
            println!("{}", serde_json::to_string_pretty(value)?);
        } else {
            println!("{}", text(value));
        }
        Ok(())
    };
    match args.cmd {
        CoolingCmd::List => {
            let channels = call("cooling.channels.list", Value::Null)?;
            print(&channels, &|v| {
                let mut lines = vec![format!("{:<36} {:<8} {:>8}  name", "id", "kind", "value")];
                for c in v.as_array().into_iter().flatten() {
                    lines.push(format!(
                        "{:<36} {:<8} {:>8}  {}",
                        c["sensor_id"]
                            .as_str()
                            .unwrap_or(c["id"].as_str().unwrap_or("?")),
                        c["kind"].as_str().unwrap_or("?"),
                        c["reading"]["value"]
                            .as_f64()
                            .map_or("-".into(), |x| format!("{x:.0}")),
                        c["name"].as_str().unwrap_or("")
                    ));
                }
                lines.join("\n")
            })
        }
        CoolingCmd::Status => {
            let status = json!({
                "sessions": call("cooling.sessions.list", Value::Null)?,
                "service": call("cooling.service.get", Value::Null)?,
                "profiles": call("cooling.profiles.list", Value::Null)?,
            });
            print(&status, &|v| {
                let mut lines = vec![format!(
                    "service: mode={} headless={} boot={}",
                    v["service"]["mode"].as_str().unwrap_or("?"),
                    v["service"]["headless"],
                    v["service"]["boot"]["state"].as_str().unwrap_or("?"),
                )];
                lines.push(format!(
                    "active profile: {}",
                    v["profiles"]["active"].as_str().unwrap_or("-")
                ));
                for job in v["sessions"]["active"].as_array().into_iter().flatten() {
                    lines.push(format!(
                        "{}  {}  {}",
                        job["control_id"].as_str().unwrap_or("?"),
                        job["mode"].as_str().unwrap_or("?"),
                        job["state"].as_str().unwrap_or("?")
                    ));
                }
                let assignments = &v["sessions"]["assignments"]["assignments"];
                for a in assignments.as_array().into_iter().flatten() {
                    if let Some(reason) = a["needs_attention"].as_str() {
                        lines.push(format!(
                            "{}  needs attention: {reason}",
                            a["control_id"].as_str().unwrap_or("?")
                        ));
                    }
                }
                lines.join("\n")
            })
        }
        CoolingCmd::SetManual {
            control_id,
            percent,
            temperature,
            rpm,
            confirmed_fan,
        } => {
            if percent > 100 {
                bail!("percent must be 0..100");
            }
            let job = call(
                "cooling.sessions.start",
                json!({"mode":"manual","control_id":control_id,"temperature_id":temperature,
                    "rpm_id":rpm,"confirmed_fan":confirmed_fan,"curve":null,"critical_c":null,
                    "duty_percent":percent}),
            )?;
            print(&job, &|v| format!("started job {v}"))
        }
        CoolingCmd::Stop { control_id } => {
            let r = call(
                "cooling.sessions.stop_channel",
                json!({ "control_id": control_id }),
            )?;
            print(&r, &|_| "stop requested".into())
        }
        CoolingCmd::StopAll => {
            let r = call("cooling.sessions.stop_all", Value::Null)?;
            print(&r, &|v| format!("stop requested: {v}"))
        }
        CoolingCmd::Profiles => {
            let r = call("cooling.profiles.list", Value::Null)?;
            print(&r, &|v| {
                let active = v["active"].as_str();
                v["profiles"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|p| {
                        let name = p["name"].as_str().unwrap_or("?");
                        format!(
                            "{} {name}  (background {}, live {})",
                            if Some(name) == active { "*" } else { " " },
                            p["channels"],
                            p["live_channels"]
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        CoolingCmd::Save { name } => {
            let r = call("cooling.profiles.save", json!({ "name": name }))?;
            print(&r, &|_| "saved".into())
        }
        CoolingCmd::Apply { name } => {
            let r = call("cooling.profiles.activate", json!({ "name": name }))?;
            print(&r, &|v| {
                format!(
                    "activated {}; started {}, restarting {}, background applied: {}",
                    v["profile"], v["started"], v["restarting"], v["background_applied"]
                )
            })
        }
        CoolingCmd::Delete { name } => {
            let r = call("cooling.profiles.delete", json!({ "name": name }))?;
            print(&r, &|v| format!("deleted: {}", v["deleted"]))
        }
        CoolingCmd::Export { name, out } => {
            let document = call("cooling.profiles.export", json!({ "name": name }))?;
            let text = serde_json::to_string_pretty(&document)?;
            match out {
                Some(path) => {
                    std::fs::write(&path, text)?;
                    if json_out {
                        println!("{}", json!({"written": path}));
                    } else {
                        println!("written to {}", path.display());
                    }
                }
                None => println!("{text}"),
            }
            Ok(())
        }
        CoolingCmd::Import { file, name, remap } => {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("cannot read {}", file.display()))?;
            let mut map = serde_json::Map::new();
            for entry in remap {
                let (from, to) = entry.split_once('=').context("--remap expects FROM=TO")?;
                map.insert(from.into(), json!(to));
            }
            let r = call(
                "cooling.profiles.import",
                json!({"document": text, "name": name, "remap": map}),
            )?;
            print(&r, &|v| {
                let report = &v["report"];
                let mut text = format!(
                    "imported {} (remapped {} ids, same board: {})",
                    v["name"], report["remapped"], report["same_board"]
                );
                if report["needs_confirmation"] == true {
                    text.push_str("\nfan confirmations cleared: confirm each channel again");
                }
                if let Some(missing) = report["unresolved"].as_array().filter(|a| !a.is_empty()) {
                    text.push_str(&format!("\nnot on this system: {missing:?}"));
                }
                text
            })
        }
        CoolingCmd::Service { mode, boot_profile } => {
            if mode.is_some() || boot_profile.is_some() {
                let current = call("cooling.service.get", Value::Null)?;
                let mode = match mode {
                    Some(Mode::Login) => json!("login"),
                    Some(Mode::Service) => json!("service"),
                    None => current["mode"].clone(),
                };
                let boot = match boot_profile {
                    Some(name) if name.is_empty() => Value::Null,
                    Some(name) => json!(name),
                    None => current["boot_profile"].clone(),
                };
                call(
                    "cooling.service.set",
                    json!({"mode": mode, "boot_profile": boot}),
                )?;
            }
            let r = call("cooling.service.get", Value::Null)?;
            print(&r, &|v| {
                format!(
                    "mode={} boot_profile={} headless={} boot={}",
                    v["mode"], v["boot_profile"], v["headless"], v["boot"]
                )
            })
        }
    }
}
