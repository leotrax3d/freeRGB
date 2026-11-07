//! Synthetic API 4 module. Reads only its packaged fixture-policy.json.
//! No device discovery, operating system transport or family implementation.
use freergb_core::{LedLayout, Rgb};
use freergb_module_api::transport::sdk::{run, Bus, Module};
use freergb_module_api::transport::{Identity, Policy, API};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, ModuleError, ModuleInfo, Restore,
};

struct Fixture {
    policy: Policy,
    mode: String,
}
impl Fixture {
    fn execute(&self, id: &str, colors: &[Rgb], bus: &mut dyn Bus) -> Result<(), ModuleError> {
        let r = self.policy.recipes.iter().find(|r| r.id == id).unwrap();
        let mut steps = r.steps.clone();
        let rgb: Vec<u8> = colors.iter().flat_map(|c| [c.r, c.g, c.b]).collect();
        let mut index = 0;
        for (step, masks) in steps.iter_mut().zip(&r.masks) {
            for (byte, mask) in step.data.iter_mut().zip(masks) {
                if *mask != 0 && !rgb.is_empty() {
                    *byte = (*byte & !mask) | (rgb[index % rgb.len()] & mask);
                    index += 1;
                }
            }
        }
        if id == "frame" && self.mode == "deny" {
            steps[0].data.push(0xff);
        }
        bus.transaction(id, steps)
            .map(|_| ())
            .map_err(|e| ModuleError::new(ErrorCode::Protocol, format!("{e:?}")))
    }
}
impl Module for Fixture {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: "transport-fixture".into(),
            version: "0.0.1".into(),
            family: "fixture".into(),
            api: API,
        }
    }
    fn identify(&self, identity: Identity) -> Identification {
        Identification {
            supported: self.policy.identities.contains(&identity),
            model: Some("Simulated transport".into()),
            kind: Some(DeviceKind::LightingController),
            reason: None,
        }
    }
    fn initialize(&mut self, _: Identity, bus: &mut dyn Bus) -> Result<Capabilities, ModuleError> {
        self.execute("initialize", &[], bus)?;
        Ok(Capabilities {
            name: "Transport fixture".into(),
            kind: DeviceKind::LightingController,
            layout: LedLayout::strip(1),
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], bus: &mut dyn Bus) -> Result<(), ModuleError> {
        match self.mode.as_str() {
            "crash" => std::process::exit(3),
            "hang" => std::thread::sleep(std::time::Duration::from_secs(30)),
            _ => {}
        }
        self.execute("frame", colors, bus)
    }
    fn shutdown(&mut self, bus: &mut dyn Bus) -> Result<(), ModuleError> {
        self.execute("shutdown", &[], bus)
    }
}
fn main() {
    let policy = serde_json::from_slice(&std::fs::read("fixture-policy.json").unwrap()).unwrap();
    let mode = std::fs::read_to_string("fixture-mode").unwrap_or_default();
    run(Fixture { policy, mode });
}
