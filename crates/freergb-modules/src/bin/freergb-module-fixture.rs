//! Test fixture module (MIT). Behaviour comes from `fixture-mode` in its
//! working directory: ok | crash-frame | hang-frame | bad-layout | refuse.
use freergb_core::{LedLayout, Rgb};
use freergb_module_api::sdk::{run, Hid, Module};
use freergb_module_api::*;

struct Fixture(String);

impl Module for Fixture {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: "fixture".into(),
            version: "0.0.1".into(),
            family: "fixture".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, _: &InterfaceInfo) -> Identification {
        Identification {
            supported: true,
            model: Some("Fixture".into()),
            kind: Some(DeviceKind::Mouse),
            reason: None,
        }
    }
    fn initialize(
        &mut self,
        interface: &InterfaceInfo,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        initialize_inner(&self.0, interface, None, hid)
    }
    fn initialize_with_zones(
        &mut self,
        interface: &InterfaceInfo,
        zones: Option<&[Zone]>,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        initialize_inner(&self.0, interface, zones, hid)
    }
    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        match self.0.as_str() {
            "crash-frame" => std::process::exit(3),
            "hang-frame" => loop {
                std::thread::sleep(std::time::Duration::from_secs(1))
            },
            _ => {}
        }
        let mut report = vec![1];
        for c in colors {
            report.extend([c.r, c.g, c.b]);
        }
        hid.write(&report)?;
        Ok(())
    }
    fn shutdown(&mut self, color: Option<Rgb>, hid: &mut dyn Hid) -> Result<(), ModuleError> {
        hid.write(&[2, color.map_or(0, |c| c.r)])?;
        Ok(())
    }
}

fn initialize_inner(
    mode: &str,
    _: &InterfaceInfo,
    configured_zones: Option<&[Zone]>,
    hid: &mut dyn Hid,
) -> Result<Capabilities, ModuleError> {
    if mode == "refuse" {
        return Err(ModuleError::new(ErrorCode::Unsupported, "fixture refuses"));
    }
    hid.write(&[0, 0xAB])?;
    let echo = hid.read(8, 100)?;
    if echo.first() != Some(&0xAB) {
        return Err(ModuleError::new(ErrorCode::Protocol, "no echo"));
    }
    let leds = if mode == "bad-layout" {
        0
    } else {
        configured_zones
            .map(|zones| zones.iter().map(|zone| zone.leds).sum())
            .unwrap_or(2)
    };
    Ok(Capabilities {
        name: "Fixture Mouse".into(),
        kind: DeviceKind::Mouse,
        layout: LedLayout::strip(
            u16::try_from(leds)
                .map_err(|_| ModuleError::new(ErrorCode::Protocol, "too many fixture LEDs"))?,
        ),
        firmware: Some("v1".into()),
        restore: Restore::LastFrame,
        zones: configured_zones.map_or_else(Vec::new, |zones| zones.to_vec()),
    })
}

fn main() {
    let mode = std::fs::read_to_string("fixture-mode").unwrap_or_else(|_| "ok".into());
    run(Fixture(mode.trim().to_owned()));
}
