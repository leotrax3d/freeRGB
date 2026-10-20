//! iCUE LINK hub as an engine device (experimental, opt-in in the daemon).
//!
//! Lighting needs the hub in software mode for as long as frames are sent.
//! Safety rules:
//! * `connect` only starts from hardware mode (otherwise another program may
//!   be in charge of the hub).
//! * Fan/pump RPM is checked every [`WATCHDOG_INTERVAL`] by an [`RpmWatchdog`]
//!   (two bad samples in a row on a sensor that was spinning at connect:
//!   missing or below 50 %). An alarm returns the hub to hardware mode at once
//!   and *trips* the device: it refuses to reconnect until the process restarts.
//! * `disconnect` closes the lighting handle and restores hardware mode,
//!   verified by reading the mode back.
//! * Only the known chain prefix is lit (see `known_lighting_prefix`).

use super::link_hub::{
    close_handle, encode_lighting, known_lighting_prefix, lighting_packets, open_endpoint,
    parse_child_list, ping, ChildDevice, LinkHubProbe, ProbeError, RpmWatchdog, EP_CHILD_DEVICES,
    EP_LIGHTING_CONTROLLER, TARGET_HUB,
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
    watchdog: RpmWatchdog,
    last_check: Instant,
    tripped: Option<String>,
    rpm: Vec<Option<u16>>,
    rpm_read_at: Option<Instant>,
    temperatures: Vec<Option<f64>>,
    temperatures_read_at: Option<Instant>,
    hardware_owned: bool,
    children: Vec<ChildDevice>,
}

fn proto(e: ProbeError) -> DeviceError {
    match e {
        // Keep transport errors (e.g. unplugged) distinguishable for the engine.
        ProbeError::Transport(t) => DeviceError::Transport(t),
        other => DeviceError::Protocol(other.to_string()),
    }
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
            watchdog: RpmWatchdog::new(Vec::new()),
            last_check: Instant::now(),
            tripped: None,
            rpm: Vec::new(),
            rpm_read_at: None,
            temperatures: Vec::new(),
            temperatures_read_at: None,
            hardware_owned: false,
            children: Vec::new(),
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
        if !self.hardware_owned {
            return Ok(());
        }
        let _ = self.p.ok(
            close_handle(TARGET_HUB, Handle::Lighting),
            Command::CloseHandle,
        );
        for _ in 0..3 {
            if self.p.set_mode(Mode::Hardware).is_ok()
                && self.p.property(Property::Mode).ok() == Some(Mode::Hardware as u32)
            {
                self.hardware_owned = false;
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

        self.hardware_owned = true;
        if let Err(error) = self.p.set_mode(Mode::Software) {
            let _ = self.restore_hardware();
            return Err(proto(error));
        }
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
            self.children = children;
            self.rpm = self.p.rpm();
            self.rpm_read_at = Some(Instant::now());
            if !self.rpm.iter().flatten().any(|rpm| *rpm > 300) {
                return Err(DeviceError::Refused(
                    "no valid spinning RPM baseline; refusing an unmonitored software session"
                        .into(),
                ));
            }
            self.watchdog = RpmWatchdog::new(self.rpm.clone());
            // Optional telemetry must not stop the RPM watchdog or falsify freshness.
            self.temperatures.clear();
            self.temperatures_read_at = None;
            if let Ok(values) = self.p.temperatures() {
                self.temperatures = values;
                self.temperatures_read_at = Some(Instant::now());
            }
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

    pub(super) fn refresh_telemetry(&mut self) -> Result<(), DeviceError> {
        let now = self.p.rpm();
        self.rpm = now.clone();
        self.rpm_read_at = Some(Instant::now());
        if let Some(msg) = self.watchdog.check(&now) {
            return Err(self.trip(msg));
        }
        if let Ok(values) = self.p.temperatures() {
            self.temperatures = values;
            self.temperatures_read_at = Some(Instant::now());
        }
        Ok(())
    }

    fn sensor_name(&self, index: usize, kind: &str) -> String {
        let candidate = index
            .checked_sub(1)
            .and_then(|slot| self.children.iter().find(|child| child.slot == slot));
        match candidate {
            Some(child) => format!(
                "{kind} {index} — candidate {} slot {} (physical identity unconfirmed)",
                child.model_name().unwrap_or("unknown child"),
                child.slot
            ),
            None => format!("{kind} {index} (channel identity unconfirmed)"),
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
            self.refresh_telemetry()?;
        }
        Ok(())
    }

    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }

    fn sensors(&self) -> Vec<freergb_device_api::SensorReading> {
        let age_ms = self.rpm_read_at.map_or(u64::MAX, |at| {
            at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
        });
        let mut readings: Vec<_> = self
            .rpm
            .iter()
            .enumerate()
            .map(|(index, value)| freergb_device_api::SensorReading {
                id: format!("rpm:{index}"),
                name: self.sensor_name(index, "RPM sensor"),
                unit: "rpm".into(),
                value: value.map(f64::from),
                age_ms,
            })
            .collect();
        let temperature_age = self.temperatures_read_at.map_or(u64::MAX, |at| {
            at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
        });
        readings.extend(self.temperatures.iter().enumerate().map(|(index, value)| {
            freergb_device_api::SensorReading {
                id: format!("temperature:{index}"),
                name: self.sensor_name(index, "Temperature sensor"),
                unit: "°C".into(),
                value: *value,
                age_ms: temperature_age,
            }
        }));
        readings
    }

    fn disconnect(&mut self, final_color: Option<Rgb>) -> Result<(), DeviceError> {
        if !self.hardware_owned {
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

impl<T: HidTransport> Drop for LinkHubDevice<T> {
    fn drop(&mut self) {
        if let Err(error) = self.restore_hardware() {
            tracing::error!(%error, "hub hardware-mode restore failed during cleanup");
        }
    }
}
