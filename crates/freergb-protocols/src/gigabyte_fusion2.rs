//! Independently implemented Gigabyte RGB Fusion 2 USB motherboard ARGB protocol.
//! Wire facts: OpenRGB 029758e2260208c6de06eefc7106cd5c462c2310,
//! Controllers/GigabyteRGBFusion2USBController/{GigabyteRGBFusion2USBController.cpp,
//! GigabyteRGBFusion2USBController.h,GigabyteFusion2USB_Devices.h}.
//! Variant channels: 5711 six (four headers, two onboard), 8950 two, a100 one.
//! IT5711 accepts at most 256 LEDs/channel; other variants accept 1024.
//! Extended 5711 calibration is read with CC61 at offsets 4/8/12/16.
//! General wire facts: docs/MAINBOARD_RGB.md.
//! Volatile only: no flash save (`0x5E`), no fan or power commands.
use freergb_core::{Frame, LedLayout, Rgb};
use freergb_device_api::{
    check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, HidTransport,
    RgbDevice,
};

/// Feature report ID and length (report ID included).
const REPORT_ID: u8 = 0xcc;
const REPORT_LEN: usize = 64;
/// Two addressable 5 V headers (D_LED1/D_LED2) are driven.
pub const HEADERS: usize = 2;
/// Firmware maximum per header (largest strip-length class).
pub const MAX_LEDS: u16 = 1024;
const LEDS_PER_PACKET: usize = 19;
const CHANNELS: usize = 6;
const DIRECT_COMMANDS: [u8; CHANNELS] = [0x58, 0x59, 0x62, 0x63, 0x64, 0x65];
const MODE_BITS: [u8; CHANNELS] = [1, 2, 8, 16, 32, 64];

/// Number of addressable channels, including IT5711 onboard channels 5/6.
pub fn header_count(vid: u16, pid: u16) -> Option<usize> {
    match (vid, pid) {
        (0x048d, 0x5711) => Some(6),
        (0x048d, 0x5702 | 0x8297 | 0x8950) => Some(2),
        (0x0414, 0xa100) => Some(1),
        _ => None,
    }
}

fn decode_order(report: &[u8], at: usize) -> Result<[usize; 3], DeviceError> {
    let bytes = report
        .get(at..at + 4)
        .ok_or_else(|| invalid("short Fusion 2 calibration"))?;
    let value = [bytes[2], bytes[1], bytes[0]].map(usize::from);
    let mut seen = [false; 3];
    for index in value {
        if index > 2 || std::mem::replace(&mut seen[index], true) {
            return Err(invalid("invalid Fusion 2 colour calibration"));
        }
    }
    if bytes[3] != 0 {
        return Err(invalid("invalid Fusion 2 calibration reserved byte"));
    }
    Ok(value)
}

fn invalid(message: impl Into<String>) -> DeviceError {
    DeviceError::Protocol(message.into())
}

/// Controller information reply (`0x60`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info {
    pub product: String,
    /// Packed strip-length classes for headers 0..5 (three bytes, two nibbles each).
    pub counts: [u8; 3],
    /// Colour byte order per driven header: index of R, G and B within a triple.
    pub order: [[usize; 3]; HEADERS],
}

impl Info {
    pub fn decode(report: &[u8]) -> Result<Self, DeviceError> {
        if report.len() != REPORT_LEN || report[0] != REPORT_ID {
            return Err(invalid("invalid Fusion 2 information reply"));
        }
        let product = report[12..40]
            .iter()
            .take_while(|b| **b != 0)
            .map(|b| {
                if b.is_ascii_graphic() || *b == b' ' {
                    *b as char
                } else {
                    '?'
                }
            })
            .collect();
        Ok(Self {
            product,
            counts: [report[8], report[9], report[10]],
            order: [decode_order(report, 44)?, decode_order(report, 48)?],
        })
    }
}

/// Strip-length class for `count` LEDs (32, 64, 256, 512, 1024).
pub fn length_class(count: u16) -> u8 {
    match count {
        0..=32 => 0,
        33..=64 => 1,
        65..=256 => 2,
        257..=512 => 3,
        _ => 4,
    }
}

