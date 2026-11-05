// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};

pub const MODULE_ID: &str = "corsair-peripheral";
#[derive(Default)]
pub struct CorsairPeripheral {
    active: bool,
    kind: u8, // 0=unknown,1=mouse,2=mat,3=headset
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
fn firmware_type(h: &mut dyn Hid) -> Result<u8, ModuleError> {
    for _ in 0..10usize {
        let mut buf = vec![0u8; 65];
        buf[1] = 0x0E;
        buf[2] = 0x01;
        if h.write(&buf).is_ok() {
            match h.read(65, 1000) {
                Ok(v) if v.len() > 0x15 && (v[0x14] == 0xC1 || v[0x14] == 0xC2) => {
                    return Ok(v[0x14]);
                }
                _ => {}
            }
        }
        let mut buf = vec![0u8; 65];
        buf[1] = 0x0E;
        buf[2] = 0x01;
        let _ = h.send_feature(&buf);
        if let Ok(v) = h.get_feature(&[0x00], 65) {
            if v.len() > 0x15 && (v[0x14] == 0xC1 || v[0x14] == 0xC2) {
                return Ok(v[0x14]);
            }
        }
    }
    Err(err("could not determine firmware type"))
}
impl Module for CorsairPeripheral {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CorsairPeripheralController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }
        let typ = firmware_type(h)?;
        // C1 mouse, C2 mousemat (default) or headset stand.
        let (kind, leds, layout, zones, kind_byte, k) = match typ {
            0xC1 => {
                let (l, z) = mouse_layout();
                (DeviceKind::Mouse, 15usize, l, z, 0x01u8, 1u8)
            }
            0xC2 if i.pid == 0x0a34 => (
                DeviceKind::LightingController,
                9usize,
                (0..9usize)
                    .map(|n| Led {
                        name: format!("Stand LED {}", n + 1),
                        pos: (n as u16, 0),
                    })
                    .collect(),
                Zone {
                    name: "Base".into(),
                    leds: 9,
                },
                0x03u8,
                3u8,
            ),
            0xC2 => (
                DeviceKind::LightingController,
                15usize,
                (0..15usize)
                    .map(|n| Led {
                        name: format!("Mousemat LED {}", n + 1),
                        pos: (n as u16, 0),
                    })
                    .collect(),
                Zone {
                    name: "Mousemat".into(),
                    leds: 15,
                },
                0x04u8,
                2u8,
            ),
            _ => return Err(err("generic keyboard not ported")),
        };
        // LightingControl software
        h.write(&report::lighting_control_header_bytes(kind_byte))
            .map_err(err)?;
        self.active = true;
        self.kind = k;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Corsair peripheral".into()),
            kind,
            layout: LedLayout {
                width: 1,
                height: leds as u16,
                leds: layout,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![zones],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active {
            return Err(err("Corsair peripheral module is not initialized"));
        }
        if self.kind == 1 {
            if colors.len() != 15 {
                return Err(err("invalid mouse frame"));
            }
            h.write(&report::mouse_direct(colors)).map_err(err)?;
        } else if self.kind == 2 {
            if colors.len() != 15 {
                return Err(err("invalid mousemat frame"));
            }
            h.write(&report::mousemat_direct(colors)).map_err(err)?;
        } else if self.kind == 3 {
            if colors.len() != 9 {
                return Err(err("invalid headset stand frame"));
            }
            h.write(&report::stand_direct(colors)).map_err(err)?;
        } else {
            return Err(err("unsupported device"));
        }
        Ok(())
    }
    fn shutdown(
        &mut self,
        _: Option<freergb_core::Rgb>,
        _: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
fn mouse_layout() -> (Vec<Led>, Zone) {
    (
        (0..15usize)
            .map(|n| Led {
                name: format!("Mouse LED {}", n + 1),
                pos: (n as u16, 0),
            })
            .collect(),
        Zone {
            name: "Mouse".into(),
            leds: 15,
        },
    )
}
#[cfg(test)]
mod tests;
