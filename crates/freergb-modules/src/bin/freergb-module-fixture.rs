//! Test fixture module (MIT). Behaviour comes from `fixture-mode` in its
//! working directory: ok | crash-frame | hang-frame | bad-layout | refuse.
use freergb_core::{LedLayout, Rgb};
use freergb_module_api::sdk::{run, Hid, Module};
use freergb_module_api::*;

struct Fixture {
    mode: String,
    probed: bool,
}

impl Module for Fixture {
    fn tick_interval_ms(&self) -> Option<u64> {
        match self.mode.as_str() {
            "tick" | "tick-error" | "tick-hang" => Some(MIN_TICK_MS),
            "tick-fast" => Some(MIN_TICK_MS - 1),
            "tick-slow" => Some(MAX_TICK_MS + 1),
            "tick-max" => Some(MAX_TICK_MS),
            _ => None,
        }
    }
    fn tick(&mut self, hid: &mut dyn Hid) -> Result<(), ModuleError> {
        match self.mode.as_str() {
            "tick-error" => return Err(ModuleError::new(ErrorCode::Protocol, "tick failed")),
            "tick-hang" => loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            },
            _ => {}
        }
        hid.write(&[3, 0xAB])?;
        hid.delay_ms(13)?;
        hid.write(&[4, 0xCD])?;
        Ok(())
    }
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
    fn identify_with_platform(
        &self,
        interface: &InterfaceInfo,
        platform: Option<&PlatformIdentity>,
    ) -> Identification {
        let mut id = self.identify(interface);
        if self.mode == "platform-required" && !matches_platform(platform) {
            id.supported = false;
            id.reason = Some("fixture board identity does not match".into());
        }
        id
    }
    fn initialize_with_platform(
        &mut self,
        interface: &InterfaceInfo,
        zones: Option<&[Zone]>,
        platform: Option<&PlatformIdentity>,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        if self.mode == "platform-required" && !matches_platform(platform) {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                "fixture board identity does not match",
            ));
        }
        self.initialize_with_zones(interface, zones, hid)
    }
    fn probe(
        &mut self,
        interface: &InterfaceInfo,
        hid: &mut dyn freergb_module_api::probe::Hid,
    ) -> Result<Identification, ModuleError> {
        if self.mode.starts_with("probe") {
            let serial = hid.serial_string()?;
            let product = hid.product_string()?;
            let feature = hid.feature(0x52, 200)?;
            let supported = serial.as_deref() == Some("7D03-test")
                && product.as_deref() == Some("Simulated controller")
                && feature == [0x52, 1, 2, 3]
                && self.mode != "probe-refuse";
            if self.mode == "probe-error" {
                return Err(ModuleError::new(
                    ErrorCode::Internal,
                    serial.unwrap_or_default(),
                ));
            }
            self.probed = supported;
            let mut id = self.identify(interface);
            id.supported = supported;
            return Ok(id);
        }
        Ok(self.identify(interface))
    }
    fn initialize(
        &mut self,
        interface: &InterfaceInfo,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        if self.mode.starts_with("probe") && !self.probed {
            return Err(ModuleError::new(ErrorCode::Protocol, "probe state missing"));
        }
        initialize_inner(&self.mode, interface, None, hid)
    }
    fn initialize_with_zones(
        &mut self,
        interface: &InterfaceInfo,
        zones: Option<&[Zone]>,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        if self.mode.starts_with("probe") && !self.probed {
            return Err(ModuleError::new(ErrorCode::Protocol, "probe state missing"));
        }
        initialize_inner(&self.mode, interface, zones, hid)
    }
    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if self.mode == "delay-budget" {
            for _ in 0..11 {
                hid.delay_ms(MAX_DELAY_MS)?;
            }
        }
        match self.mode.as_str() {
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

fn matches_platform(platform: Option<&PlatformIdentity>) -> bool {
    platform.is_some_and(|p| {
        p.board_vendor.as_deref() == Some("Fixture Vendor")
            && p.board_name.as_deref() == Some("Fixture Board A (FB-1)")
            && p.product.as_deref() == Some("Fixture Product")
    })
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
    run(Fixture {
        mode: mode.trim().to_owned(),
        probed: false,
    });
}