fn command(bytes: &[u8]) -> [u8; REPORT_LEN] {
    let mut packet = [0; REPORT_LEN];
    packet[0] = REPORT_ID;
    packet[1..1 + bytes.len()].copy_from_slice(bytes);
    packet
}

/// Direct colour packets for one header: byte offset (LE), byte count, 19 LEDs each.
pub fn direct_packets(
    header: usize,
    order: [usize; 3],
    colors: &[Rgb],
) -> Result<Vec<[u8; REPORT_LEN]>, DeviceError> {
    if header >= CHANNELS || colors.is_empty() || colors.len() > usize::from(MAX_LEDS) {
        return Err(invalid("Fusion 2 header/LED count out of range"));
    }
    if order.iter().any(|index| *index > 2)
        || order[0] == order[1]
        || order[0] == order[2]
        || order[1] == order[2]
    {
        return Err(invalid("invalid Fusion 2 colour order"));
    }
    Ok(colors
        .chunks(LEDS_PER_PACKET)
        .enumerate()
        .map(|(part, chunk)| {
            let offset = (part * LEDS_PER_PACKET * 3) as u16;
            let mut packet = command(&[DIRECT_COMMANDS[header]]);
            packet[2..4].copy_from_slice(&offset.to_le_bytes());
            packet[4] = (chunk.len() * 3) as u8;
            for (index, color) in chunk.iter().enumerate() {
                let rgb = color.to_array();
                for channel in 0..3 {
                    packet[5 + index * 3 + order[channel]] = rgb[channel];
                }
            }
            packet
        })
        .collect())
}

pub struct Fusion2Device<T: HidTransport> {
    transport: T,
    info: DeviceInfo,
    layout: LedLayout,
    leds: Vec<u16>,
    names: Vec<Option<String>>,
    order: [[usize; 3]; CHANNELS],
    /// Built-in effects are disabled (direct mode owned) and must be returned.
    owned: bool,
    status: DeviceStatus,
}

