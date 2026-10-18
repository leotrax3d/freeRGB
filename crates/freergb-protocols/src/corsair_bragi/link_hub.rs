//! CORSAIR iCUE LINK System Hub (`1b1c:0c3f`) – read-only support.
//!
//! The hub speaks a Bragi variant (evidence: SignalRGB 2.5.80 plugin
//! `Corsair_ICUE_Link_Hub.js`, behaviour only, plus a raw child-list packet
//! that SignalRGB logged on the development machine):
//!
//! * Requests carry one extra zero byte: `[report id 0x00, 0x00, target, cmd, args..]`.
//! * The hub itself is target `0x01` (no `0x08` bit, except for lighting writes).
//! * Reports are 512 bytes (+ report ID).
//!
//! The hub also drives fans and the AIO pump. This module therefore only
//! implements reads: ping, properties, and the child-list / fan-RPM /
//! temperature endpoints. It never writes to an endpoint. The endpoints only
//! open in software mode [confirmed]; `run_with_software_window` switches to
//! software mode just for those reads and verifiably back to hardware mode, so
//! the hub keeps running its own cooling curves. `run` never changes the mode.

use super::packet::{Command, Handle, Mode, Property, Response, ResponseError};
use freergb_device_api::{HidTransport, TransportError};

pub const TARGET_HUB: u8 = 0x01;

pub const EP_FAN_RPM: u8 = 0x17;
pub const EP_TEMPERATURES: u8 = 0x21;
pub const EP_CHILD_DEVICES: u8 = 0x36;

fn req(target: u8, cmd: Command, args: &[u8]) -> Vec<u8> {
    let mut p = vec![0x00, 0x00, target, cmd as u8];
    p.extend_from_slice(args);
    p
}

pub fn ping(target: u8) -> Vec<u8> {
    req(target, Command::Ping, &[])
}

pub fn get_property(target: u8, prop: Property) -> Vec<u8> {
    req(target, Command::GetProperty, &(prop as u16).to_le_bytes())
}

pub fn open_endpoint(target: u8, handle: Handle, endpoint: u8) -> Vec<u8> {
    req(target, Command::OpenEndpoint, &[handle as u8, endpoint])
}

pub fn close_handle(target: u8, handle: Handle) -> Vec<u8> {
    req(target, Command::CloseHandle, &[0x01, handle as u8])
}

pub fn check_handle(target: u8, handle: Handle) -> Vec<u8> {
    req(target, Command::CheckHandle, &[handle as u8, 0x00])
}

/// The only write this module can build: the mode property (allowlisted in
/// `Property::is_writable`).
pub fn set_mode_request(target: u8, mode: Mode) -> Vec<u8> {
    debug_assert!(Property::Mode.is_writable());
    let mut args = (Property::Mode as u16).to_le_bytes().to_vec();
    args.extend_from_slice(&(mode as u32).to_le_bytes()[..3]);
    req(target, Command::SetProperty, &args)
}

pub fn read_endpoint(target: u8, handle: Handle) -> Vec<u8> {
    req(target, Command::ReadEndpoint, &[handle as u8])
}

/// Decodes a hub response. The command echo was observed at index 3 in
/// SignalRGB's buffers (which include the report ID); hidapi on Windows strips
/// the report ID, giving index 2. Both are accepted.
pub fn parse(raw: &[u8], cmd: Command) -> Result<Response<'_>, ResponseError> {
    Response::parse_at(raw, cmd, 2).or_else(|_| Response::parse_at(raw, cmd, 3))
}

/// One occupied slot in the child list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildDevice {
    pub slot: usize,
    pub device_type: u8,
    pub subtype: u8,
    /// ASCII identifier as reported (hardware ID + unique suffix).
    pub id: String,
}

