//! Independently implemented ASUS Aura motherboard USB protocol (all mainboard
//! PIDs listed in docs/MAINBOARD_RGB.md).
//! Wire facts and limits: docs/MAINBOARD_RGB.md. No fan-power commands or flash commit.
use crate::mainboard_headers::{
    publish, ColorOrder, DetectedHeader, Fallback, HeaderReport, ReportCell,
};
use freergb_core::{Frame, LedLayout, Rgb};
use freergb_device_api::{
    check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, HidTransport,
    RgbDevice,
};

/// Addressable 5 V headers (direct channels 0..3).
pub const HEADERS: usize = 4;
/// Largest LED count per addressable header.
pub const MAX_LEDS: u16 = 512;
/// Firmware effect numbers used by this adapter (`ec 35 … mode`).
const MODE_OFF: u8 = 0x00;
const MODE_SPECTRUM: u8 = 0x04;
const MODE_DIRECT: u8 = 0xff;

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
        let rgb_headers = report[4 + 0x1d];
        // Some firmware (e.g. PRIME X870-P) reports 0 fixed LEDs next to its
        // 12V headers; the headers still sit on the fixed channel.
        let capabilities = Self {
            fixed_leds: report[4 + 0x1b].max(rgb_headers),
            rgb_headers,
            argb_headers: report[4 + 2],
        };
        if capabilities.fixed_leds > 16 || capabilities.argb_headers > 4 {
            return Err(invalid("unsupported Aura header/LED configuration"));
        }
        Ok(capabilities)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Zone {
    effect: u8,
    direct: u8,
    count: usize,
    /// Addressable header index (`None` for the fixed onboard/12 V channel).
    header: Option<usize>,
    order: ColorOrder,
    fallback: Fallback,
}

/// Direct colour reports (`ec 40`) for one channel, bytes in `order`.
pub fn direct_packets(
    channel: u8,
    colors: &[Rgb],
    order: ColorOrder,
) -> Result<Vec<[u8; 65]>, DeviceError> {
    if channel > 4 || colors.is_empty() || colors.len() > usize::from(MAX_LEDS) {
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
            packet[5 + index * 3..8 + index * 3].copy_from_slice(&order.bytes(*color));
        }
        packets.push(packet);
    }
    Ok(packets)
}

/// Effect report (`ec 35 channel 00 00 mode`), volatile (shutdown flag 0).
pub fn effect_packet(channel: u8, mode: u8) -> [u8; 65] {
    let mut packet = [0; 65];
    packet[..6].copy_from_slice(&[0xec, 0x35, channel, 0, 0, mode]);
    packet
}

