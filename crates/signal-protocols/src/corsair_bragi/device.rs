//! Bragi device state machine on top of a [`HidTransport`].

use super::packet::{
    self, Command, Endpoint, Handle, Mode, Property, Response, ResponseError, TARGET_BASE,
};
use signal_core::{Frame, LedLayout, Rgb};
use signal_device_api::{
    check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, HidTransport,
    RgbDevice,
};

/// How a model expects its colour buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightingScheme {
    /// Endpoint `0x01`, `RRR..GGG..BBB`, `led_count` entries per plane.
    Planar { led_count: usize },
    /// Endpoint `0x22`, `[0x12, 0x00]` + `RGBRGB..` for `led_count` LEDs.
    Controller { led_count: usize },
}

impl LightingScheme {
    fn endpoint(self) -> Endpoint {
        match self {
            LightingScheme::Planar { .. } => Endpoint::Lighting,
            LightingScheme::Controller { .. } => Endpoint::LightingController,
        }
    }

    fn led_count(self) -> usize {
        match self {
            LightingScheme::Planar { led_count } | LightingScheme::Controller { led_count } => {
                led_count
            }
        }
    }

    /// Encodes `frame` (layout order) using `wire_map` (layout index → wire
    /// index). LEDs not present in the layout stay black.
    pub fn encode(self, frame: &Frame, wire_map: &[u16]) -> Vec<u8> {
        let n = self.led_count();
        match self {
            LightingScheme::Planar { .. } => {
                let mut buf = vec![0u8; n * 3];
                for (c, &w) in frame.colors().iter().zip(wire_map) {
                    let w = usize::from(w);
                    if w < n {
                        buf[w] = c.r;
                        buf[w + n] = c.g;
                        buf[w + 2 * n] = c.b;
                    }
                }
                buf
            }
            LightingScheme::Controller { .. } => {
                let mut buf = Vec::with_capacity(2 + n * 3);
                buf.extend_from_slice(&packet::LIGHTING_CONTROLLER_PREFIX);
                buf.resize(2 + n * 3, 0);
                for (c, &w) in frame.colors().iter().zip(wire_map) {
                    let w = usize::from(w);
                    if w < n {
                        buf[2 + w * 3..2 + w * 3 + 3].copy_from_slice(&c.to_array());
                    }
                }
                buf
            }
        }
    }
}

/// Static description of a supported Bragi model.
#[derive(Debug, Clone)]
pub struct BragiModel {
    pub name: &'static str,
    pub pids: &'static [u16],
    pub kind: DeviceKind,
    pub layout: LedLayout,
    pub wire_map: Vec<u16>,
    /// Number of LED slots on the wire (max wire index + 1).
    pub wire_led_count: usize,
    /// Continuation packets need the `0x08` target bit and are acknowledged.
    pub stream_ack: bool,
}

impl BragiModel {
    pub fn for_pid(pid: u16) -> Option<Self> {
        match pid {
            0x1BFD | 0x1BFF => {
                let wire_map = super::k70_core::wire_map();
                let wire_led_count = usize::from(*wire_map.iter().max().unwrap_or(&0)) + 1;
                Some(Self {
                    name: "CORSAIR K70 CORE RGB",
                    pids: &[0x1BFD, 0x1BFF],
                    kind: DeviceKind::Keyboard,
                    layout: super::k70_core::layout(),
                    wire_map,
                    wire_led_count,
                    stream_ack: true,
                })
            }
            _ => None,
        }
    }
}

/// Read-only identification data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub vid: u32,
    pub pid: u32,
    pub firmware: String,
    pub mode: u32,
    pub layout: Option<u32>,
    pub polling_rate: Option<u32>,
}

pub struct BragiDevice<T: HidTransport> {
    transport: T,
    model: BragiModel,
    info: DeviceInfo,
    target: u8,
    scheme: Option<LightingScheme>,
    status: DeviceStatus,
    timeout_ms: i32,
}

const MAX_READS_PER_TRANSACTION: usize = 8;