impl<T: HidTransport> Fusion2Device<T> {
    pub fn new(
        transport: T,
        vid: u16,
        pid: u16,
        serial: Option<String>,
        leds: impl Into<Vec<u16>>,
    ) -> Self {
        Self {
            transport,
            info: DeviceInfo {
                id: DeviceId(format!(
                    "hid:{vid:04x}:{pid:04x}:{}",
                    serial.as_deref().unwrap_or("noserial")
                )),
                name: "Gigabyte RGB Fusion 2 motherboard ARGB".into(),
                vendor: "Gigabyte".into(),
                kind: DeviceKind::LightingController,
                usb_vid: Some(vid),
                usb_pid: Some(pid),
                serial,
                firmware: None,
            },
            layout: LedLayout::strip(0),
            leds: leds.into(),
            names: Default::default(),
            order: [[0, 1, 2]; CHANNELS],
            owned: false,
            status: DeviceStatus::Disconnected,
        }
    }
    /// User names for the ARGB headers (shown on the layout canvas).
    pub fn with_names(mut self, names: impl Into<Vec<Option<String>>>) -> Self {
        self.names = names.into();
        self
    }
    fn send(&mut self, packet: &[u8; REPORT_LEN]) -> Result<(), DeviceError> {
        if self.transport.send_feature(packet)? != REPORT_LEN {
            return Err(invalid("short Fusion 2 feature report"));
        }
        Ok(())
    }
    fn acquire(&mut self) -> Result<(), DeviceError> {
        let channels = header_count(
            self.info.usb_vid.unwrap_or(0),
            self.info.usb_pid.unwrap_or(0),
        )
        .ok_or_else(|| invalid("unsupported Fusion 2 controller"))?;
        if self.leds.len() > CHANNELS || self.leds.iter().skip(channels).any(|count| *count != 0) {
            return Err(invalid("Fusion 2 header unavailable on this variant"));
        }
        self.leds.resize(channels, 0);
        let max_leds = if self.info.usb_pid == Some(0x5711) {
            256
        } else {
            MAX_LEDS
        };
        if self.leds.iter().any(|count| *count > max_leds) {
            return Err(invalid("ARGB LED count exceeds the controller limit"));
        }
        if self.leds.iter().all(|count| *count == 0) {
            return Err(invalid(
                "no ARGB header selected; set the LED count per header in the device settings",
            ));
        }
        self.send(&command(&[0x60]))?;
        let mut reply = [0; REPORT_LEN];
        reply[0] = REPORT_ID;
        let count = self.transport.get_feature(&mut reply)?;
        if count != REPORT_LEN || reply[0] != REPORT_ID {
            return Err(invalid("invalid Fusion 2 information reply"));
        }
        // Unselected/absent channels may report OFF or invalid calibration.
        // Decode only driven calibration; never write the calibration table.
        let mut decoded = reply;
        for header in 0..2 {
            if self.leds.get(header).copied().unwrap_or(0) == 0 {
                decoded[44 + header * 4..48 + header * 4].copy_from_slice(&[2, 1, 0, 0]);
            }
        }
        let info = Info::decode(&decoded)?;
        self.order[..channels.min(2)].copy_from_slice(&info.order[..channels.min(2)]);
        if channels == CHANNELS {
            self.send(&command(&[0x61]))?;
            let mut calibration = [0; REPORT_LEN];
            calibration[0] = REPORT_ID;
            if self.transport.get_feature(&mut calibration)? != REPORT_LEN
                || calibration[0] != REPORT_ID
            {
                return Err(invalid("invalid Fusion 2 extended calibration reply"));
            }
            for header in 2..CHANNELS {
                if self.leds[header] > 0 {
                    self.order[header] = decode_order(&calibration, 4 + (header - 2) * 4)?;
                }
            }
        }
        if !info.product.is_empty() {
            self.info.firmware = Some(info.product.clone());
        }
        // Update only driven channels; preserve every unselected firmware nibble.
        let mut counts = info.counts;
        for (header, count) in self.leds.iter().enumerate() {
            if *count > 0 {
                let shift = (header % 2) * 4;
                counts[header / 2] =
                    (counts[header / 2] & !(0xf << shift)) | (length_class(*count) << shift);
            }
        }
        if counts != info.counts {
            self.send(&command(&[0x34, counts[0], counts[1], counts[2]]))?;
        }
        let total: u16 = self.leds.iter().sum();
        let mut layout = LedLayout::strip(total);
        let mut index = 0;
        for header in 0..self.leds.len() {
            let label = self
                .names
                .get(header)
                .cloned()
                .flatten()
                .unwrap_or_else(|| format!("ARGB header {}", header + 1));
            for led in 0..self.leds[header] {
                layout.leds[index].name = format!("{label} LED {}", led + 1);
                index += 1;
            }
        }
        self.layout = layout;
        let mask = (0..self.leds.len())
            .filter(|h| self.leds[*h] > 0)
            .fold(0u8, |mask, h| mask | MODE_BITS[h]);
        self.owned = true;
        self.send(&command(&[0x32, mask]))
    }
    fn release(&mut self) -> Result<(), DeviceError> {
        if self.owned {
            // Re-enable the stored firmware effect on every header (volatile).
            self.send(&command(&[0x32, 0]))?;
            self.owned = false;
        }
        Ok(())
    }
}

