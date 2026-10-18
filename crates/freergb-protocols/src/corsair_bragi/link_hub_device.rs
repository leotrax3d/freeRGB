//! iCUE LINK hub as an engine device (experimental, opt-in in the daemon).
//!
//! Lighting needs the hub in software mode for as long as frames are sent.
//! Safety rules:
//! * `connect` only starts from hardware mode (otherwise another program may
//!   be in charge of the hub).
//! * Fan/pump RPM is compared with the values at connect every
//!   [`WATCHDOG_INTERVAL`]. A drop below 50 % on any sensor that was spinning
//!   returns the hub to hardware mode at once and *trips* the device: it
//!   refuses to reconnect until the process restarts.
//! * `disconnect` closes the lighting handle and restores hardware mode,
//!   verified by reading the mode back.
//! * Only the known chain prefix is lit (see `known_lighting_prefix`).

use super::link_hub::{
    close_handle, encode_lighting, known_lighting_prefix, lighting_packets, open_endpoint,
    parse_child_list, ping, LinkHubProbe, ProbeError, EP_CHILD_DEVICES, EP_LIGHTING_CONTROLLER,
    TARGET_HUB,
};
use super::packet::{Command, Handle, Mode, Property};
use freergb_core::{Frame, Led, LedLayout, Rgb};
use freergb_device_api::{
    check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, HidTransport,
    RgbDevice,
};
use std::time::{Duration, Instant};

pub const WATCHDOG_INTERVAL: Duration = Duration::from_secs(2);

pub struct LinkHubDevice<T: HidTransport> {
    p: LinkHubProbe<T>,
    info: DeviceInfo,
    layout: LedLayout,
    /// LED count per lit segment, in chain order.
    segments: Vec<usize>,
    status: DeviceStatus,
    baseline: Vec<Option<u16>>,
    last_check: Instant,
    tripped: Option<String>,
}

fn proto(e: ProbeError) -> DeviceError {
    DeviceError::Protocol(e.to_string())
}

impl<T: HidTransport> LinkHubDevice<T> {
    pub fn new(transport: T, serial: Option<String>) -> Self {
        Self {
            p: LinkHubProbe::new(transport).without_raw_log(),
            info: DeviceInfo {
                id: DeviceId(format!(
                    "hid:1b1c:0c3f:{}",
                    serial.as_deref().unwrap_or("noserial")
                )),
                name: "CORSAIR iCUE LINK System Hub".into(),
                vendor: "Corsair".into(),
                kind: DeviceKind::FanHub,
                usb_vid: Some(0x1B1C),
                usb_pid: Some(0x0C3F),
                serial,
                firmware: None,
            },
            layout: LedLayout::strip(1),
            segments: Vec::new(),
            status: DeviceStatus::Disconnected,
            baseline: Vec::new(),
            last_check: Instant::now(),
            tripped: None,
        }
    }

    pub fn tripped(&self) -> Option<&str> {
        self.tripped.as_deref()
    }

    /// One row per segment, LEDs left to right.
    fn build_layout(names: &[String], segments: &[usize]) -> LedLayout {
        let width = segments.iter().copied().max().unwrap_or(1).max(1) as u16;
        let mut leds = Vec::new();
        for (row, (name, n)) in names.iter().zip(segments).enumerate() {
            for i in 0..*n {
                leds.push(Led {
                    name: format!("{name} {} LED {}", row + 1, i + 1),
                    pos: (i as u16, row as u16),
                });
            }
        }
        LedLayout::new(width, segments.len().max(1) as u16, leds)
            .expect("generated hub layout is valid")
    }