impl<T: HidTransport> BragiDevice<T> {
    pub fn new(transport: T, model: BragiModel, serial: Option<String>) -> Self {
        let pid = model.pids[0];
        let info = DeviceInfo {
            id: DeviceId(format!(
                "hid:1b1c:{pid:04x}:{}",
                serial.as_deref().unwrap_or("noserial")
            )),
            name: model.name.to_owned(),
            vendor: "Corsair".to_owned(),
            kind: model.kind,
            usb_vid: Some(0x1B1C),
            usb_pid: Some(pid),
            serial,
            firmware: None,
        };
        Self {
            transport,
            model,
            info,
            target: TARGET_BASE,
            scheme: None,
            status: DeviceStatus::Disconnected,
            timeout_ms: 500,
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn scheme(&self) -> Option<LightingScheme> {
        self.scheme
    }

    /// Writes `pkt` and returns the first input report that echoes `cmd`.
    fn transact(&mut self, pkt: &[u8], cmd: Command) -> Result<Vec<u8>, DeviceError> {
        self.transport.drain()?;
        self.transport.write(pkt)?;
        let mut buf = vec![0u8; self.transport.input_report_len().max(8)];
        for _ in 0..MAX_READS_PER_TRANSACTION {
            let n = self.transport.read_timeout(&mut buf, self.timeout_ms)?;
            if n == 0 {
                return Err(signal_device_api::TransportError::Timeout.into());
            }
            match Response::parse(&buf[..n], cmd) {
                Ok(_) => return Ok(buf[..n].to_vec()),
                // Unsolicited notification (e.g. key events); keep reading.
                Err(ResponseError::WrongCommand { .. }) => continue,
                Err(e) => return Err(DeviceError::Protocol(e.to_string())),
            }
        }
        Err(DeviceError::Protocol(format!(
            "no response echoing command {:#04x}",
            cmd as u8
        )))
    }

    fn expect_ok(&mut self, pkt: &[u8], cmd: Command) -> Result<Vec<u8>, DeviceError> {
        let raw = self.transact(pkt, cmd)?;
        Response::parse(&raw, cmd)
            .and_then(Response::ok)
            .map_err(|e| DeviceError::Protocol(format!("{cmd:?}: {e}")))?;
        Ok(raw)
    }

    pub fn ping(&mut self) -> Result<(), DeviceError> {
        self.expect_ok(&packet::ping(self.target), Command::Ping)
            .map(|_| ())
    }

    pub fn get_property(&mut self, prop: Property) -> Result<u32, DeviceError> {
        let raw = self.expect_ok(
            &packet::get_property(self.target, prop),
            Command::GetProperty,
        )?;
        let r = Response::parse(&raw, Command::GetProperty)
            .map_err(|e| DeviceError::Protocol(e.to_string()))?;
        Ok(r.property_value())
    }

    fn try_get_property(&mut self, prop: Property) -> Option<u32> {
        self.get_property(prop).ok()
    }

    fn set_mode(&mut self, mode: Mode) -> Result<(), DeviceError> {
        let pkt = packet::set_property(self.target, Property::Mode, mode as u32)
            .ok_or_else(|| DeviceError::Refused("mode write not allowlisted".into()))?;
        self.expect_ok(&pkt, Command::SetProperty).map(|_| ())
    }

    /// `true` if `handle` is open. The plugin treats error code 3 on
    /// `CheckHandle` as "closed".
    fn is_handle_open(&mut self, handle: Handle) -> Result<bool, DeviceError> {
        let raw = self.transact(
            &packet::check_handle(self.target, handle),
            Command::CheckHandle,
        )?;
        let r = Response::parse(&raw, Command::CheckHandle)
            .map_err(|e| DeviceError::Protocol(e.to_string()))?;
        Ok(r.error != 3)
    }

    fn close_handle(&mut self, handle: Handle) -> Result<(), DeviceError> {
        self.expect_ok(
            &packet::close_handle(self.target, handle),
            Command::CloseHandle,
        )
        .map(|_| ())
    }

    fn close_if_open(&mut self, handle: Handle) -> Result<(), DeviceError> {
        if self.is_handle_open(handle)? {
            self.close_handle(handle)?;
        }
        Ok(())
    }

    fn open(&mut self, handle: Handle, ep: Endpoint) -> Result<(), DeviceError> {
        self.expect_ok(
            &packet::open_endpoint(self.target, handle, ep),
            Command::OpenEndpoint,
        )
        .map(|_| ())
    }

    /// Whether `ep` can be opened, tested on the Background handle and closed
    /// again immediately (same probe the plugin performs on every start).
    fn endpoint_supported(&mut self, ep: Endpoint) -> Result<bool, DeviceError> {
        self.close_if_open(Handle::Background)?;
        let supported = self.open(Handle::Background, ep).is_ok();
        if supported {
            self.close_handle(Handle::Background)?;
        }
        Ok(supported)
    }

    /// Read-only identification. Does not change device state.
    pub fn probe(&mut self) -> Result<ProbeReport, DeviceError> {
        self.ping()?;
        let vid = self.get_property(Property::VendorId)?;
        let pid = self.get_property(Property::ProductId)?;
        let fw = self.get_property(Property::Firmware)?;
        let mode = self.get_property(Property::Mode)?;
        let firmware = format!("{}.{}.{}", fw & 0xFF, (fw >> 8) & 0xFF, (fw >> 16) & 0xFF);
        Ok(ProbeReport {
            vid,
            pid,
            firmware,
            mode,
            layout: self.try_get_property(Property::Layout),
            polling_rate: self.try_get_property(Property::PollingRate),
        })
    }

    fn send_payload(&mut self, payload: &[u8]) -> Result<(), DeviceError> {
        let stream_target = if self.model.stream_ack {
            self.target
        } else {
            self.target & !TARGET_BASE
        };
        let packets = packet::write_endpoint_packets(
            self.target,
            stream_target,
            Handle::Lighting,
            payload,
            self.transport.output_report_len(),
        );
        for (i, pkt) in packets.iter().enumerate() {
            if i == 0 {
                self.expect_ok(pkt, Command::WriteEndpoint)?;
            } else if self.model.stream_ack {
                self.expect_ok(pkt, Command::StreamEndpoint)?;
            } else {
                self.transport.write(pkt)?;
            }
        }
        Ok(())
    }

    fn write_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        let scheme = self.scheme.ok_or(DeviceError::NotConnected)?;
        let payload = scheme.encode(frame, &self.model.wire_map);
        match self.send_payload(&payload) {
            Ok(()) => Ok(()),
            Err(DeviceError::Protocol(msg)) => {
                // Handle may have been closed by another program; reopen once.
                tracing::warn!(device = %self.info.id, %msg, "lighting write failed, reopening handle");
                self.close_if_open(Handle::Lighting)?;
                self.open(Handle::Lighting, scheme.endpoint())?;
                self.send_payload(&payload)
            }
            Err(e) => Err(e),
        }
    }
}

impl<T: HidTransport> RgbDevice for BragiDevice<T> {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn connect(&mut self) -> Result<(), DeviceError> {
        if self.status == DeviceStatus::Connected {
            return Ok(());
        }
        let report = self.probe()?;
        if !self.model.pids.iter().any(|&p| u32::from(p) == report.pid) {
            return Err(DeviceError::Refused(format!(
                "device reports PID {:#06x}, model {} expects {:04x?}",
                report.pid, self.model.name, self.model.pids
            )));
        }
        self.info.firmware = Some(report.firmware);