impl<T: HidTransport> RgbDevice for Fusion2Device<T> {
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
        for header in 0..self.leds.len() {
            let count = usize::from(self.leds[header]);
            if count == 0 {
                continue;
            }
            let packets = direct_packets(
                header,
                self.order[header],
                &frame.colors()[offset..offset + count],
            )?;
            for packet in packets {
                if let Err(error) = self.send(&packet) {
                    self.status = DeviceStatus::Faulted(error.to_string());
                    return Err(error);
                }
            }
            offset += count;
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
impl<T: HidTransport> Drop for Fusion2Device<T> {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_device_api::TransportError;

    /// Feature-report fixture: records writes, answers `0x60` with `reply`.
    #[derive(Clone)]
    struct Fixture {
        reply: Vec<u8>,
        sent: Vec<Vec<u8>>,
        fail_release: bool,
        calibration: Vec<u8>,
    }
    impl HidTransport for Fixture {
        fn write(&mut self, _: &[u8]) -> Result<usize, TransportError> {
            Err(TransportError::Io("output reports are not used".into()))
        }
        fn read_timeout(&mut self, _: &mut [u8], _: i32) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn output_report_len(&self) -> usize {
            0
        }
        fn input_report_len(&self) -> usize {
            0
        }
        fn send_feature(&mut self, report: &[u8]) -> Result<usize, TransportError> {
            if self.fail_release && report[1] == 0x32 && report[2] == 0 {
                return Err(TransportError::Io("fixture release failure".into()));
            }
            self.sent.push(report.to_vec());
            Ok(report.len())
        }
        fn get_feature(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
            let reply = if self.sent.last().is_some_and(|packet| packet[1] == 0x61) {
                &self.calibration
            } else {
                &self.reply
            };
            let n = reply.len().min(buf.len());
            buf[..n].copy_from_slice(&reply[..n]);
            Ok(n)
        }
    }
    fn reply() -> Vec<u8> {
        let mut reply = vec![0; 64];
        reply[0] = 0xcc;
        reply[8] = 0x00;
        reply[9] = 0x22;
        reply[12..20].copy_from_slice(b"IT5702-G");
        // D_LED1 GRB (R at 1, G at 0, B at 2), D_LED2 plain RGB.
        reply[44..48].copy_from_slice(&0x0001_0002u32.to_le_bytes());
        reply[48..52].copy_from_slice(&0x0000_0102u32.to_le_bytes());
        reply
    }
    fn device(reply: Vec<u8>, leds: [u16; 2]) -> Fusion2Device<Fixture> {
        let fixture = Fixture {
            reply,
            sent: Vec::new(),
            fail_release: false,
            calibration: Vec::new(),
        };
        Fusion2Device::new(fixture, 0x048d, 0x5702, None, leds)
    }

    // Expected reports are specified independently, including all zero padding.
    fn wire(prefix: &[u8]) -> Vec<u8> {
        let mut packet = vec![0; 64];
        packet[..prefix.len()].copy_from_slice(prefix);
        packet
    }

    #[test]
    fn variant_transactions_are_byte_exact() {
        for (vid, pid, leds, mask, commands) in [
            (
                0x048d,
                0x5711,
                vec![33, 1, 65, 1, 1, 1],
                0x7b,
                vec![0x58, 0x59, 0x62, 0x63, 0x64, 0x65],
            ),
            (0x048d, 0x8950, vec![1, 33], 3, vec![0x58, 0x59]),
            (0x0414, 0xa100, vec![33], 1, vec![0x58]),
        ] {
            let mut fixture = device(reply(), [1, 1]).transport.clone();
            fixture.reply[8..11].copy_from_slice(&[0x44, 0x43, 0x42]);
            if pid == 0xa100 {
                fixture.reply[48..52].fill(0);
            }
            fixture.calibration = wire(&[
                0xcc, 0, 0, 0, 0, 2, 1, 0, 1, 0, 2, 0, 0, 1, 2, 0, 1, 2, 0, 0,
            ]);
            let mut d = Fusion2Device::new(fixture, vid, pid, None, leds.clone());
            d.connect().unwrap();
            let mut expected = vec![wire(&[0xcc, 0x60])];
            if pid == 0x5711 {
                expected.push(wire(&[0xcc, 0x61]));
            }
            expected.push(wire(match pid {
                0x5711 => &[0xcc, 0x34, 0x01, 0x02, 0x00],
                0x8950 => &[0xcc, 0x34, 0x10, 0x43, 0x42],
                _ => &[0xcc, 0x34, 0x41, 0x43, 0x42],
            }));
            expected.push(wire(&[0xcc, 0x32, mask]));
            assert_eq!(d.transport.sent, expected);
            let n = leds.iter().map(|n| usize::from(*n)).sum();
            d.push_frame(&Frame::solid(n, Rgb::new(11, 22, 33)))
                .unwrap();
            let triples = [
                [22, 11, 33],
                [11, 22, 33],
                [33, 11, 22],
                [22, 33, 11],
                [33, 22, 11],
                [11, 33, 22],
            ];
            for (header, opcode) in commands.iter().enumerate() {
                let count = usize::from(leds[header]);
                for offset in (0..count).step_by(19) {
                    let take = (count - offset).min(19);
                    let bytes = (offset * 3) as u16;
                    let mut packet = wire(&[
                        0xcc,
                        *opcode,
                        bytes as u8,
                        (bytes >> 8) as u8,
                        (take * 3) as u8,
                    ]);
                    for chunk in packet[5..5 + take * 3].as_chunks_mut::<3>().0 {
                        chunk.copy_from_slice(&triples[header]);
                    }
                    expected.push(packet);
                }
            }
            d.disconnect(None).unwrap();
            expected.push(wire(&[0xcc, 0x32, 0]));
            assert_eq!(d.transport.sent, expected, "variant {pid:04x}");
        }
    }

    #[test]
    fn selected_extended_header_preserves_other_lengths_and_uses_its_own_mode_bit() {
        let mut fixture = device(reply(), [1, 0]).transport.clone();
        fixture.reply[8..11].copy_from_slice(&[0x43, 0x41, 0x32]);
        fixture.reply[44..52].fill(0);
        // Only channel 4 is selected; unused CC61 words may be OFF (zero).
        fixture.calibration = wire(&[0xcc, 0, 0, 0, 0, 0, 0, 0, 2, 1, 0, 0]);
        let mut d = Fusion2Device::new(fixture, 0x048d, 0x5711, None, [0, 0, 0, 256]);
        d.connect().unwrap();
        assert_eq!(d.layout().len(), 256);
        assert_eq!(
            d.transport.sent,
            vec![
                wire(&[0xcc, 0x60]),
                wire(&[0xcc, 0x61]),
                wire(&[0xcc, 0x34, 0x43, 0x21, 0x32]),
                wire(&[0xcc, 0x32, 0x10])
            ]
        );
    }

    #[test]
    fn variants_reject_unavailable_headers_limits_and_bad_extended_calibration() {
        for (vid, pid, counts) in [
            (0x0414, 0xa100, vec![1, 1]),
            (0x048d, 0x8950, vec![1, 0, 1]),
            (0x048d, 0x5711, vec![257]),
            (0x1234, 0x5711, vec![1]),
        ] {
            let fixture = device(reply(), [1, 0]).transport.clone();
            let mut d = Fusion2Device::new(fixture, vid, pid, None, counts);
            assert!(d.connect().is_err());
            assert!(d.transport.sent.is_empty());
        }
        for calibration in [
            vec![],
            wire(&[0]),
            wire(&[0xcc]),
            wire(&[0xcc, 0, 0, 0, 2, 1, 0, 1]),
        ] {
            let mut fixture = device(reply(), [1, 0]).transport.clone();
            fixture.calibration = calibration;
            let mut d = Fusion2Device::new(fixture, 0x048d, 0x5711, None, [1, 0, 1]);
            assert!(d.connect().is_err());
            assert_eq!(
                d.transport.sent,
                vec![wire(&[0xcc, 0x60]), wire(&[0xcc, 0x61])]
            );
            assert!(!d.owned);
        }
        assert!(direct_packets(0, [3, 0, 1], &[Rgb::RED]).is_err());
    }

    #[test]
    fn detection_reads_info_sets_lengths_and_takes_only_selected_headers() {
        let mut device = device(reply(), [20, 0]).with_names([Some("Front fans".into()), None]);
        device.connect().unwrap();
        assert_eq!(device.info().id.0, "hid:048d:5702:noserial");
        assert_eq!(device.info().firmware.as_deref(), Some("IT5702-G"));
        assert_eq!(device.layout().len(), 20);
        assert_eq!(device.layout().leds[0].name, "Front fans LED 1");
        let sent = &device.transport.sent;
        assert_eq!(&sent[0][..2], &[0xcc, 0x60]);
        // Header 1 and 2 class 0 is unchanged from the reply: no 0x34 write.
        assert_eq!(&sent[1][..3], &[0xcc, 0x32, 0x01]);
        assert_eq!(sent.len(), 2);
        let mut longer = self::device(reply(), [20, 100]);
        longer.connect().unwrap();
        assert_eq!(
            &longer.transport.sent[1][..5],
            &[0xcc, 0x34, 0x20, 0x22, 0x00]
        );
        assert_eq!(&longer.transport.sent[2][..3], &[0xcc, 0x32, 0x03]);
    }

    #[test]
    fn packets_chunk_by_19_with_byte_offsets_and_calibrated_order() {
        let colors = vec![Rgb::new(1, 2, 3); 40];
        let packets = direct_packets(0, [1, 0, 2], &colors).unwrap();
        assert_eq!(packets.len(), 3);
        assert_eq!(&packets[0][..5], &[0xcc, 0x58, 0, 0, 57]);
        assert_eq!(&packets[0][5..8], &[2, 1, 3]);
        assert_eq!(&packets[1][..5], &[0xcc, 0x58, 57, 0, 57]);
        assert_eq!(&packets[2][..5], &[0xcc, 0x58, 114, 0, 6]);
        let long = vec![Rgb::BLACK; 1024];
        let last = direct_packets(1, [0, 1, 2], &long).unwrap().pop().unwrap();
        assert_eq!(last[1], 0x59);
        assert_eq!(u16::from_le_bytes([last[2], last[3]]), 53 * 57);
        assert!(direct_packets(6, [0, 1, 2], &colors).is_err());
        assert!(direct_packets(0, [0, 1, 2], &[]).is_err());
        assert!(direct_packets(0, [0, 1, 2], &vec![Rgb::BLACK; 1025]).is_err());
        assert_eq!(
            [0, 32, 33, 64, 256, 257, 1024].map(length_class),
            [0, 0, 1, 1, 2, 3, 4]
        );
    }

    #[test]
    fn invalid_replies_and_settings_never_write_modes() {
        let mut bad_order = reply();
        bad_order[44..48].copy_from_slice(&0x0001_0001u32.to_le_bytes());
        let mut bad_id = reply();
        bad_id[0] = 0x00;
        for bytes in [Vec::new(), reply()[..40].to_vec(), bad_order, bad_id] {
            let mut device = device(bytes, [10, 0]);
            assert!(device.connect().is_err());
            assert_eq!(
                device.transport.sent.len(),
                1,
                "only the info query is sent"
            );
            assert!(!device.owned);
        }
        for leds in [[0, 0], [1025, 0]] {
            let mut device = device(reply(), leds);
            assert!(device.connect().is_err());
            assert!(device.transport.sent.is_empty());
        }
    }

    #[test]
    fn frames_and_volatile_fallback_without_flash_or_effect_writes() {
        let mut device = device(reply(), [3, 2]);
        assert!(device.push_frame(&Frame::solid(5, Rgb::RED)).is_err());
        device.connect().unwrap();
        assert!(device.push_frame(&Frame::solid(4, Rgb::RED)).is_err());
        device.push_frame(&Frame::solid(5, Rgb::RED)).unwrap();
        let frames: Vec<_> = device
            .transport
            .sent
            .iter()
            .filter(|p| matches!(p[1], 0x58 | 0x59))
            .collect();
        assert_eq!(frames.len(), 2);
        assert_eq!(&frames[0][..8], &[0xcc, 0x58, 0, 0, 9, 0, 255, 0]);
        assert_eq!(&frames[1][..8], &[0xcc, 0x59, 0, 0, 6, 255, 0, 0]);
        device.disconnect(Some(Rgb::BLACK)).unwrap();
        let last = device.transport.sent.last().unwrap();
        assert_eq!(&last[..3], &[0xcc, 0x32, 0x00]);
        assert!(device
            .transport
            .sent
            .iter()
            .all(|p| [0x60, 0x34, 0x32, 0x58, 0x59].contains(&p[1])));
    }

    #[test]
    fn failed_return_keeps_ownership_until_it_succeeds() {
        let mut device = device(reply(), [3, 0]);
        device.connect().unwrap();
        device.transport.fail_release = true;
        assert!(device.disconnect(None).is_err());
        assert!(device.owned);
        let queries = device
            .transport
            .sent
            .iter()
            .filter(|p| p[1] == 0x60)
            .count();
        assert!(
            device.connect().is_err(),
            "no reacquisition while unreturned"
        );
        assert_eq!(
            device
                .transport
                .sent
                .iter()
                .filter(|p| p[1] == 0x60)
                .count(),
            queries
        );
        device.transport.fail_release = false;
        device.disconnect(None).unwrap();
        assert!(!device.owned);
    }
}