impl ChildDevice {
    /// Model name for known types. Table from the SignalRGB plugin; types not
    /// listed there (e.g. 27) stay unknown.
    pub fn model_name(&self) -> Option<&'static str> {
        Some(match (self.device_type, self.subtype) {
            (1, _) => "QX Fan",
            (2, _) => "LX Fan",
            (6, _) => "LINK LCD",
            (15, _) => "RX Fan",
            (17, 0) => "LINK Titan 240 RX",
            (17, 1) => "LINK Titan 280 RX",
            (17, 2) => "LINK Titan 360 RX",
            (17, 3) => "LINK Titan 420 RX",
            (19, _) => "RX PWM Fan",
            _ => return None,
        })
    }

    /// LEDs according to the plugin table (RX fan 8, Titan AIO pump head 20).
    pub fn led_count(&self) -> Option<usize> {
        match self.device_type {
            15 => Some(8),
            17 => Some(20),
            2 => Some(18),
            1 => Some(34),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChildListError {
    #[error("child list payload too short ({0} bytes)")]
    Short(usize),
    #[error("slot {slot} at offset {offset} runs past the end of the packet")]
    Truncated { slot: usize, offset: usize },
}

/// Parses the child-list endpoint payload (the bytes after the error code).
///
/// Layout (confirmed against a logged packet from the development machine):
/// `[type_lo, type_hi, slot_count, slot..]`, each slot either 8 zero bytes
/// (empty) or `[0, 0, type, subtype, 0, 0, 0x05, id_len, id[id_len]]`.
pub fn parse_child_list(data: &[u8]) -> Result<Vec<ChildDevice>, ChildListError> {
    if data.len() < 3 {
        return Err(ChildListError::Short(data.len()));
    }
    let slots = usize::from(data[2]);
    let mut pos = 3;
    let mut out = Vec::new();
    for slot in 0..slots {
        let header = data
            .get(pos..pos + 8)
            .ok_or(ChildListError::Truncated { slot, offset: pos })?;
        let id_len = usize::from(header[7]);
        if header[2] == 0 && id_len == 0 {
            pos += 8;
            continue;
        }
        let id = data
            .get(pos + 8..pos + 8 + id_len)
            .ok_or(ChildListError::Truncated { slot, offset: pos })?;
        out.push(ChildDevice {
            slot,
            device_type: header[2],
            subtype: header[3],
            id: String::from_utf8_lossy(id).into_owned(),
        });
        pos += 8 + id_len;
    }
    Ok(out)
}

/// Parses `[type_lo, type_hi, count, (status, lo, hi) * count]` into raw
/// 16-bit values. Used for fan RPM (data type 0x06) and temperatures (data
/// type 0x10, value in 0.1 °C). Entries with status != 0 are reported as
/// `None` (probable meaning: sensor absent).
pub fn parse_sensor_values(data: &[u8]) -> Vec<Option<u16>> {
    let Some(&count) = data.get(2) else {
        return Vec::new();
    };
    (0..usize::from(count))
        .map_while(|i| data.get(3 + i * 3..6 + i * 3))
        .map(|e| (e[0] == 0).then(|| u16::from_le_bytes([e[1], e[2]])))
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct HubReport {
    pub vid: u32,
    pub pid: u32,
    pub firmware: String,
    pub mode: u32,
    pub children: Result<Vec<ChildDevice>, String>,
    pub fan_rpm: Result<Vec<Option<u16>>, String>,
    pub temperatures_c: Result<Vec<Option<f32>>, String>,
    /// Mode read back after a software window (`Some(1)` = hardware restored).
    pub restored_mode: Option<u32>,
    /// Every raw exchange as (request, response) for documentation.
    pub raw: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Read-only session with a hub.
pub struct LinkHubProbe<T: HidTransport> {
    t: T,
    timeout_ms: i32,
    raw: Vec<(Vec<u8>, Vec<u8>)>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("transport: {0}")]
    Transport(#[from] TransportError),
    #[error("{0:?}: {1}")]
    Response(Command, ResponseError),
    #[error("no response to {0:?}")]
    NoResponse(Command),
}

impl<T: HidTransport> LinkHubProbe<T> {
    pub fn new(t: T) -> Self {
        Self {
            t,
            timeout_ms: 500,
            raw: Vec::new(),
        }
    }

    fn transact(&mut self, pkt: Vec<u8>, cmd: Command) -> Result<Vec<u8>, ProbeError> {
        self.t.drain()?;
        self.t.write(&pkt)?;
        let mut buf = vec![0u8; self.t.input_report_len().max(8)];
        for _ in 0..8 {
            let n = self.t.read_timeout(&mut buf, self.timeout_ms)?;
            if n == 0 {
                break;
            }
            let resp = buf[..n].to_vec();
            let matches = parse(&resp, cmd).is_ok();
            self.raw.push((pkt.clone(), resp.clone()));
            if matches {
                return Ok(resp);
            }
        }
        Err(ProbeError::NoResponse(cmd))
    }

    fn ok(&mut self, pkt: Vec<u8>, cmd: Command) -> Result<Vec<u8>, ProbeError> {
        let raw = self.transact(pkt, cmd)?;
        parse(&raw, cmd)
            .and_then(Response::ok)
            .map_err(|e| ProbeError::Response(cmd, e))?;
        Ok(raw)
    }

    pub fn property(&mut self, prop: Property) -> Result<u32, ProbeError> {
        let raw = self.ok(get_property(TARGET_HUB, prop), Command::GetProperty)?;
        let r = parse(&raw, Command::GetProperty)
            .map_err(|e| ProbeError::Response(Command::GetProperty, e))?;
        Ok(r.property_value())
    }

    /// Opens `endpoint` on the Background handle, reads one report, closes it.
    pub fn read_endpoint(&mut self, endpoint: u8) -> Result<Vec<u8>, ProbeError> {
        let check = self.transact(
            check_handle(TARGET_HUB, Handle::Background),
            Command::CheckHandle,
        )?;
        let open = parse(&check, Command::CheckHandle)
            .map(|r| r.error != 3)
            .unwrap_or(false);
        if open {
            self.ok(
                close_handle(TARGET_HUB, Handle::Background),
                Command::CloseHandle,
            )?;
        }
        self.ok(
            open_endpoint(TARGET_HUB, Handle::Background, endpoint),
            Command::OpenEndpoint,
        )?;
        let result = self.ok(
            read_endpoint(TARGET_HUB, Handle::Background),
            Command::ReadEndpoint,
        );
        let close = self.ok(
            close_handle(TARGET_HUB, Handle::Background),
            Command::CloseHandle,
        );
        let raw = result?;
        close?;
        let r = parse(&raw, Command::ReadEndpoint)
            .map_err(|e| ProbeError::Response(Command::ReadEndpoint, e))?;
        Ok(r.data.to_vec())
    }

    fn set_mode(&mut self, mode: Mode) -> Result<(), ProbeError> {
        self.ok(set_mode_request(TARGET_HUB, mode), Command::SetProperty)
            .map(|_| ())
    }

    /// Like [`Self::run`], but if the hub is in hardware mode it is switched
    /// to software mode only for the endpoint reads and then returned to
    /// hardware mode. The return is attempted even if a read fails, retried,
    /// and verified by reading the mode back; `HubReport::restored_mode` holds
    /// the read-back value.
    pub fn run_with_software_window(mut self) -> Result<HubReport, ProbeError> {
        self.ok(ping(TARGET_HUB), Command::Ping)?;
        let before = self.property(Property::Mode)?;
        if before != Mode::Hardware as u32 {
            // Not our state to change; read without switching.
            return self.run();
        }
        self.set_mode(Mode::Software)?;
        let mut report = self.read_all();

        let mut restored = None;
        for _ in 0..3 {
            if self.set_mode(Mode::Hardware).is_ok() {
                if let Ok(m) = self.property(Property::Mode) {
                    restored = Some(m);
                    if m == Mode::Hardware as u32 {
                        break;
                    }
                }
            }
        }
        // Report the mode the hub was in before the window, not during it.
        report.mode = before;
        report.restored_mode = restored;
        report.raw = std::mem::take(&mut self.raw);
        Ok(report)
    }

    fn read_all(&mut self) -> HubReport {
        let children = self
            .read_endpoint(EP_CHILD_DEVICES)
            .map_err(|e| e.to_string())
            .and_then(|d| parse_child_list(&d).map_err(|e| e.to_string()));
        let fan_rpm = self
            .read_endpoint(EP_FAN_RPM)
            .map(|d| parse_sensor_values(&d))
            .map_err(|e| e.to_string());
        let temperatures_c = self
            .read_endpoint(EP_TEMPERATURES)
            .map(|d| {
                parse_sensor_values(&d)
                    .into_iter()
                    .map(|v| v.map(|t| f32::from(t) / 10.0))
                    .collect()
            })
            .map_err(|e| e.to_string());

        let id = (|| -> Result<_, ProbeError> {
            Ok((
                self.property(Property::VendorId)?,
                self.property(Property::ProductId)?,
                self.property(Property::Firmware)?,
                self.property(Property::Mode)?,
            ))
        })();
        let (vid, pid, fw, mode) = id.unwrap_or((0, 0, 0, 0));
        HubReport {
            vid,
            pid,
            firmware: format!("{}.{}.{}", fw & 0xFF, (fw >> 8) & 0xFF, (fw >> 16) & 0xFF),
            mode,
            children,
            fan_rpm,
            temperatures_c,
            restored_mode: None,
            raw: Vec::new(),
        }
    }

    /// Strictly read-only: never changes the mode.
    pub fn run(mut self) -> Result<HubReport, ProbeError> {
        self.ok(ping(TARGET_HUB), Command::Ping)?;
        let mut report = self.read_all();
        report.raw = std::mem::take(&mut self.raw);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(t: u8, sub: u8, id: &str) -> Vec<u8> {
        let mut v = vec![0, 0, t, sub, 0, 0, 0x05, id.len() as u8];
        v.extend_from_slice(id.as_bytes());
        v
    }

    /// Same structure as the packet logged on the development machine
    /// (types, subtypes, id lengths and empty slots), with synthetic IDs.
    fn sample_payload() -> Vec<u8> {
        let mut d = vec![0x21, 0x00, 15];
        d.extend(slot(15, 0, "01000000000000000000000001"));
        d.extend(slot(15, 0, "01000000000000000000000002"));
        d.extend(slot(15, 0, "01000000000000000000000003"));
        d.extend(slot(17, 2, "01000000000000000000000004"));
        d.extend(slot(6, 0, "LCD000000000000000000005"));
        d.extend([0u8; 8 * 7]);
        d.extend(slot(27, 0, "0700000000000000000000000600"));
        d.extend(slot(2, 0, "01000000000000000000000007"));
        d.extend(slot(27, 0, "0700000000000000000000000601"));
        d.resize(509, 0);
        d
    }

    #[test]
    fn parses_child_list_like_the_logged_packet() {
        let c = parse_child_list(&sample_payload()).unwrap();
        let summary: Vec<_> = c
            .iter()
            .map(|d| (d.slot, d.device_type, d.subtype))
            .collect();
        assert_eq!(
            summary,
            vec![
                (0, 15, 0),
                (1, 15, 0),
                (2, 15, 0),
                (3, 17, 2),
                (4, 6, 0),
                (12, 27, 0),
                (13, 2, 0),
                (14, 27, 0)
            ]
        );
        assert_eq!(c[3].model_name(), Some("LINK Titan 360 RX"));
        assert_eq!(c[3].led_count(), Some(20));
        assert_eq!(c[5].model_name(), None);
        assert_ne!(c[5].id, c[7].id, "type-27 IDs must stay distinct");
    }

    /// Simulated hub: endpoints only open in software mode; temperature
    /// endpoint optionally broken to exercise the restore path.
    fn sim_hub(
        mode: std::sync::Arc<std::sync::Mutex<u8>>,
        broken_temps: bool,
    ) -> freergb_device_api::mock::MockTransport {
        let mut open: Option<u8> = None;
        freergb_device_api::mock::MockTransport::new(513, 513, move |r| {
            assert_eq!(&r[..3], &[0, 0, TARGET_HUB], "hub framing");
            let (cmd, a, b) = (r[3], r[4], r[5]);
            let mut m = mode.lock().unwrap();
            let mut resp = vec![0, 0, cmd, 0];
            match cmd {
                0x12 => {}
                0x02 => {
                    let v: u32 = match a {
                        0x11 => 0x1B1C,
                        0x12 => 0x0C3F,
                        0x13 => 0x90_01_04,
                        0x03 => u32::from(*m),
                        _ => 0,
                    };
                    resp.extend_from_slice(&v.to_le_bytes()[..3]);
                }
                0x01 => {
                    assert_eq!(a, 0x03, "only the mode may be written");
                    *m = r[6];
                }
                0x09 => resp[3] = if open.is_some() { 0 } else { 3 },
                0x05 => open = None,
                0x0D => {
                    if *m != 2 || (broken_temps && b == EP_TEMPERATURES) {
                        resp[3] = 3;
                    } else {
                        open = Some(b);
                    }
                }
                0x08 => match open {
                    Some(EP_CHILD_DEVICES) => resp.extend(sample_payload()),
                    Some(EP_FAN_RPM) => resp.extend([0x06, 0, 1, 0, 0xB0, 0x04]),
                    Some(EP_TEMPERATURES) => resp.extend([0x10, 0, 1, 0, 0x2C, 0x01]),
                    _ => resp[3] = 3,
                },
                _ => panic!("unexpected command {cmd:#04x} sent to hub"),
            }
            resp.resize(512, 0);
            vec![resp]
        })
    }

    #[test]
    fn read_only_run_never_changes_mode() {
        let mode = std::sync::Arc::new(std::sync::Mutex::new(1u8));
        let r = LinkHubProbe::new(sim_hub(mode.clone(), false))
            .run()
            .unwrap();
        assert_eq!(r.firmware, "4.1.144");
        assert!(r.children.is_err(), "endpoints are closed in hardware mode");
        assert_eq!(*mode.lock().unwrap(), 1);
    }

    #[test]
    fn software_window_reads_and_restores_hardware_mode() {
        let mode = std::sync::Arc::new(std::sync::Mutex::new(1u8));
        let r = LinkHubProbe::new(sim_hub(mode.clone(), false))
            .run_with_software_window()
            .unwrap();
        assert_eq!(r.children.unwrap().len(), 8);
        assert_eq!(r.fan_rpm.unwrap(), vec![Some(1200)]);
        assert_eq!(r.temperatures_c.unwrap(), vec![Some(30.0)]);
        assert_eq!(r.restored_mode, Some(1));
        assert_eq!(*mode.lock().unwrap(), 1);
    }

    #[test]
    fn hardware_mode_is_restored_even_if_a_read_fails() {
        let mode = std::sync::Arc::new(std::sync::Mutex::new(1u8));
        let r = LinkHubProbe::new(sim_hub(mode.clone(), true))
            .run_with_software_window()
            .unwrap();
        assert!(r.temperatures_c.is_err());
        assert_eq!(r.restored_mode, Some(1));
        assert_eq!(*mode.lock().unwrap(), 1);
    }

    #[test]
    fn truncated_child_list_is_an_error() {
        let mut d = vec![0x21, 0x00, 2];
        d.extend(slot(15, 0, "0100"));
        d.extend([0, 0, 15, 0, 0, 0, 5, 26]);
        assert!(matches!(
            parse_child_list(&d),
            Err(ChildListError::Truncated { slot: 1, .. })
        ));
        assert!(parse_child_list(&[0x21]).is_err());
    }

    #[test]
    fn sensor_values() {
        let d = [0x10, 0x00, 3, 0, 0x2C, 0x01, 1, 0, 0, 0, 0xDC, 0x05];
        assert_eq!(parse_sensor_values(&d), vec![Some(300), None, Some(1500)]);
        assert!(parse_sensor_values(&[]).is_empty());
    }

    #[test]
    fn request_framing_and_response_offsets() {
        assert_eq!(
            get_property(TARGET_HUB, Property::ProductId),
            vec![0, 0, 0x01, 0x02, 0x12, 0x00]
        );
        assert_eq!(
            read_endpoint(TARGET_HUB, Handle::Background),
            vec![0, 0, 0x01, 0x08, 0x01]
        );
        // Logged form (report ID included): cmd at index 3.
        let logged = [0, 0, 0, 0x08, 0, 0x21, 0, 15];
        assert_eq!(parse(&logged, Command::ReadEndpoint).unwrap().data[2], 15);
        // hidapi form (report ID stripped): cmd at index 2.
        let stripped = [0, 0, 0x08, 0, 0x21, 0, 15];
        assert_eq!(parse(&stripped, Command::ReadEndpoint).unwrap().data[2], 15);
    }
}