        for h in [Handle::Lighting, Handle::Background, Handle::Auxiliary] {
            self.close_if_open(h)?;
        }
        if report.mode != Mode::Software as u32 {
            self.set_mode(Mode::Software)?;
        }

        let n = self.model.wire_led_count;
        let scheme = if self.endpoint_supported(Endpoint::LightingController)? {
            LightingScheme::Controller { led_count: n }
        } else {
            LightingScheme::Planar { led_count: n }
        };
        tracing::info!(device = %self.info.id, ?scheme, "bragi lighting scheme");
        self.open(Handle::Lighting, scheme.endpoint())?;
        self.scheme = Some(scheme);
        self.status = DeviceStatus::Connected;
        Ok(())
    }

    fn layout(&self) -> &LedLayout {
        &self.model.layout
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        if self.status != DeviceStatus::Connected {
            return Err(DeviceError::NotConnected);
        }
        check_frame(&self.model.layout, frame)?;
        self.write_frame(frame).inspect_err(|e| {
            self.status = DeviceStatus::Faulted(e.to_string());
        })
    }

    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }

    fn disconnect(&mut self, final_color: Option<Rgb>) -> Result<(), DeviceError> {
        if self.status == DeviceStatus::Disconnected {
            return Ok(());
        }
        if let Some(c) = final_color {
            let f = Frame::solid(self.model.layout.len(), c);
            let _ = self.write_frame(&f);
        }
        let _ = self.close_if_open(Handle::Lighting);
        let result = self.set_mode(Mode::Hardware);
        self.scheme = None;
        self.status = DeviceStatus::Disconnected;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_device_api::mock::MockTransport;
    use std::sync::{Arc, Mutex};

    /// Minimal simulated K70 CORE: answers the commands above, tracks mode and
    /// handle state, and supports the LightingController endpoint.
    #[derive(Default)]
    struct SimState {
        mode: u32,
        open: [Option<u8>; 3],
        lighting_bytes: usize,
        expected_len: usize,
    }

    fn sim(state: Arc<Mutex<SimState>>) -> MockTransport {
        MockTransport::new(65, 65, move |r| {
            let mut s = state.lock().unwrap();
            let (cmd, a, b) = (r[2], r[3], r[4]);
            let mut resp = vec![0x00, 0x00, cmd, 0x00];
            match cmd {
                0x12 => {}
                0x02 => {
                    let v: u32 = match a {
                        0x11 => 0x1B1C,
                        0x12 => 0x1BFD,
                        0x13 => 0x48_15_01, // 1.21.72
                        0x03 => s.mode,
                        0x41 => 2,
                        0x01 => 4,
                        _ => {
                            resp[3] = 5;
                            0
                        }
                    };
                    resp.extend_from_slice(&v.to_le_bytes()[..3]);
                }
                0x01 => {
                    assert_eq!(a, 0x03, "only mode may be written");
                    s.mode = u32::from(r[5]);
                }
                0x09 => resp[3] = if s.open[a as usize].is_some() { 0 } else { 3 },
                0x05 => s.open[b as usize] = None,
                0x0D => {
                    if s.open[a as usize].is_some() || !(b == 0x22 || b == 0x01) {
                        resp[3] = 3;
                    } else {
                        s.open[a as usize] = Some(b);
                    }
                }
                0x06 => {
                    assert_eq!(s.open[0], Some(0x22));
                    s.expected_len = u32::from_le_bytes([r[4], r[5], r[6], r[7]]) as usize;
                    s.lighting_bytes = 65 - 8;
                }
                0x07 => {
                    assert_eq!(r[1], 0x08, "K70 CORE streams with ack target");
                    s.lighting_bytes += 65 - 4;
                }
                _ => resp[3] = 3,
            }
            resp.resize(65, 0);
            vec![resp]
        })
    }

    #[test]
    fn controller_payload_encoding() {
        let model = BragiModel::for_pid(0x1BFD).unwrap();
        assert_eq!(model.wire_led_count, 123);
        let mut f = Frame::solid(model.layout.len(), Rgb::BLACK);
        let esc = model.layout.index_of("Esc").unwrap();
        f.colors_mut()[esc] = Rgb::new(1, 2, 3);
        let p = LightingScheme::Controller { led_count: 123 }.encode(&f, &model.wire_map);
        assert_eq!(p.len(), 2 + 123 * 3);
        assert_eq!(&p[..2], &[0x12, 0x00]);
        assert_eq!(&p[2 + 41 * 3..2 + 41 * 3 + 3], &[1, 2, 3]);
        assert_eq!(p.iter().map(|&b| u32::from(b)).sum::<u32>(), 0x12 + 6);
    }

    #[test]
    fn planar_payload_encoding() {
        let f = Frame::from_colors(vec![Rgb::new(10, 20, 30), Rgb::new(40, 50, 60)]);
        let p = LightingScheme::Planar { led_count: 3 }.encode(&f, &[2, 0]);
        assert_eq!(p, vec![40, 0, 10, 50, 0, 20, 60, 0, 30]);
    }

    #[test]
    fn full_lifecycle_against_simulator() {
        let state = Arc::new(Mutex::new(SimState {
            mode: 1,
            ..Default::default()
        }));
        let model = BragiModel::for_pid(0x1BFD).unwrap();
        let n = model.layout.len();
        let mut dev = BragiDevice::new(sim(state.clone()), model, Some("SIM".into()));

        let probe = dev.probe().unwrap();
        assert_eq!(probe.pid, 0x1BFD);
        assert_eq!(probe.firmware, "1.21.72");
        assert_eq!(state.lock().unwrap().mode, 1, "probe must not change mode");

        dev.connect().unwrap();
        assert_eq!(dev.status(), DeviceStatus::Connected);
        assert_eq!(
            dev.scheme(),
            Some(LightingScheme::Controller { led_count: 123 })
        );
        assert_eq!(state.lock().unwrap().mode, 2);
        assert_eq!(state.lock().unwrap().open, [Some(0x22), None, None]);

        dev.push_frame(&Frame::solid(n, Rgb::RED)).unwrap();
        {
            let s = state.lock().unwrap();
            assert_eq!(s.expected_len, 2 + 123 * 3);
            assert!(s.lighting_bytes >= s.expected_len);
        }

        assert!(matches!(
            dev.push_frame(&Frame::solid(3, Rgb::RED)),
            Err(DeviceError::FrameSize { .. })
        ));

        dev.disconnect(Some(Rgb::BLACK)).unwrap();
        assert_eq!(state.lock().unwrap().mode, 1, "hardware mode restored");
        assert_eq!(state.lock().unwrap().open, [None, None, None]);
    }

    #[test]
    fn refuses_wrong_pid() {
        let state = Arc::new(Mutex::new(SimState::default()));
        let mut model = BragiModel::for_pid(0x1BFD).unwrap();
        model.pids = &[0x1B2D];
        let mut dev = BragiDevice::new(sim(state.clone()), model, None);
        assert!(matches!(dev.connect(), Err(DeviceError::Refused(_))));
        assert_eq!(
            state.lock().unwrap().mode,
            0,
            "nothing written after refusal"
        );
    }
}
