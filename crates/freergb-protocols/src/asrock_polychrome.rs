//! Independently implemented ASRock Polychrome USB motherboard RGB/ARGB protocol
//! (`26ce:01a2`, `26ce:01a6`). Wire facts and limits: docs/MAINBOARD_RGB.md.
//! Volatile only: the commit/save command (`0x12`) is never sent; the LED-count
//! table, RGB-swap byte and every zone's effect read at start are written back.
use freergb_core::{Frame, LedLayout, Rgb};
use freergb_device_api::{
    check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, HidTransport,
    RgbDevice,
};

const SET_ZONE: u8 = 0x10;
const READ_ZONE: u8 = 0x11;
const READ_HEADER: u8 = 0x14;
const WRITE_HEADER: u8 = 0x15;
const LED_COUNT_CFG: u8 = 0x02;
const RGSWAP_CFG: u8 = 0x03;
const AVAILABLE_CFG: u8 = 0x01;
const UNAVAILABLE: u8 = 0x1e;
const PER_LED_FIRST: u8 = 0xe3;
const PER_LED_NEXT: u8 = 0xe4;
pub const ZONES: usize = 8;
/// Addressable headers ARGB1, ARGB2 and ARGB3/audio channel (zone indices).
pub const ARGB_ZONES: [usize; 3] = [2, 3, 7];
/// Per addressable header (firmware/reference maximum).
pub const MAX_LEDS: u16 = 100;
const ZONE_NAMES: [&str; ZONES] = [
    "12V RGB header 1",
    "12V RGB header 2",
    "ARGB header 1",
    "ARGB header 2",
    "PCH",
    "IO cover",
    "PCB",
    "ARGB header 3",
];

fn invalid(message: impl Into<String>) -> DeviceError {
    DeviceError::Protocol(message.into())
}

fn command(bytes: &[(usize, u8)]) -> [u8; 65] {
    let mut packet = [0u8; 65];
    for (at, value) in bytes {
        packet[*at] = *value;
    }
    packet
}

