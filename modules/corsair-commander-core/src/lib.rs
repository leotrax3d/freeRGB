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
pub const MODULE_ID: &str = "corsair-commander-core";
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
fn err_io(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Disconnected, e.to_string())
}
#[derive(Default)]
pub struct CommanderCore {
    active: bool,
    packet_size: usize,
    res_size: usize,
    xt: bool,
    version: [u8; 3],
    /// zones in SetDirectColor order (name, led count)
    zones: Vec<(String, usize)>,
}
impl CommanderCore {
    /// upstream: CorsairCommanderCoreController.cpp SendCommand()
    /// Writes the command frame, reads answers until the report id byte is
    /// 0x00 and returns the command response slice (res = &buf[3]).
    fn send_command(
        &mut self,
        cmd: [u8; 2],
        data: Option<&[u8]>,
        read_response: bool,
        h: &mut dyn Hid,
    ) -> Result<Vec<u8>, ModuleError> {
        h.write(&report::command_packet(self.packet_size, cmd, data))
            .map_err(err)?;
        let mut res = Ok(Vec::new());
        if read_response {
            let mut answer;
            loop {
                answer = h.read(self.packet_size, 0).map_err(err_io)?;
                if answer.first() == Some(&0x00) {
                    break;
                }
            }
            res = Ok(answer.iter().skip(3).copied().collect());
        }
        res
    }
    /// upstream: WriteData() - open endpoint, first packet with 6-byte header
    /// plus data chunks (0x06), continuation 0x07 packets, close endpoint.
    fn write_data(
        &mut self,
        endpoint: [u8; 2],
        data_type: [u8; 2],
        data: &[u8],
        h: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        self.send_command(report::endpoints::OPEN_ENDPOINT, Some(&endpoint), false, h)?;
        let mut start = 0usize;
        while start < data.len() {
            if start == 0 {
                let mut chunk = self.packet_size - 10;
                if data.len() < chunk {
                    chunk = data.len();
                }
                let pkt = report::data_packet(data_type, &data[start..start + chunk], data.len());
                self.send_command(report::endpoints::WRITE_FIRST, Some(&pkt), false, h)?;
                start += chunk;
            } else {
                let mut chunk = self.packet_size - 4;
                if data.len() - start < chunk {
                    chunk = data.len() - start;
                }
                self.send_command(
                    report::endpoints::WRITE_REST,
                    Some(&data[start..start + chunk]),
                    false,
                    h,
                )?;
                start += chunk;
            }
        }
        self.send_command(report::endpoints::CLOSE_ENDPOINT, None, false, h)?;
        Ok(())
    }
    /// upstream: ReadData() - open endpoint, read data, close endpoint.
    fn read_data(&mut self, endpoint: [u8; 2], h: &mut dyn Hid) -> Result<Vec<u8>, ModuleError> {
        self.send_command(report::endpoints::OPEN_ENDPOINT, Some(&endpoint), false, h)?;
        let res = self.send_command(report::endpoints::READ, None, true, h)?;
        self.send_command(report::endpoints::CLOSE_ENDPOINT, None, false, h)?;
        Ok(res)
    }
    /// upstream: CorsairCommanderCoreController.cpp InitController()
    fn init_controller(&mut self, pid: u16, h: &mut dyn Hid) -> Result<(), ModuleError> {
        self.packet_size = report::PACKET_V2;
        self.res_size = self.packet_size - 4;
        if pid == report::PID_CORE2 {
            self.packet_size = report::PACKET_V3;
            self.res_size = self.packet_size - 4;
        } else if pid == report::PID_CORE_XT {
            self.packet_size = report::PACKET_XT;
            self.res_size = self.packet_size - 4;
        }
        // version query
        let res = self.send_command(report::endpoints::VERSION, None, true, h)?;
        self.version = [res[0], res[1], res[2]];
        if pid == report::PID_CORE && self.version[0] == 1 {
            self.packet_size = report::PACKET_V1;
            self.res_size = self.packet_size - 4;
        }
        self.set_fan_mode(pid, false, h)
    }
    /// upstream: CorsairCommanderCoreController.cpp SetFanMode(...)
    fn set_fan_mode(
        &mut self,
        pid: u16,
        external_rgb_port: bool,
        h: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        let buf = report::fan_mode_packet(pid, external_rgb_port);
        self.write_data(
            report::endpoints::FAN_ENDPOINT,
            report::endpoints::FAN_DATA_TYPE,
            &buf,
            h,
        )?;
        // wake the device
        self.send_command(
            report::endpoints::WAKE_HW,
            Some(&report::endpoints::WAKE_DATA_INIT),
            true,
            h,
        )?;
        Ok(())
    }
}
impl Module for CommanderCore {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CorsairCommanderCoreController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize_with_zones(
        &mut self,
        i: &InterfaceInfo,
        z: Option<&[Zone]>,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::detector(i).ok_or_else(|| err("no Corsair Commander Core detector"))?;
        let pid = d.pid;
        self.xt = pid == report::PID_CORE_XT;
        // upstream: InitController()
        self.init_controller(pid, h)?;
        // upstream: RGBController SetupZones(): non-XT reads the per-port LED
        // counts (endpoint 0x20); XT keeps a manually configurable header.
        let mut zone_counts: Vec<(String, usize)> = Vec::new();
        if self.xt {
            zone_counts.push(("Corsair RGB Header".to_string(), 0));
        } else {
            let res = self.read_data(report::endpoints::LED_COUNT_ENDPOINT, h)?;
            let counts = report::led_counts_from_response(&res);
            let pump = counts
                .first()
                .copied()
                .ok_or_else(|| err("Commander Core did not report a pump LED count"))?;
            zone_counts.push(("Pump".to_string(), pump));
        }
        // fan headers 1..6 start at 0 LEDs; the host may configure them
        for idx in 1..=report::NUM_CHANNELS {
            let host = z
                .and_then(|zs| zs.get(idx))
                .map(|zz| zz.leds)
                .unwrap_or(0)
                .min(34);
            zone_counts.push((format!("Corsair Fan Header {}", idx), host));
        }
        // XT hosts may also configure zone 0
        if self.xt {
            if let Some(first) = z.and_then(|zs| zs.first()) {
                zone_counts[0].1 = first.leds.min(204);
            }
        }
        self.zones = zone_counts;
        self.active = true;
        let count: usize = self.zones.iter().map(|z| z.1).sum();
        Ok(Capabilities {
            name: d.name.into(),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: count as u16,
                height: 1,
                leds: led_list(&self.zones),
            },
            firmware: Some(format!(
                "v{}.{}.{}",
                self.version[0], self.version[1], self.version[2]
            )),
            restore: Restore::LastFrame,
            zones: self
                .zones
                .iter()
                .map(|(name, leds)| Zone {
                    name: name.clone(),
                    leds: *leds,
                })
                .collect(),
        })
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, h)
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active {
            return Err(err("Corsair Commander Core module is not initialized"));
        }
        let total: usize = self.zones.iter().map(|z| z.1).sum();
        if c.len() != total {
            return Err(err("invalid Commander Core frame"));
        }
        // upstream: DeviceUpdateMode() -> SetDirectColor(colors, zones)
        let counts: Vec<usize> = self.zones.iter().map(|z| z.1).collect();
        let (buf, offset) = report::rgb_buffer(self.xt, &counts, c);
        self.write_data(
            report::endpoints::RGB_ENDPOINT,
            report::endpoints::RGB_DATA_TYPE,
            &buf[..offset],
            h,
        )?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, h: &mut dyn Hid) -> Result<(), ModuleError> {
        if self.active {
            // upstream: destructor sends the hardware mode wake command
            // [01 03] with data [00 01]
            self.send_command(
                report::endpoints::WAKE_HW,
                Some(&report::endpoints::HW_MODE_DATA),
                true,
                h,
            )?;
        }
        self.active = false;
        Ok(())
    }
}
fn led_list(zones: &[(String, usize)]) -> Vec<Led> {
    let mut leds = Vec::new();
    let mut idx = 0usize;
    for (name, count) in zones {
        for k in 0..*count {
            leds.push(Led {
                name: format!("{}, LED {}", name, k + 1),
                pos: (idx as u16, 0),
            });
            idx += 1;
        }
    }
    leds
}
#[cfg(test)]
mod tests;
