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
            (27, _) => "iCUE COMMANDER DUO",
            _ => return None,
        })
    }

    /// LEDs this child occupies in the hub's lighting buffer.
    ///
    /// Sources: SignalRGB plugin table (RX 8, Titan 20, LX 18, QX 34, LCD ring
    /// 24) and the OpenLinkHub device database (GPL-3.0, facts only), which
    /// lists type 27 as "iCUE COMMANDER DUO" with 0 LED channels and agrees on
    /// the RX/LX/Titan counts.
    pub fn led_count(&self) -> Option<usize> {
        match self.device_type {
            15 => Some(8),
            17 => Some(20),
            2 => Some(18),
            1 => Some(34),
            6 => Some(24),
            27 => Some(0),
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
    pub(super) t: T,
    timeout_ms: i32,
    raw: Vec<(Vec<u8>, Vec<u8>)>,
    /// Keep every exchange in `raw` (diagnostics). Off for long-running use.
    record_raw: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("transport: {0}")]
    Transport(#[from] TransportError),
    #[error("{0:?}: {1}")]
    Response(Command, ResponseError),
    #[error("no response to {0:?}")]
    NoResponse(Command),
    #[error("child list: {0}")]
    ChildList(#[from] ChildListError),
    #[error("refused: {0}")]
    Refused(String),
}

impl<T: HidTransport> LinkHubProbe<T> {
    pub fn new(t: T) -> Self {
        Self {
            t,
            timeout_ms: 500,
            raw: Vec::new(),
            record_raw: true,
        }
    }

    /// Disables raw exchange recording (for long-running sessions).
    pub fn without_raw_log(mut self) -> Self {
        self.record_raw = false;
        self
    }

    pub(super) fn transact(&mut self, pkt: Vec<u8>, cmd: Command) -> Result<Vec<u8>, ProbeError> {
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
            if self.record_raw {
                self.raw.push((pkt.clone(), resp.clone()));
            }
            if matches {
                return Ok(resp);
            }
        }
        Err(ProbeError::NoResponse(cmd))
    }

    pub(super) fn ok(&mut self, pkt: Vec<u8>, cmd: Command) -> Result<Vec<u8>, ProbeError> {
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

    pub(super) fn set_mode(&mut self, mode: Mode) -> Result<(), ProbeError> {
        self.ok(set_mode_request(TARGET_HUB, mode), Command::SetProperty)
            .map(|_| ())
    }

    /// Like [`Self::run`], but if the hub is in hardware mode it is switched
    /// to software mode only for the endpoint reads and then returned to
    /// hardware mode. The return is attempted even if a read fails, retried,
    /// and verified by reading the mode back; `HubReport::restored_mode` holds
    /// the read-back value.
    /// Puts the hub back into hardware mode (its own fan/pump curves and
    /// lighting), e.g. after another program was killed while controlling it.
    /// Returns the mode read back afterwards.
    pub fn restore_hardware_mode(mut self) -> Result<(u32, u32), ProbeError> {
        self.ok(ping(TARGET_HUB), Command::Ping)?;
        let before = self.property(Property::Mode)?;
        let _ = self.ok(
            close_handle(TARGET_HUB, Handle::Lighting),
            Command::CloseHandle,
        );
        if before != Mode::Hardware as u32 {
            self.set_mode(Mode::Hardware)?;
        }
        Ok((before, self.property(Property::Mode)?))
    }

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

// ---------------------------------------------------------------------------
// Lighting (experimental). Evidence: plugin `SendRGBData`/`getColors` [P].

/// Target byte for lighting writes (`0x08 | hub`).
pub const TARGET_HUB_LIGHTING: u8 = 0x08 | TARGET_HUB;
pub const EP_LIGHTING_CONTROLLER: u8 = 0x22;
/// Report ID, 0x00, target, cmd, handle, u32 length.
const WRITE_HEADER: usize = 9;
/// Report ID, 0x00, target, cmd, handle.
const STREAM_HEADER: usize = 5;

/// Children whose LED segment position in the chain is known: the leading
/// run of children with a known LED count. Everything after the first
/// unknown type is left out, because its LED count would shift all later
/// offsets. Returns `(child, led_count)` in chain order.
pub fn known_lighting_prefix(children: &[ChildDevice]) -> Vec<(ChildDevice, usize)> {
    let mut out = Vec::new();
    for c in children {
        match c.led_count() {
            // Children without LEDs (e.g. Commander DUO) take no buffer space.
            Some(0) => continue,
            Some(n) => out.push((c.clone(), n)),
            None => break,
        }
    }
    out
}

/// `[0x12, 0x00]` + RGB for every LED of every segment, in order.
pub fn encode_lighting(segments: &[Vec<freergb_core::Rgb>]) -> Vec<u8> {
    let mut p = vec![0x12, 0x00];
    for seg in segments {
        for c in seg {
            p.extend_from_slice(&c.to_array());
        }
    }
    p
}

/// First packet `WriteEndpoint` (target `0x09`, length), then unacknowledged
/// `StreamEndpoint` packets (target `0x01`).
pub fn lighting_packets(payload: &[u8], out_len: usize) -> Vec<Vec<u8>> {
    let first_cap = out_len - WRITE_HEADER;
    let (first, mut rest) = payload.split_at(payload.len().min(first_cap));
    let mut p = vec![
        0,
        0,
        TARGET_HUB_LIGHTING,
        Command::WriteEndpoint as u8,
        Handle::Lighting as u8,
    ];
    p.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    p.extend_from_slice(first);
    let mut out = vec![p];
    while !rest.is_empty() {
        let (chunk, r) = rest.split_at(rest.len().min(out_len - STREAM_HEADER));
        let mut s = vec![
            0,
            0,
            TARGET_HUB,
            Command::StreamEndpoint as u8,
            Handle::Lighting as u8,
        ];
        s.extend_from_slice(chunk);
        out.push(s);
        rest = r;
    }
    out
}

/// Fan/pump RPM watchdog. Every sensor that spun above 300 rpm at the start
/// is monitored. A sample is *bad* if the sensor is missing/unreadable or
/// below 50 % of its start value. Two bad samples in a row on the same sensor
/// raise the alarm; a single bad sample is counted as a glitch. (Soak test
/// 2026-10-18 showed one isolated unreadable sample after ~4 min.)
#[derive(Debug, Clone, PartialEq)]
pub struct RpmWatchdog {
    baseline: Vec<Option<u16>>,
    strikes: Vec<u8>,
    pub glitches: u32,
    /// The last bad sample, for diagnostics.
    pub last_bad: Option<Vec<Option<u16>>>,
}

impl RpmWatchdog {
    pub const STRIKES_TO_ALARM: u8 = 2;

    pub fn new(baseline: Vec<Option<u16>>) -> Self {
        Self {
            strikes: vec![0; baseline.len()],
            baseline,
            glitches: 0,
            last_bad: None,
        }
    }

    pub fn baseline(&self) -> &[Option<u16>] {
        &self.baseline
    }

    /// Feeds one sample; returns an alarm message when it trips.
    pub fn check(&mut self, now: &[Option<u16>]) -> Option<String> {
        let mut alarm = None;
        let mut any_bad = false;
        for (i, b) in self.baseline.iter().enumerate() {
            let Some(b) = *b else { continue };
            if b <= 300 {
                continue;
            }
            let n = now.get(i).copied().flatten();
            let bad = n.map_or(true, |n| n < b / 2);
            if bad {
                any_bad = true;
                self.strikes[i] += 1;
                if self.strikes[i] >= Self::STRIKES_TO_ALARM && alarm.is_none() {
                    alarm = Some(format!(
                        "rpm sensor {i}: {} bad samples in a row (start {b}, now {n:?})",
                        self.strikes[i]
                    ));
                }
            } else {
                self.strikes[i] = 0;
            }
        }
        if any_bad {
            self.glitches += 1;
            self.last_bad = Some(now.to_vec());
        }
        alarm
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LightingTestReport {
    pub segments: Vec<(ChildDevice, usize)>,
    pub frames: u32,
    pub rpm_before: Vec<Option<u16>>,
    /// RPM samples taken during the test (about once per second).
    pub rpm_during: Vec<Vec<Option<u16>>>,
    pub aborted: Option<String>,
    /// Isolated bad samples tolerated by the watchdog, and the last of them.
    pub glitches: u32,
    pub last_bad_sample: Option<Vec<Option<u16>>>,
    pub restored_mode: Option<u32>,
}

impl<T: HidTransport> LinkHubProbe<T> {
    pub(super) fn rpm(&mut self) -> Vec<Option<u16>> {
        self.read_endpoint(EP_FAN_RPM)
            .map(|d| parse_sensor_values(&d))
            .unwrap_or_default()
    }

    /// Shows `color_for(segment_index)` on the known chain prefix for
    /// `duration`, while watching the RPM of every fan/pump that was spinning
    /// before. Aborts if any of them drops below half its starting speed.
    /// Always returns the hub to hardware mode and reads it back.
    pub fn lighting_test(
        mut self,
        color_for: impl Fn(usize) -> freergb_core::Rgb,
        duration: std::time::Duration,
    ) -> Result<LightingTestReport, ProbeError> {
        self.ok(ping(TARGET_HUB), Command::Ping)?;
        let mode = self.property(Property::Mode)?;
        if mode != Mode::Hardware as u32 {
            return Err(ProbeError::Refused(format!(
                "hub is in mode {mode}, expected hardware (1); another program may control it"
            )));
        }
        self.set_mode(Mode::Software)?;

        let mut report = LightingTestReport {
            segments: Vec::new(),
            frames: 0,
            rpm_before: Vec::new(),
            rpm_during: Vec::new(),
            aborted: None,
            glitches: 0,
            last_bad_sample: None,
            restored_mode: None,
        };
        let body = (|| -> Result<(), ProbeError> {
            let children = self.read_endpoint(EP_CHILD_DEVICES)?;
            let children = parse_child_list(&children)?;
            report.segments = known_lighting_prefix(&children);
            report.rpm_before = self.rpm();

            let segments: Vec<Vec<freergb_core::Rgb>> = report
                .segments
                .iter()
                .enumerate()
                .map(|(i, (_, n))| vec![color_for(i); *n])
                .collect();
            let packets = lighting_packets(&encode_lighting(&segments), self.t.output_report_len());

            self.ok(
                open_endpoint(TARGET_HUB, Handle::Lighting, EP_LIGHTING_CONTROLLER),
                Command::OpenEndpoint,
            )?;
            let mut watchdog = RpmWatchdog::new(report.rpm_before.clone());
            let start = std::time::Instant::now();
            let mut last_check = start;
            while start.elapsed() < duration {
                for (i, p) in packets.iter().enumerate() {
                    if i == 0 {
                        self.ok(p.clone(), Command::WriteEndpoint)?;
                    } else {
                        self.t.write(p)?;
                    }
                }
                report.frames += 1;
                if last_check.elapsed() >= std::time::Duration::from_secs(1) {
                    last_check = std::time::Instant::now();
                    let now = self.rpm();
                    let alarm = watchdog.check(&now);
                    report.rpm_during.push(now);
                    if let Some(msg) = alarm {
                        report.aborted = Some(msg);
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            report.glitches = watchdog.glitches;
            report.last_bad_sample = watchdog.last_bad.clone();
            Ok(())
        })();

        // Restore regardless of what happened above.
        let _ = self.ok(
            close_handle(TARGET_HUB, Handle::Lighting),
            Command::CloseHandle,
        );
        for _ in 0..3 {
            if self.set_mode(Mode::Hardware).is_ok() {
                if let Ok(m) = self.property(Property::Mode) {
                    report.restored_mode = Some(m);
                    if m == Mode::Hardware as u32 {
                        break;
                    }
                }
            }
        }
        body.map(|()| report)
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
        assert_eq!(c[5].model_name(), Some("iCUE COMMANDER DUO"));
        assert_eq!(c[5].led_count(), Some(0));
        assert_ne!(c[5].id, c[7].id, "type-27 IDs must stay distinct");
    }

    /// Simulated hub: endpoints only open in software mode; temperature
    /// endpoint optionally broken to exercise the restore path.
    fn sim_hub(
        mode: std::sync::Arc<std::sync::Mutex<u8>>,
        broken_temps: bool,
    ) -> freergb_device_api::mock::MockTransport {
        sim_hub_full(SimHub {
            mode,
            broken_temps,
            ..SimHub::default()
        })
    }

    #[derive(Default, Clone)]
    struct SimHub {
        mode: std::sync::Arc<std::sync::Mutex<u8>>,
        broken_temps: bool,
        /// Declared payload length of the last lighting write and bytes received.
        lighting: std::sync::Arc<std::sync::Mutex<(usize, usize, Vec<u8>)>>,
        /// Pump RPM reported at sensor index 4.
        pump_rpm: std::sync::Arc<std::sync::Mutex<u16>>,
    }

    fn sim_hub_full(s: SimHub) -> freergb_device_api::mock::MockTransport {
        let mut open: [Option<u8>; 3] = [None; 3];
        *s.pump_rpm.lock().unwrap() = 1700;
        freergb_device_api::mock::MockTransport::new(513, 513, move |r| {
            let (target, cmd, a, b) = (r[2], r[3], r[4], r[5]);
            let mut m = s.mode.lock().unwrap();
            if cmd == 0x07 {
                assert_eq!(target, TARGET_HUB);
                assert_eq!(open[0], Some(EP_LIGHTING_CONTROLLER));
                let mut l = s.lighting.lock().unwrap();
                let take = (l.0 - l.1).min(r.len() - 5);
                l.2.extend_from_slice(&r[5..5 + take]);
                l.1 += take;
                return vec![]; // streams are not acknowledged
            }
            if cmd == 0x06 {
                assert_eq!(target, TARGET_HUB_LIGHTING, "lighting writes use 0x09");
            } else {
                assert_eq!(target, TARGET_HUB, "hub framing");
            }
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
                0x09 => resp[3] = if open[usize::from(a)].is_some() { 0 } else { 3 },
                0x05 => open[usize::from(b)] = None,
                0x0D => {
                    if *m != 2
                        || (s.broken_temps && b == EP_TEMPERATURES)
                        || open[usize::from(a)].is_some()
                    {
                        resp[3] = 3;
                    } else {
                        open[usize::from(a)] = Some(b);
                    }
                }
                0x06 => {
                    assert_eq!(*m, 2, "lighting only in software mode");
                    assert_eq!(open[0], Some(EP_LIGHTING_CONTROLLER));
                    let len = u32::from_le_bytes([r[5], r[6], r[7], r[8]]) as usize;
                    let first = len.min(r.len() - 9);
                    *s.lighting.lock().unwrap() = (len, first, r[9..9 + first].to_vec());
                }
                0x08 => match open[usize::from(a)] {
                    Some(EP_CHILD_DEVICES) => resp.extend(sample_payload()),
                    Some(EP_FAN_RPM) => {
                        let p = s.pump_rpm.lock().unwrap().to_le_bytes();
                        resp.extend([
                            0x25, 0, 5, 1, 0, 0, 0, 0xD8, 0x02, 0, 0xD9, 0x02, 0, 0xD6, 0x02, 0,
                            p[0], p[1],
                        ]);
                    }
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
    fn lighting_prefix_skips_ledless_children_and_stops_at_unknown() {
        let c = parse_child_list(&sample_payload()).unwrap();
        let seg: Vec<_> = known_lighting_prefix(&c)
            .iter()
            .map(|(d, n)| (d.slot, *n))
            .collect();
        // RX, RX, RX, Titan, LCD, (Commander DUO: no LEDs), LX, (Commander DUO).
        assert_eq!(
            seg,
            vec![(0, 8), (1, 8), (2, 8), (3, 20), (4, 24), (13, 18)]
        );

        // An unknown type still ends the prefix.
        let mut unknown = c.clone();
        unknown[5].device_type = 99;
        let seg: Vec<_> = known_lighting_prefix(&unknown)
            .iter()
            .map(|(d, _)| d.slot)
            .collect();
        assert_eq!(seg, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn lighting_packets_split_and_target() {
        let payload: Vec<u8> = (0..1100u32).map(|i| i as u8).collect();
        let p = lighting_packets(&payload, 513);
        assert_eq!(p.len(), 3);
        assert_eq!(&p[0][..9], &[0, 0, 0x09, 0x06, 0x00, 0x4C, 0x04, 0, 0]);
        assert_eq!(&p[1][..5], &[0, 0, 0x01, 0x07, 0x00]);
        let total: usize = p[0].len() - 9 + p[1].len() - 5 + p[2].len() - 5;
        assert_eq!(total, 1100);
    }

    #[test]
    fn lighting_test_in_simulator_writes_expected_buffer_and_restores() {
        use freergb_core::Rgb;
        let sim = SimHub::default();
        *sim.mode.lock().unwrap() = 1;
        let palette = [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE, Rgb::BLACK];
        let r = LinkHubProbe::new(sim_hub_full(sim.clone()))
            .lighting_test(
                |i| palette[i % palette.len()],
                std::time::Duration::from_millis(1200),
            )
            .unwrap();
        assert!(r.frames >= 1);
        assert_eq!(r.aborted, None);
        assert_eq!(r.restored_mode, Some(1));
        assert_eq!(*sim.mode.lock().unwrap(), 1);

        let (declared, received, data) = sim.lighting.lock().unwrap().clone();
        // 2-byte prefix + (3*8 + 20 + 24) LEDs * 3.
        assert_eq!(declared, 2 + 86 * 3);
        assert_eq!(received, declared);
        assert_eq!(&data[..2], &[0x12, 0x00]);
        assert_eq!(&data[2..5], &[255, 0, 0], "fan 1 red");
        assert_eq!(&data[2 + 8 * 3..2 + 8 * 3 + 3], &[0, 255, 0], "fan 2 green");
        assert_eq!(
            &data[2 + 24 * 3..2 + 24 * 3 + 3],
            &[255, 255, 255],
            "AIO white"
        );
    }

    #[test]
    fn lighting_test_aborts_when_pump_slows_and_still_restores() {
        let sim = SimHub::default();
        *sim.mode.lock().unwrap() = 1;
        let pump = sim.pump_rpm.clone();
        let t = sim_hub_full(sim.clone());
        // The simulator starts at 1700 rpm; drop it after the start reading.
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            *pump.lock().unwrap() = 400;
        });
        let r = LinkHubProbe::new(t)
            .lighting_test(
                |_| freergb_core::Rgb::RED,
                std::time::Duration::from_secs(5),
            )
            .unwrap();
        handle.join().unwrap();
        assert!(r.aborted.is_some(), "must abort on pump slowdown");
        assert_eq!(r.restored_mode, Some(1));
        assert_eq!(*sim.mode.lock().unwrap(), 1);
    }

    #[test]
    fn watchdog_tolerates_single_glitch_but_alarms_on_two() {
        let mut w = RpmWatchdog::new(vec![None, Some(720), Some(1700), Some(200)]);
        assert_eq!(
            w.check(&[None, Some(730), Some(1690), Some(0)]),
            None,
            "low-rpm sensor ignored"
        );
        assert_eq!(w.check(&[None, None, Some(1690)]), None, "single glitch");
        assert_eq!(w.glitches, 1);
        assert_eq!(w.check(&[None, Some(725), Some(1700)]), None, "recovered");
        assert_eq!(
            w.check(&[None, Some(725), Some(800)]),
            None,
            "first low pump sample"
        );
        assert!(
            w.check(&[None, Some(725), Some(790)]).is_some(),
            "second low sample alarms"
        );
    }

    #[test]
    fn watchdog_alarms_on_repeated_unreadable_rpm() {
        let mut w = RpmWatchdog::new(vec![None, Some(720)]);
        assert_eq!(w.check(&[]), None);
        assert!(
            w.check(&[]).is_some(),
            "empty reads must not be silently ignored"
        );
    }

    #[test]
    fn hub_device_lifecycle_restores_hardware_mode() {
        use super::super::link_hub_device::LinkHubDevice;
        use freergb_core::{Frame, Rgb};
        use freergb_device_api::{DeviceStatus, RgbDevice};

        let sim = SimHub::default();
        *sim.mode.lock().unwrap() = 1;
        let mut dev = LinkHubDevice::new(sim_hub_full(sim.clone()), Some("SIM".into()));
        dev.connect().unwrap();
        assert_eq!(*sim.mode.lock().unwrap(), 2);
        assert_eq!(dev.layout().len(), 86);
        assert_eq!(dev.layout().height, 6);
        assert_eq!(dev.info().firmware.as_deref(), Some("4.1.144"));

        let mut f = Frame::solid(86, Rgb::BLACK);
        f.colors_mut()[24] = Rgb::new(1, 2, 3); // first LED of the AIO segment
        dev.push_frame(&f).unwrap();
        let data = sim.lighting.lock().unwrap().2.clone();
        assert_eq!(&data[2 + 24 * 3..2 + 24 * 3 + 3], &[1, 2, 3]);

        dev.disconnect(None).unwrap();
        assert_eq!(dev.status(), DeviceStatus::Disconnected);
        assert_eq!(*sim.mode.lock().unwrap(), 1);
    }

    #[test]
    fn hub_device_watchdog_trips_and_blocks_reconnect() {
        use super::super::link_hub_device::{LinkHubDevice, WATCHDOG_INTERVAL};
        use freergb_core::{Frame, Rgb};
        use freergb_device_api::{DeviceError, DeviceStatus, RgbDevice};

        let sim = SimHub::default();
        *sim.mode.lock().unwrap() = 1;
        let mut dev = LinkHubDevice::new(sim_hub_full(sim.clone()), None);
        dev.connect().unwrap();
        *sim.pump_rpm.lock().unwrap() = 300;
        // First bad sample is tolerated, the second one trips.
        std::thread::sleep(WATCHDOG_INTERVAL + std::time::Duration::from_millis(50));
        dev.push_frame(&Frame::solid(86, Rgb::RED)).unwrap();
        assert_eq!(
            *sim.mode.lock().unwrap(),
            2,
            "still running after one bad sample"
        );
        std::thread::sleep(WATCHDOG_INTERVAL + std::time::Duration::from_millis(50));
        let err = dev.push_frame(&Frame::solid(86, Rgb::RED)).unwrap_err();
        assert!(matches!(err, DeviceError::Refused(_)), "{err}");
        assert!(dev.tripped().is_some());
        assert!(matches!(dev.status(), DeviceStatus::Faulted(_)));
        assert_eq!(*sim.mode.lock().unwrap(), 1, "hardware mode restored");

        *sim.pump_rpm.lock().unwrap() = 1700;
        assert!(matches!(dev.connect(), Err(DeviceError::Refused(_))));
        assert_eq!(*sim.mode.lock().unwrap(), 1, "no reconnect after a trip");
    }

    #[test]
    fn hub_device_refuses_software_mode_start() {
        use super::super::link_hub_device::LinkHubDevice;
        use freergb_device_api::{DeviceError, RgbDevice};

        let sim = SimHub::default();
        *sim.mode.lock().unwrap() = 2;
        let mut dev = LinkHubDevice::new(sim_hub_full(sim.clone()), None);
        assert!(matches!(dev.connect(), Err(DeviceError::Refused(_))));
        assert_eq!(*sim.mode.lock().unwrap(), 2);
    }

    #[test]
    fn lighting_test_refuses_when_not_in_hardware_mode() {
        let sim = SimHub::default();
        *sim.mode.lock().unwrap() = 2;
        let r = LinkHubProbe::new(sim_hub_full(sim.clone())).lighting_test(
            |_| freergb_core::Rgb::RED,
            std::time::Duration::from_secs(1),
        );
        assert!(matches!(r, Err(ProbeError::Refused(_))));
        assert_eq!(*sim.mode.lock().unwrap(), 2, "mode untouched");
    }

    #[test]
    fn software_window_reads_and_restores_hardware_mode() {
        let mode = std::sync::Arc::new(std::sync::Mutex::new(1u8));
        let r = LinkHubProbe::new(sim_hub(mode.clone(), false))
            .run_with_software_window()
            .unwrap();
        assert_eq!(r.children.unwrap().len(), 8);
        assert_eq!(
            r.fan_rpm.unwrap(),
            vec![None, Some(728), Some(729), Some(726), Some(1700)]
        );
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