/// Per-LED stream for the packed colour bytes (all present zones in order),
/// padded like the vendor tool: first report 18 LEDs from byte 9, then 19 from byte 5.
pub fn per_led_packets(bytes: &[u8]) -> Vec<[u8; 65]> {
    let mut packets = Vec::new();
    let mut rest = bytes;
    let mut first = true;
    while first || !rest.is_empty() {
        let (start, capacity, mode) = if first {
            (9, 54, PER_LED_FIRST)
        } else {
            (5, 57, PER_LED_NEXT)
        };
        let mut packet = command(&[(1, SET_ZONE), (3, 0xff), (4, mode), (0x40, 0x65)]);
        if first {
            packet[7] = 0xff;
        }
        let take = capacity.min(rest.len());
        packet[start..start + take].copy_from_slice(&rest[..take]);
        rest = &rest[take..];
        packets.push(packet);
        first = false;
    }
    packets
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Original {
    counts: [u8; ZONES],
    rgswap: u8,
    /// Per present zone: (zone, mode, r, g, b, speed) as read.
    effects: Vec<[u8; 6]>,
}

pub struct PolychromeDevice<T: HidTransport> {
    transport: T,
    info: DeviceInfo,
    layout: LedLayout,
    /// Requested LEDs for ARGB1/ARGB2/ARGB3; 0 keeps the controller's own count.
    leds: [u16; 3],
    names: [Option<String>; 3],
    /// Present zones in order with their LED counts and RGB (vs GRB) byte order.
    zones: Vec<(usize, usize, bool)>,
    /// Padded stream length in LEDs (sum of zone maxima + 8, as the vendor tool).
    stream: usize,
    original: Option<Original>,
    status: DeviceStatus,
}

impl<T: HidTransport> PolychromeDevice<T> {
    pub fn new(transport: T, pid: u16, serial: Option<String>, leds: [u16; 3]) -> Self {
        Self {
            transport,
            info: DeviceInfo {
                id: DeviceId(format!(
                    "hid:26ce:{pid:04x}:{}",
                    serial.as_deref().unwrap_or("noserial")
                )),
                name: "ASRock Polychrome motherboard RGB/ARGB".into(),
                vendor: "ASRock".into(),
                kind: DeviceKind::LightingController,
                usb_vid: Some(0x26ce),
                usb_pid: Some(pid),
                serial,
                firmware: None,
            },
            layout: LedLayout::strip(0),
            leds,
            names: Default::default(),
            zones: Vec::new(),
            stream: 0,
            original: None,
            status: DeviceStatus::Disconnected,
        }
    }
    /// User names for ARGB1/ARGB2/ARGB3 (shown on the layout canvas).
    pub fn with_names(mut self, names: [Option<String>; 3]) -> Self {
        self.names = names;
        self
    }
    /// Writes one report and consumes the controller's acknowledgement.
    fn exchange(&mut self, packet: &[u8; 65]) -> Result<[u8; 64], DeviceError> {
        if self.transport.write(packet)? != 65 {
            return Err(invalid("short Polychrome output report"));
        }
        let mut reply = [0u8; 64];
        let count = self.transport.read_timeout(&mut reply, 500)?;
        if count < 8 {
            return Err(invalid("no Polychrome reply"));
        }
        Ok(reply)
    }
    fn read_header(&mut self, cfg: u8) -> Result<[u8; 8], DeviceError> {
        let reply = self.exchange(&command(&[(1, READ_HEADER), (3, cfg)]))?;
        if reply[1] != READ_HEADER || reply[3] != cfg {
            return Err(invalid("invalid Polychrome header reply"));
        }
        let mut out = [0u8; 8];
        out.copy_from_slice(&reply[4..12]);
        Ok(out)
    }
    fn write_header(&mut self, cfg: u8, data: &[u8]) -> Result<(), DeviceError> {
        let mut packet = command(&[(1, WRITE_HEADER), (3, cfg)]);
        packet[4..4 + data.len()].copy_from_slice(data);
        self.exchange(&packet).map(|_| ())
    }
    fn acquire(&mut self) -> Result<(), DeviceError> {
        if self.transport.output_report_len() != 65 || self.transport.input_report_len() < 64 {
            return Err(invalid("unsupported Polychrome HID report sizes"));
        }
        if self.leds.iter().any(|n| *n > MAX_LEDS) {
            return Err(invalid("ARGB LED counts must be 0..100"));
        }
        if self.leds.contains(&u16::from(UNAVAILABLE)) {
            // 0x1e marks an absent zone in the controller's table.
            return Err(invalid(
                "30 LEDs cannot be set on Polychrome headers; use 29 or 31",
            ));
        }
        self.transport.drain()?;
        // Read everything first; any implausible answer stops before a write.
        let counts = self.read_header(LED_COUNT_CFG)?;
        let rgswap = self.read_header(RGSWAP_CFG)?[0];
        let available = self.read_header(AVAILABLE_CFG)?[0];
        let present: Vec<usize> = (0..ZONES)
            .filter(|z| counts[*z] != UNAVAILABLE && available >> z & 1 == 1)
            .collect();
        if present.is_empty() || present.iter().any(|z| counts[*z] > MAX_LEDS as u8) {
            return Err(invalid("implausible Polychrome zone table"));
        }
        let mut effects = Vec::new();
        for zone in &present {
            let reply = self.exchange(&command(&[(1, READ_ZONE), (3, *zone as u8)]))?;
            if reply[1] != READ_ZONE || usize::from(reply[3]) != *zone {
                return Err(invalid("invalid Polychrome zone reply"));
            }
            effects.push([
                *zone as u8,
                reply[4],
                reply[5],
                reply[6],
                reply[7],
                reply[8],
            ]);
        }
        for (slot, zone) in ARGB_ZONES.iter().enumerate() {
            if self.leds[slot] > 0 && !present.contains(zone) {
                return Err(invalid("LED count supplied for an absent ARGB header"));
            }
        }
        let mut wanted = counts;
        for (slot, zone) in ARGB_ZONES.iter().enumerate() {
            if self.leds[slot] > 0 {
                wanted[*zone] = self.leds[slot] as u8;
            }
        }
        self.original = Some(Original {
            counts,
            rgswap,
            effects,
        });
        if wanted != counts {
            self.write_header(LED_COUNT_CFG, &wanted)?;
        }
        // Hardware RGB swap flickers in per-LED mode; order bytes per zone instead.
        self.write_header(RGSWAP_CFG, &[0])?;
        self.zones = present
            .iter()
            .map(|z| (*z, usize::from(wanted[*z]), rgswap >> z & 1 == 1))
            .collect();
        self.stream = present
            .iter()
            .map(|z| {
                if ARGB_ZONES.contains(z) {
                    usize::from(MAX_LEDS)
                } else {
                    usize::from(wanted[*z])
                }
            })
            .sum::<usize>()
            + 8;
        let total: usize = self.zones.iter().map(|z| z.1).sum();
        let mut layout = LedLayout::strip(total as u16);
        let mut index = 0;
        for (zone, count, _) in &self.zones {
            let label = ARGB_ZONES
                .iter()
                .position(|a| a == zone)
                .and_then(|slot| self.names[slot].clone())
                .unwrap_or_else(|| ZONE_NAMES[*zone].to_owned());
            for led in 0..*count {
                layout.leds[index].name = if *count == 1 {
                    label.clone()
                } else {
                    format!("{label} LED {}", led + 1)
                };
                index += 1;
            }
        }
        self.layout = layout;
        Ok(())
    }
    fn release(&mut self) -> Result<(), DeviceError> {
        let Some(original) = self.original.clone() else {
            return Ok(());
        };
        self.write_header(LED_COUNT_CFG, &original.counts)?;
        self.write_header(RGSWAP_CFG, &[original.rgswap])?;
        for [zone, mode, r, g, b, speed] in original.effects.iter().copied() {
            // Per-LED modes have no stored effect to return to: use the firmware spectrum cycle.
            let mode = if mode >= 0x0f { 0x04 } else { mode };
            let packet = command(&[
                (1, SET_ZONE),
                (3, zone),
                (4, mode),
                (5, r),
                (6, g),
                (7, b),
                (8, speed),
                (9, 0xff),
            ]);
            self.exchange(&packet)?;
        }
        self.original = None;
        Ok(())
    }
}

impl<T: HidTransport> RgbDevice for PolychromeDevice<T> {
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
        let mut bytes = Vec::with_capacity(self.stream * 3);
        for (color, rgb_order) in frame.colors().iter().zip(
            self.zones
                .iter()
                .flat_map(|(_, count, rgb)| std::iter::repeat_n(*rgb, *count)),
        ) {
            let [r, g, b] = color.to_array();
            bytes.extend_from_slice(&if rgb_order { [r, g, b] } else { [g, r, b] });
        }
        bytes.resize(self.stream * 3, 0);
        for packet in per_led_packets(&bytes) {
            if let Err(error) = self.exchange(&packet) {
                self.status = DeviceStatus::Faulted(error.to_string());
                return Err(error);
            }
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
impl<T: HidTransport> Drop for PolychromeDevice<T> {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_device_api::mock::MockTransport;

    /// Controller with RGB1 (1 LED, RGB order), ARGB1 (24) and PCB (6 LEDs).
    /// (30 = 0x1e would read as "zone unavailable", as on the real controller.)
    fn controller(bad_zone_reply: bool) -> MockTransport {
        MockTransport::new(65, 64, move |p| {
            let mut reply = vec![0u8; 64];
            reply[..4].copy_from_slice(&[p[0], p[1], p[2], p[3]]);
            match (p[1], p[3]) {
                (READ_HEADER, LED_COUNT_CFG) => {
                    reply[4..12].copy_from_slice(&[1, 0x1e, 24, 0x1e, 0x1e, 0x1e, 6, 0x1e]);
                }
                (READ_HEADER, RGSWAP_CFG) => reply[4] = 0b0000_0001,
                (READ_HEADER, AVAILABLE_CFG) => reply[4] = 0b0100_0101,
                (READ_ZONE, zone) => {
                    reply[3] = if bad_zone_reply { 9 } else { zone };
                    reply[4..9].copy_from_slice(&[0x02, 10, 20, 30, 0xe0]);
                }
                _ => {}
            }
            vec![reply]
        })
    }
    fn writes(device: &PolychromeDevice<MockTransport>, op: u8) -> Vec<Vec<u8>> {
        device
            .transport
            .written()
            .iter()
            .filter(|p| p[1] == op)
            .cloned()
            .collect()
    }

    #[test]
    fn detection_reads_tables_resizes_headers_and_disables_swap() {
        let mut device = PolychromeDevice::new(controller(false), 0x01a2, None, [16, 0, 0])
            .with_names([Some("Front fans".into()), None, None]);
        device.connect().unwrap();
        assert_eq!(device.info().id.0, "hid:26ce:01a2:noserial");
        assert_eq!(device.layout().len(), 1 + 16 + 6);
        assert_eq!(device.layout().leds[0].name, "12V RGB header 1");
        assert_eq!(device.layout().leds[1].name, "Front fans LED 1");
        assert_eq!(device.layout().leds[17].name, "PCB LED 1");
        let headers = writes(&device, WRITE_HEADER);
        assert_eq!(
            &headers[0][3..12],
            &[LED_COUNT_CFG, 1, 0x1e, 16, 0x1e, 0x1e, 0x1e, 6, 0x1e]
        );
        assert_eq!(&headers[1][3..5], &[RGSWAP_CFG, 0]);
        // 1 + 100 (ARGB maximum) + 6 + 8 padding LEDs.
        assert_eq!(device.stream, 115);
    }

    #[test]
    fn invalid_replies_and_settings_never_write() {
        let mut bad = PolychromeDevice::new(controller(true), 0x01a2, None, [0; 3]);
        assert!(bad.connect().is_err());
        assert!(writes(&bad, WRITE_HEADER).is_empty() && writes(&bad, SET_ZONE).is_empty());
        let silent = MockTransport::new(65, 64, |_| Vec::new());
        let mut silent = PolychromeDevice::new(silent, 0x01a6, None, [0; 3]);
        assert!(silent.connect().is_err());
        assert_eq!(silent.transport.written().len(), 1);
        for leds in [[101, 0, 0], [0, 10, 0], [30, 0, 0]] {
            let mut device = PolychromeDevice::new(controller(false), 0x01a2, None, leds);
            assert!(device.connect().is_err(), "{leds:?}");
            assert!(writes(&device, WRITE_HEADER).is_empty());
        }
    }

    #[test]
    fn frames_pack_zones_in_order_with_swap_and_chunking() {
        let mut device = PolychromeDevice::new(controller(false), 0x01a2, None, [0, 0, 0]);
        device.connect().unwrap();
        assert_eq!(device.layout().len(), 1 + 24 + 6);
        let colors: Vec<Rgb> = (0..31).map(|i| Rgb::new(i, 100, 200)).collect();
        device.push_frame(&Frame::from_colors(colors)).unwrap();
        let frames: Vec<_> = writes(&device, SET_ZONE);
        // (1 + 100 + 6 + 8) * 3 = 345 bytes: 54 + 6 * 57 - 51 -> 7 reports.
        assert_eq!(frames.len(), 7);
        assert_eq!(
            &frames[0][..10],
            &[0, SET_ZONE, 0, 0xff, PER_LED_FIRST, 0, 0, 0xff, 0, 0]
        );
        assert_eq!(frames[0][0x40], 0x65);
        // RGB1 keeps RGB order (swap bit set), ARGB1 is sent G,R,B.
        assert_eq!(&frames[0][9..15], &[0, 100, 200, 100, 1, 200]);
        assert_eq!(&frames[1][..5], &[0, SET_ZONE, 0, 0xff, PER_LED_NEXT]);
        assert_eq!(per_led_packets(&[]).len(), 1);
    }

    #[test]
    fn release_restores_tables_and_effects_without_commit() {
        let mut device = PolychromeDevice::new(controller(false), 0x01a2, None, [16, 0, 0]);
        device.connect().unwrap();
        device.disconnect(Some(Rgb::BLACK)).unwrap();
        let headers = writes(&device, WRITE_HEADER);
        let n = headers.len();
        assert_eq!(
            &headers[n - 2][3..12],
            &[LED_COUNT_CFG, 1, 0x1e, 24, 0x1e, 0x1e, 0x1e, 6, 0x1e]
        );
        assert_eq!(&headers[n - 1][3..5], &[RGSWAP_CFG, 0b0000_0001]);
        let restored: Vec<_> = writes(&device, SET_ZONE)
            .into_iter()
            .filter(|p| p[3] != 0xff)
            .map(|p| (p[3], p[4], p[5], p[8]))
            .collect();
        assert_eq!(
            restored,
            vec![(0, 2, 10, 0xe0), (2, 2, 10, 0xe0), (6, 2, 10, 0xe0)]
        );
        assert!(
            device.transport.written().iter().all(|p| p[1] != 0x12),
            "never commit"
        );
        assert!(device.original.is_none());
    }
}