pub struct AuraDevice<T: HidTransport> {
    transport: T,
    info: DeviceInfo,
    layout: LedLayout,
    argb_leds: [u16; HEADERS],
    names: [Option<String>; HEADERS],
    orders: [ColorOrder; HEADERS],
    fallbacks: [Fallback; HEADERS],
    report: Option<ReportCell>,
    zones: Vec<Zone>,
    /// Effect channels in direct mode, returned to their fallback on release.
    owned: Vec<Zone>,
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
            orders: [ColorOrder::Rgb; HEADERS],
            fallbacks: [Fallback::Spectrum; HEADERS],
            report: None,
            zones: Vec::new(),
            owned: Vec::new(),
            status: DeviceStatus::Disconnected,
        }
    }
    /// User names for the ARGB headers (shown on the layout canvas).
    pub fn with_names(mut self, names: [Option<String>; HEADERS]) -> Self {
        self.names = names;
        self
    }
    /// Per ARGB header: colour byte order of the strip and what it shows on release.
    pub fn with_header_options(
        mut self,
        orders: [ColorOrder; HEADERS],
        fallbacks: [Fallback; HEADERS],
    ) -> Self {
        self.orders = orders;
        self.fallbacks = fallbacks;
        self
    }
    /// Slot that receives the header table read on every connect attempt.
    pub fn with_report(mut self, cell: ReportCell) -> Self {
        self.report = Some(cell);
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
        let mut report = HeaderReport {
            fixed_leds: u16::from(capabilities.fixed_leds),
            headers: (0..HEADERS)
                .map(|header| DetectedHeader {
                    present: header < usize::from(capabilities.argb_headers),
                    max_leds: MAX_LEDS,
                    ..Default::default()
                })
                .collect(),
        };
        // The table names headers only; attached LED counts are not reported.
        publish(&self.report, &report);
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
                header: None,
                order: ColorOrder::Rgb,
                fallback: Fallback::Spectrum,
            });
        }
        for header in 0..capabilities.argb_headers {
            let index = usize::from(header);
            let count = usize::from(self.argb_leds[index]);
            if count > 0 {
                zones.push(Zone {
                    effect: header + u8::from(capabilities.fixed_leds > 0),
                    direct: header,
                    count,
                    header: Some(index),
                    order: self.orders[index],
                    fallback: self.fallbacks[index],
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
            if let Some(header) = zone.header {
                report.headers[header].start = Some(index);
                report.headers[header].count = zone.count;
            }
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
        publish(&self.report, &report);
        self.zones = zones;
        self.layout = layout;
        if self.zones.iter().any(|zone| zone.direct != 4) {
            // Controller-wide Gen1 selection: documented opt-in, not LED detection.
            let mut gen1 = [0; 65];
            gen1[..5].copy_from_slice(&[0xec, 0x52, 0x53, 0, 1]);
            self.write(&gen1)?;
        }
        for zone in self.zones.clone() {
            self.owned.push(zone.clone());
            self.write(&effect_packet(zone.effect, MODE_DIRECT))?;
        }
        Ok(())
    }
    /// Reports that return one owned effect channel to its fallback.
    fn fallback_packets(zone: &Zone) -> Result<Vec<[u8; 65]>, DeviceError> {
        Ok(match zone.fallback {
            // Known volatile firmware effect; the prior effect cannot be read.
            Fallback::Spectrum => vec![effect_packet(zone.effect, MODE_SPECTRUM)],
            Fallback::Off => vec![effect_packet(zone.effect, MODE_OFF)],
            // Last direct frame; the channel stays in direct mode (no effect write).
            Fallback::Static { color } => {
                direct_packets(zone.direct, &vec![color; zone.count], zone.order)?
            }
        })
    }
    fn release(&mut self) -> Result<(), DeviceError> {
        while let Some(zone) = self.owned.last() {
            for packet in Self::fallback_packets(zone)? {
                self.write(&packet)?;
            }
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
            let colors = &frame.colors()[offset..offset + zone.count];
            for packet in direct_packets(zone.direct, colors, zone.order)? {
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
        assert_eq!(
            device.owned.iter().map(|z| z.effect).collect::<Vec<_>>(),
            vec![0]
        );
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
            value[33] = 17;
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
    fn rgb_headers_without_fixed_leds_use_the_fixed_channel() {
        // PRIME X870-P WIFI (19af) reply: 0 fixed LEDs, 3 RGB headers, 3 ARGB.
        let mut value = reply();
        value[6] = 3;
        value[31] = 0;
        value[33] = 3;
        assert_eq!(
            Capabilities::decode(&value).unwrap(),
            Capabilities {
                fixed_leds: 3,
                rgb_headers: 3,
                argb_headers: 3
            }
        );
    }
    #[test]
    fn packets_apply_only_last_chunk_and_handle_ninth_offset_bit() {
        let colors = vec![Rgb::new(1, 2, 3); 512];
        let packets = direct_packets(2, &colors, ColorOrder::Rgb).unwrap();
        assert_eq!(packets.len(), 26);
        for (index, packet) in packets.iter().enumerate() {
            assert_eq!(packet[2] & 0x80, if index == 25 { 0x80 } else { 0 });
            assert_eq!(packet[2] & 0x10, if index * 20 >= 256 { 0x10 } else { 0 });
            assert_eq!(packet[3], (index * 20) as u8);
            assert_eq!(&packet[5..8], &[1, 2, 3]);
        }
        assert!(direct_packets(5, &colors, ColorOrder::Rgb).is_err());
        assert!(direct_packets(0, &[], ColorOrder::Rgb).is_err());
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

    fn grb_device(fallback: Fallback) -> AuraDevice<MockTransport> {
        let transport = MockTransport::new(65, 65, |packet| {
            if packet[1] == 0xb0 {
                vec![reply()]
            } else {
                Vec::new()
            }
        });
        AuraDevice::new(transport, 0x19af, None, [0, 2, 0, 0]).with_header_options(
            [
                ColorOrder::Rgb,
                ColorOrder::Grb,
                ColorOrder::Rgb,
                ColorOrder::Rgb,
            ],
            [
                Fallback::Spectrum,
                fallback,
                Fallback::Spectrum,
                Fallback::Spectrum,
            ],
        )
    }

    #[test]
    fn header_color_order_reorders_only_that_headers_bytes() {
        let mut device = grb_device(Fallback::Off);
        device.connect().unwrap();
        device
            .push_frame(&Frame::solid(6, Rgb::new(1, 2, 3)))
            .unwrap();
        let direct: Vec<_> = device
            .transport
            .written()
            .iter()
            .filter(|p| p[1] == 0x40)
            .collect();
        // Fixed channel stays rgb; header 2 (direct channel 1) is sent g, r, b.
        assert_eq!(&direct[0][5..8], &[1, 2, 3]);
        assert_eq!(&direct[1][5..11], &[2, 1, 3, 2, 1, 3]);
    }

    #[test]
    fn header_fallback_off_and_static_are_byte_exact() {
        let mut device = grb_device(Fallback::Off);
        device.connect().unwrap();
        device.disconnect(None).unwrap();
        let written = device.transport.written();
        let tail: Vec<_> = written[written.len() - 2..]
            .iter()
            .map(|p| p[..6].to_vec())
            .collect();
        // Release runs in reverse zone order: header 2 (effect channel 2) first.
        assert_eq!(tail[0], vec![0xec, 0x35, 2, 0, 0, 0]);
        assert_eq!(tail[1], vec![0xec, 0x35, 0, 0, 0, 4]);

        let mut device = grb_device(Fallback::Static {
            color: Rgb::new(9, 8, 7),
        });
        device.connect().unwrap();
        device.disconnect(None).unwrap();
        let written = device.transport.written();
        let last_direct = written.iter().rev().find(|p| p[1] == 0x40).unwrap();
        assert_eq!(&last_direct[5..11], &[8, 9, 7, 8, 9, 7]);
        assert!(device.owned.is_empty());
    }
}