    /// Returns to hardware mode; retried and verified.
    fn restore_hardware(&mut self) -> Result<(), DeviceError> {
        let _ = self.p.ok(
            close_handle(TARGET_HUB, Handle::Lighting),
            Command::CloseHandle,
        );
        for _ in 0..3 {
            if self.p.set_mode(Mode::Hardware).is_ok()
                && self.p.property(Property::Mode).ok() == Some(Mode::Hardware as u32)
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(DeviceError::Protocol(
            "hub did not confirm hardware mode".into(),
        ))
    }

    fn connect_inner(&mut self) -> Result<(), DeviceError> {
        self.p.ok(ping(TARGET_HUB), Command::Ping).map_err(proto)?;
        let mode = self.p.property(Property::Mode).map_err(proto)?;
        if mode != Mode::Hardware as u32 {
            return Err(DeviceError::Refused(format!(
                "hub is in mode {mode}; another program may control it"
            )));
        }
        let fw = self.p.property(Property::Firmware).map_err(proto)?;
        self.info.firmware = Some(format!(
            "{}.{}.{}",
            fw & 0xFF,
            (fw >> 8) & 0xFF,
            (fw >> 16) & 0xFF
        ));

        self.p.set_mode(Mode::Software).map_err(proto)?;
        let result = (|| -> Result<(), DeviceError> {
            let data = self.p.read_endpoint(EP_CHILD_DEVICES).map_err(proto)?;
            let children =
                parse_child_list(&data).map_err(|e| DeviceError::Protocol(e.to_string()))?;
            let prefix = known_lighting_prefix(&children);
            let names: Vec<String> = prefix
                .iter()
                .map(|(c, _)| c.model_name().unwrap_or("Device").to_owned())
                .collect();
            let segments: Vec<usize> = prefix.iter().map(|(_, n)| *n).collect();
            if segments.is_empty() {
                return Err(DeviceError::Refused(
                    "no child with a known LED layout".into(),
                ));
            }
            if !self.segments.is_empty() && self.segments != segments {
                return Err(DeviceError::Refused(
                    "child chain changed since first connect; restart the daemon".into(),
                ));
            }
            self.layout = Self::build_layout(&names, &segments);
            self.segments = segments;
            self.baseline = self.p.rpm();
            self.p
                .ok(
                    open_endpoint(TARGET_HUB, Handle::Lighting, EP_LIGHTING_CONTROLLER),
                    Command::OpenEndpoint,
                )
                .map_err(proto)?;
            Ok(())
        })();
        if let Err(e) = result {
            let _ = self.restore_hardware();
            return Err(e);
        }
        self.last_check = Instant::now();
        Ok(())
    }

    /// Index of a sensor that dropped below half its baseline, if any.
    fn rpm_drop(baseline: &[Option<u16>], now: &[Option<u16>]) -> Option<usize> {
        baseline
            .iter()
            .zip(now)
            .position(|(b, n)| matches!((b, n), (Some(b), n) if *b > 300 && n.unwrap_or(0) < b / 2))
    }

    fn trip(&mut self, reason: String) -> DeviceError {
        tracing::error!(device = %self.info.id, %reason, "watchdog tripped, restoring hardware mode");
        let restored = self.restore_hardware();
        self.tripped = Some(reason.clone());
        self.status = DeviceStatus::Faulted(reason.clone());
        match restored {
            Ok(()) => DeviceError::Refused(format!("watchdog: {reason}")),
            Err(e) => DeviceError::Protocol(format!("watchdog: {reason}; restore failed: {e}")),
        }
    }
}

impl<T: HidTransport> RgbDevice for LinkHubDevice<T> {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn connect(&mut self) -> Result<(), DeviceError> {
        if let Some(r) = &self.tripped {
            return Err(DeviceError::Refused(format!(
                "watchdog tripped earlier ({r}); restart the daemon to retry"
            )));
        }
        if self.status == DeviceStatus::Connected {
            return Ok(());
        }
        self.connect_inner()?;
        self.status = DeviceStatus::Connected;
        Ok(())
    }

    fn layout(&self) -> &LedLayout {
        &self.layout
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        if self.status != DeviceStatus::Connected {
            return Err(DeviceError::NotConnected);
        }
        check_frame(&self.layout, frame)?;

        let mut colors = frame.colors().iter().copied();
        let segs: Vec<Vec<Rgb>> = self
            .segments
            .iter()
            .map(|n| colors.by_ref().take(*n).collect())
            .collect();
        let packets = lighting_packets(&encode_lighting(&segs), self.p.t.output_report_len());
        for (i, pkt) in packets.into_iter().enumerate() {
            let r = if i == 0 {
                self.p.ok(pkt, Command::WriteEndpoint).map(|_| ())
            } else {
                self.p.t.write(&pkt).map(|_| ()).map_err(ProbeError::from)
            };
            if let Err(e) = r {
                self.status = DeviceStatus::Faulted(e.to_string());
                let _ = self.restore_hardware();
                return Err(proto(e));
            }
        }

        if self.last_check.elapsed() >= WATCHDOG_INTERVAL {
            self.last_check = Instant::now();
            let now = self.p.rpm();
            if now.is_empty() {
                return Err(self.trip("RPM could not be read".into()));
            }
            if let Some(idx) = Self::rpm_drop(&self.baseline, &now) {
                return Err(self.trip(format!(
                    "rpm sensor {idx} dropped from {:?} to {:?}",
                    self.baseline[idx], now[idx]
                )));
            }
        }
        Ok(())
    }

    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }

    fn disconnect(&mut self, final_color: Option<Rgb>) -> Result<(), DeviceError> {
        if self.status != DeviceStatus::Connected {
            return Ok(());
        }
        if let Some(c) = final_color {
            let _ = self.push_frame(&Frame::solid(self.layout.len(), c));
        }
        let r = self.restore_hardware();
        self.status = DeviceStatus::Disconnected;
        r
    }
}
