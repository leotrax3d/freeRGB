//! Independently implemented ASUS Aura motherboard USB protocol (all mainboard
//! PIDs listed in docs/MAINBOARD_RGB.md).
//! Wire facts and limits: docs/MAINBOARD_RGB.md. No fan-power commands or flash commit.
use freergb_core::{Frame, LedLayout, Rgb};
use freergb_device_api::{
    check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, HidTransport,
    RgbDevice,
};

fn invalid(message: impl Into<String>) -> DeviceError {
    DeviceError::Protocol(message.into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub fixed_leds: u8,
    pub rgb_headers: u8,
    pub argb_headers: u8,
}

impl Capabilities {
    pub fn decode(report: &[u8]) -> Result<Self, DeviceError> {
        if !(64..=65).contains(&report.len()) || report[0] != 0xec || report[1] != 0x30 {
            return Err(invalid("invalid Aura configuration reply"));
        }
        let capabilities = Self {
            fixed_leds: report[4 + 0x1b],
            rgb_headers: report[4 + 0x1d],
            argb_headers: report[4 + 2],
        };
        if capabilities.fixed_leds > 16
            || capabilities.rgb_headers > capabilities.fixed_leds
            || capabilities.argb_headers > 4
        {
            return Err(invalid("unsupported Aura header/LED configuration"));
        }
        Ok(capabilities)
    }
}

#[derive(Clone)]
struct Zone {
    effect: u8,
    direct: u8,
    count: usize,
}

fn direct_packets(channel: u8, colors: &[Rgb]) -> Result<Vec<[u8; 65]>, DeviceError> {
    if channel > 4 || colors.is_empty() || colors.len() > 512 {
        return Err(invalid("Aura direct channel/count out of range"));
    }
    let mut packets = Vec::new();
    for (part, chunk) in colors.chunks(20).enumerate() {
        let offset = part * 20;
        let mut packet = [0; 65];
        packet[..5].copy_from_slice(&[
            0xec,
            0x40,
            channel
                | if offset >= 256 { 0x10 } else { 0 }
                | if offset + chunk.len() == colors.len() {
                    0x80
                } else {
                    0
                },
            offset as u8,
            chunk.len() as u8,
        ]);
        for (index, color) in chunk.iter().enumerate() {
            packet[5 + index * 3..8 + index * 3].copy_from_slice(&color.to_array());
        }
        packets.push(packet);
    }
    Ok(packets)
}

fn effect_packet(channel: u8, mode: u8) -> [u8; 65] {
    let mut packet = [0; 65];
    packet[..6].copy_from_slice(&[0xec, 0x35, channel, 0, 0, mode]);
    packet
}

pub struct AuraDevice<T: HidTransport> {
    transport: T,
    info: DeviceInfo,
    layout: LedLayout,
    argb_leds: [u16; 4],
    names: [Option<String>; 4],
    zones: Vec<Zone>,
    owned: Vec<u8>,
    status: DeviceStatus,
}

impl<T: HidTransport> AuraDevice<T> {
    pub fn new(transport: T, pid: u16, serial: Option<String>, argb_leds: [u16; 4]) -> Self {
        Self {
            transport,
            info: DeviceInfo {
                id: DeviceId(format!(
                    "hid:0b05:{pid:04x}:{}",
                    serial.as_deref().unwrap_or("noserial")
                )),
                name: "ASUS Aura motherboard RGB/ARGB".into(),
                vendor: "ASUS".into(),
                kind: DeviceKind::LightingController,
                usb_vid: Some(0x0b05),
                usb_pid: Some(pid),
                serial,
                firmware: None,
            },
            layout: LedLayout::strip(0),
            argb_leds,
            names: Default::default(),
            zones: Vec::new(),
            owned: Vec::new(),
            status: DeviceStatus::Disconnected,
        }
    }
    /// User names for the ARGB headers (shown on the layout canvas).
    pub fn with_names(mut self, names: [Option<String>; 4]) -> Self {
        self.names = names;
        self
    }
    fn write(&mut self, packet: &[u8; 65]) -> Result<(), DeviceError> {
        let count = self.transport.write(packet)?;
        if count != 65 {
            return Err(invalid("short Aura output report"));
        }
        Ok(())
    }
    fn acquire(&mut self) -> Result<(), DeviceError> {
        if self.transport.output_report_len() != 65
            || !(64..=65).contains(&self.transport.input_report_len())
        {
            return Err(invalid("unsupported Aura HID report sizes"));
        }
        if self.argb_leds.iter().any(|count| *count > 512) {
            return Err(invalid("ARGB LED counts must be 0..512"));
        }
        // Bounded stale-input drain; never select a mode while discovering headers.
        let mut reply = [0; 65];
        for _ in 0..4 {
            if self.transport.read_timeout(&mut reply, 0)? == 0 {
                break;
            }
        }
        let mut query = [0; 65];
        query[..2].copy_from_slice(&[0xec, 0xb0]);
        self.write(&query)?;
        let count = self.transport.read_timeout(&mut reply, 500)?;
        if count > reply.len() {
            return Err(invalid("oversized Aura input report"));
        }
        let capabilities = Capabilities::decode(&reply[..count])?;
        if self.argb_leds[usize::from(capabilities.argb_headers)..]
            .iter()
            .any(|count| *count != 0)
        {
            return Err(invalid("LED count supplied for an absent ARGB header"));
        }
        let mut zones = Vec::new();
        if capabilities.fixed_leds > 0 {
            zones.push(Zone {
                effect: 0,
                direct: 4,
                count: usize::from(capabilities.fixed_leds),
            });
        }
        for header in 0..capabilities.argb_headers {
            let count = usize::from(self.argb_leds[usize::from(header)]);
            if count > 0 {
                zones.push(Zone {
                    effect: header + u8::from(capabilities.fixed_leds > 0),
                    direct: header,
                    count,
                });
            }
        }
        if zones.is_empty() {
            return Err(invalid(
                "no RGB zones selected; configure ARGB LED counts explicitly",
            ));
        }
        let total = zones.iter().map(|zone| zone.count).sum::<usize>();
        let mut layout = LedLayout::strip(total as u16);
        let mut index = 0;
        for zone in &zones {
            for led in 0..zone.count {
                layout.leds[index].name = if zone.direct == 4 {
                    let onboard = usize::from(capabilities.fixed_leds - capabilities.rgb_headers);
                    if led < onboard {
                        format!("Onboard RGB {}", led + 1)
                    } else {
                        format!("12V RGB header {}", led - onboard + 1)
                    }
                } else {
                    match &self.names[usize::from(zone.direct)] {
                        Some(name) => format!("{name} LED {}", led + 1),
                        None => format!("5V ARGB header {} LED {}", zone.direct + 1, led + 1),
                    }
                };
                index += 1;
            }
        }
        self.zones = zones;
        self.layout = layout;
        if self.zones.iter().any(|zone| zone.direct != 4) {
            // Controller-wide Gen1 selection: documented opt-in, not LED detection.
            let mut gen1 = [0; 65];
            gen1[..5].copy_from_slice(&[0xec, 0x52, 0x53, 0, 1]);
            self.write(&gen1)?;
        }
        for zone in self.zones.clone() {
            self.owned.push(zone.effect);
            self.write(&effect_packet(zone.effect, 0xff))?;
        }
        Ok(())
    }
    fn release(&mut self) -> Result<(), DeviceError> {
        // Known volatile firmware spectrum effect; prior effect cannot be read.
        while let Some(&effect) = self.owned.last() {
            self.write(&effect_packet(effect, 4))?;
            self.owned.pop();
        }
        Ok(())
    }
}

impl<T: HidTransport> RgbDevice for AuraDevice<T> {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }
    fn layout(&self) -> &LedLayout {
        &self.layout
    }
    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }
    fn connect(&mut self) -> Result<(), DeviceError> {
        if self.status == DeviceStatus::Connected {
            return Ok(());
        }
        let result = self.release().and_then(|()| self.acquire());
        self.status = match &result {
            Ok(()) => DeviceStatus::Connected,
            Err(error) => DeviceStatus::Faulted(error.to_string()),
        };
        result
    }
    fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        if self.status != DeviceStatus::Connected {
            return Err(DeviceError::NotConnected);
        }
        check_frame(&self.layout, frame)?;
        let mut offset = 0;
        for zone in self.zones.clone() {
            for packet in direct_packets(zone.direct, &frame.colors()[offset..offset + zone.count])?
            {
                if let Err(error) = self.write(&packet) {
                    self.status = DeviceStatus::Faulted(error.to_string());
                    return Err(error);
                }
            }
            offset += zone.count;
        }
        Ok(())
    }
    fn disconnect(&mut self, final_color: Option<Rgb>) -> Result<(), DeviceError> {
        if let Some(color) = final_color {
            if self.status == DeviceStatus::Connected {
                self.push_frame(&Frame::solid(self.layout.len(), color))?;
            }
        }
        let result = self.release();
        self.status = match &result {
            Ok(()) => DeviceStatus::Disconnected,
            Err(error) => DeviceStatus::Faulted(error.to_string()),
        };
        result
    }
}
impl<T: HidTransport> Drop for AuraDevice<T> {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_device_api::mock::MockTransport;
    struct FailRestore {
        inner: MockTransport,
        fail: bool,
    }
    impl HidTransport for FailRestore {
        fn write(&mut self, report: &[u8]) -> Result<usize, freergb_device_api::TransportError> {
            if self.fail && report[1] == 0x35 && report[5] == 4 {
                return Err(freergb_device_api::TransportError::Io(
                    "fixture restore failure".into(),
                ));
            }
            self.inner.write(report)
        }
        fn read_timeout(
            &mut self,
            buffer: &mut [u8],
            timeout: i32,
        ) -> Result<usize, freergb_device_api::TransportError> {
            self.inner.read_timeout(buffer, timeout)
        }
        fn output_report_len(&self) -> usize {
            65
        }
        fn input_report_len(&self) -> usize {
            65
        }
    }
    #[test]
    fn restoration_failure_keeps_ownership_and_prevents_reacquisition() {
        let transport = FailRestore {
            inner: MockTransport::new(65, 65, |packet| {
                if packet[1] == 0xb0 {
                    vec![reply()]
                } else {
                    Vec::new()
                }
            }),
            fail: false,
        };
        let mut device = AuraDevice::new(transport, 0x19af, None, [0; 4]);
        device.connect().unwrap();
        device.transport.fail = true;
        assert!(device.disconnect(None).is_err());
        let queries = device
            .transport
            .inner
            .written()
            .iter()
            .filter(|p| p[1] == 0xb0)
            .count();
        assert!(device.connect().is_err());
        assert_eq!(
            device
                .transport
                .inner
                .written()
                .iter()
                .filter(|p| p[1] == 0xb0)
                .count(),
            queries
        );
        assert_eq!(device.owned, vec![0]);
        device.transport.fail = false;
        device.disconnect(None).unwrap();
        assert!(device.owned.is_empty());
    }
    fn reply() -> Vec<u8> {
        let mut reply = vec![0; 65];
        reply[0] = 0xec;
        reply[1] = 0x30;
        reply[6] = 2;
        reply[31] = 4;
        reply[33] = 1;
        reply
    }
    #[test]
    fn discovery_rejects_truncation_and_impossible_headers_before_mode_writes() {
        for bytes in [vec![], reply()[..63].to_vec(), {
            let mut value = reply();
            value[33] = 5;
            value
        }] {
            assert!(Capabilities::decode(&bytes).is_err());
        }
        let transport = MockTransport::new(65, 65, |_| vec![vec![0xec, 0x30]]);
        let mut device = AuraDevice::new(transport, 0x19af, None, [0; 4]);
        assert!(device.connect().is_err());
        assert_eq!(device.transport.written().len(), 1);
        assert_eq!(&device.transport.written()[0][..2], &[0xec, 0xb0]);
    }
    #[test]
    fn packets_apply_only_last_chunk_and_handle_ninth_offset_bit() {
        let colors = vec![Rgb::new(1, 2, 3); 512];
        let packets = direct_packets(2, &colors).unwrap();
        assert_eq!(packets.len(), 26);
        for (index, packet) in packets.iter().enumerate() {
            assert_eq!(packet[2] & 0x80, if index == 25 { 0x80 } else { 0 });
            assert_eq!(packet[2] & 0x10, if index * 20 >= 256 { 0x10 } else { 0 });
            assert_eq!(packet[3], (index * 20) as u8);
            assert_eq!(&packet[5..8], &[1, 2, 3]);
        }
        assert!(direct_packets(5, &colors).is_err());
        assert!(direct_packets(0, &[]).is_err());
    }
    #[test]
    fn selected_headers_have_names_and_volatile_fallback_without_flash_or_fan_packets() {
        let transport = MockTransport::new(65, 65, |packet| {
            if packet[1] == 0xb0 {
                vec![reply()]
            } else {
                Vec::new()
            }
        });
        let mut device = AuraDevice::new(transport, 0x19af, None, [21, 0, 0, 0]);
        device.connect().unwrap();
        assert_eq!(device.layout.len(), 25);
        assert_eq!(device.layout.leds[3].name, "12V RGB header 1");
        assert_eq!(device.layout.leds[4].name, "5V ARGB header 1 LED 1");
        assert!(device.push_frame(&Frame::solid(1, Rgb::RED)).is_err());
        device.push_frame(&Frame::solid(25, Rgb::RED)).unwrap();
        device.disconnect(None).unwrap();
        assert!(device
            .transport
            .written()
            .iter()
            .all(|p| [0xb0, 0x52, 0x35, 0x40].contains(&p[1])));
        let last = device.transport.written().last().unwrap();
        assert_eq!(&last[..6], &[0xec, 0x35, 0, 0, 0, 4]);
        assert!(device.owned.is_empty());
    }
}
