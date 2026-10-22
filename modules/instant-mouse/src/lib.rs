// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, MODULE_API,
};
pub const MODULE_ID: &str = "instant-mouse";
#[derive(Default)]
pub struct A {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for A {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "InstantMouseController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        table::device(i).ok_or_else(|| err("unsupported Instant mouse"))?;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Instant mouse".into()),
            kind: DeviceKind::Mouse,
            layout: LedLayout {
                width: 1,
                height: 1,
                leds: (0..1)
                    .map(|n| Led {
                        name: format!("LED {n}"),
                        pos: ((n % 14) as u16, (n / 14) as u16),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != 1 {
            return Err(err("invalid Instant mouse frame"));
        }
        for packet in report::frames(c[0]) {
            h.send_feature(&packet).map_err(err)?;
        }
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        sent: Vec<Vec<u8>>,
    }

    impl Hid for Fake {
        fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
            panic!("Instant Mouse uses feature reports")
        }
        fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
            panic!("Instant Mouse does not read during direct frames")
        }
        fn send_feature(&mut self, report: &[u8]) -> Result<usize, ModuleError> {
            self.sent.push(report.to_vec());
            Ok(report.len())
        }
        fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
            panic!("Instant Mouse does not read during direct frames")
        }
    }

    fn interface() -> InterfaceInfo {
        InterfaceInfo {
            vid: 0x30fa,
            pid: 0x1030,
            interface: 1,
            usage_page: 0xff01,
            usage: 1,
            product: None,
        }
    }

    #[test]
    fn catalog() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 4);
    }

    #[test]
    fn detector_requires_the_complete_hid_match() {
        assert!(table::identify(&interface()).supported);
        let mut wrong = interface();
        wrong.usage_page = 0;
        assert!(!table::identify(&wrong).supported);
    }

    #[test]
    fn direct_initialize_frame_shutdown_sequence_is_exact() {
        // OpenRGB RGBController_InstantMouse.cpp:168-199 and
        // InstantMouseController.cpp:57-80.
        let mut module = A::default();
        let mut hid = Fake::default();
        let capabilities = module.initialize(&interface(), &mut hid).unwrap();
        assert_eq!(capabilities.layout.leds.len(), 1);
        module
            .frame(
                &[Rgb {
                    r: 0,
                    g: 255,
                    b: 128,
                }],
                &mut hid,
            )
            .unwrap();
        module.shutdown(None, &mut hid).unwrap();
        assert_eq!(
            hid.sent,
            report::frames(Rgb {
                r: 0,
                g: 255,
                b: 128
            })
        );
    }
}
