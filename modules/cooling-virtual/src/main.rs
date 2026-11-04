//! Virtual cooling plugin: one simulated fan and pump on a liquid loop.
//!
//! The reference implementation of the API 3 contract (`--print-policy` emits
//! the policy that is signed into its package). No I/O: duty only changes the
//! simulated RPM and liquid temperature. A lease that is not renewed returns
//! the channel to its simulated firmware mode, like real firmware would.
use freergb_module_api::cooling::sdk::{run, CoolingPlugin};
use freergb_module_api::cooling::{Description, Policy, Sample, SensorValue, API};
use freergb_module_api::sdk::Hid;
use freergb_module_api::{ErrorCode, ModuleError, ModuleInfo};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub const POLICY: &str = include_str!("../cooling-policy.json");
/// Duty the simulated firmware runs when no host controls a channel.
const FIRMWARE_FAN: u8 = 60;

struct Virtual {
    policy: Policy,
    /// Channel -> (duty, lease deadline).
    manual: BTreeMap<String, (u8, Instant)>,
}

impl Virtual {
    fn duty(&mut self, channel: &str) -> u8 {
        let now = Instant::now();
        self.manual.retain(|_, (_, until)| *until > now);
        match self.manual.get(channel) {
            Some((duty, _)) => *duty,
            None if channel == "pump" => 100,
            None => FIRMWARE_FAN,
        }
    }
}

impl CoolingPlugin for Virtual {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: "cooling-virtual".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "virtual".into(),
            api: API,
        }
    }

    fn describe(&self) -> Description {
        Description {
            sensors: self.policy.sensors.clone(),
            channels: self.policy.channels.clone(),
        }
    }

    fn sample(&mut self, _: &mut dyn Hid) -> Result<Sample, ModuleError> {
        let fan = f64::from(self.duty("fan1"));
        let pump = f64::from(self.duty("pump"));
        let values = [
            ("liquid", 42.0 - (fan - 40.0) * 0.12),
            ("fan1_rpm", fan * 18.0),
            ("pump_rpm", pump * 28.0),
        ];
        Ok(Sample {
            sensors: values
                .into_iter()
                .map(|(id, value)| SensorValue {
                    id: id.into(),
                    value: Some(value),
                })
                .collect(),
        })
    }

    fn set_duty(
        &mut self,
        channel: &str,
        duty: u8,
        lease_ms: u64,
        _: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        // The host already enforced floors and pump rules; repeat them here so
        // the reference plugin never trusts its caller either.
        let Some(decl) = self.policy.channel(channel) else {
            return Err(ModuleError::new(ErrorCode::Unsupported, "unknown channel"));
        };
        if !(decl.floor_percent..=100).contains(&duty) {
            return Err(ModuleError::new(ErrorCode::Permission, "duty below floor"));
        }
        let until = Instant::now() + Duration::from_millis(lease_ms.min(3000));
        self.manual.insert(channel.to_owned(), (duty, until));
        Ok(())
    }

    fn release(&mut self, channel: &str, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.manual.remove(channel);
        Ok(())
    }

    fn shutdown(&mut self, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.manual.clear();
        Ok(())
    }
}

fn main() {
    if std::env::args().any(|a| a == "--print-policy") {
        print!("{POLICY}");
        return;
    }
    let policy: Policy = serde_json::from_str(POLICY).expect("embedded policy");
    run(Virtual {
        policy,
        manual: BTreeMap::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_policy_is_valid() {
        let policy: Policy = serde_json::from_str(POLICY).unwrap();
        policy.validate().unwrap();
        assert!(policy.is_virtual());
    }

    #[test]
    fn lease_expiry_returns_to_firmware() {
        let mut plugin = Virtual {
            policy: serde_json::from_str(POLICY).unwrap(),
            manual: BTreeMap::new(),
        };
        plugin.manual.insert(
            "fan1".into(),
            (45, Instant::now() - Duration::from_millis(1)),
        );
        assert_eq!(plugin.duty("fan1"), FIRMWARE_FAN);
        assert_eq!(plugin.duty("pump"), 100);
    }
}
