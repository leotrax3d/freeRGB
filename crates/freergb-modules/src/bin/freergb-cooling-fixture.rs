//! Test fixture cooling plugin (MIT). Reads `fixture-policy.json` and
//! `fixture-mode` from its working directory. Modes: ok | crash-sample |
//! hang-sample | out-of-range | undeclared | garbage | violate | io-in-describe |
//! wrong-describe | crash-set.
use freergb_module_api::cooling::sdk::{run, CoolingPlugin};
use freergb_module_api::cooling::{Description, Policy, Sample, SensorValue, API};
use freergb_module_api::sdk::Hid;
use freergb_module_api::{ModuleError, ModuleInfo};
use std::io::Write;

struct Fixture {
    mode: String,
    policy: Policy,
}

impl CoolingPlugin for Fixture {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: "cooling-fixture".into(),
            version: "0.0.1".into(),
            family: "fixture".into(),
            api: API,
        }
    }
    fn describe(&self) -> Description {
        if self.mode == "io-in-describe" {
            // A raw call outside an I/O request; the host must refuse it.
            let mut out = std::io::stdout();
            let _ = writeln!(out, r#"{{"v":3,"call":1,"op":"write","data":"AQE="}}"#);
            let _ = out.flush();
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
        let mut channels = self.policy.channels.clone();
        if self.mode == "wrong-describe" {
            channels[0].floor_percent = 20;
        }
        Description {
            sensors: self.policy.sensors.clone(),
            channels,
        }
    }
    fn sample(&mut self, hid: &mut dyn Hid) -> Result<Sample, ModuleError> {
        match self.mode.as_str() {
            "crash-sample" => std::process::exit(3),
            "hang-sample" => loop {
                std::thread::sleep(std::time::Duration::from_secs(1))
            },
            "garbage" => {
                println!("this is not json");
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
            "violate" => {
                hid.write(&[0x7f, 0x00])?;
            }
            _ => {}
        }
        hid.write(&[0x01, 0x10])?;
        let mut sensors: Vec<SensorValue> = self
            .policy
            .sensors
            .iter()
            .map(|s| SensorValue {
                id: s.id.clone(),
                value: Some(1000.0),
            })
            .collect();
        match self.mode.as_str() {
            "out-of-range" => sensors[0].value = Some(1.0e9),
            "undeclared" => sensors.push(SensorValue {
                id: "ghost".into(),
                value: None,
            }),
            _ => {}
        }
        Ok(Sample { sensors })
    }
    fn set_duty(
        &mut self,
        _channel: &str,
        duty: u8,
        _lease_ms: u64,
        hid: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        if self.mode == "crash-set" {
            std::process::exit(4);
        }
        hid.write(&[0x02, duty])?;
        Ok(())
    }
    fn release(&mut self, _channel: &str, hid: &mut dyn Hid) -> Result<(), ModuleError> {
        hid.write(&[0x03])?;
        Ok(())
    }
    fn shutdown(&mut self, hid: &mut dyn Hid) -> Result<(), ModuleError> {
        hid.write(&[0x04])?;
        Ok(())
    }
}

fn main() {
    let mode = std::fs::read_to_string("fixture-mode").unwrap_or_else(|_| "ok".into());
    let policy = std::fs::read("fixture-policy.json")
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .expect("fixture-policy.json");
    run(Fixture {
        mode: mode.trim().to_owned(),
        policy,
    });
}
