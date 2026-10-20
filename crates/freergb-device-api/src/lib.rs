//! Hardware abstraction for freeRGB.
//!
//! The split is:
//! * [`HidTransport`] moves raw reports to and from one HID interface.
//! * A protocol (crate `freergb-protocols`) turns intents into reports.
//! * [`RgbDevice`] is what the engine talks to: identify, connect, push
//!   frames, report status, disconnect safely.

pub mod mock;
pub mod transport;
pub mod virtual_device;

use freergb_core::{Frame, LedLayout, Rgb};
use serde::{Deserialize, Serialize};

pub use transport::{HidTransport, TransportError};
pub use virtual_device::VirtualDevice;

/// Stable identifier of a device instance, e.g. `hid:1b1c:1bfd:<serial>` or
/// `virtual:test-strip`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeviceId(pub String);

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    Keyboard,
    Mouse,
    Headset,
    LightingController,
    FanHub,
    Cooler,
    Memory,
    Virtual,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub vendor: String,
    pub kind: DeviceKind,
    pub usb_vid: Option<u16>,
    pub usb_pid: Option<u16>,
    pub serial: Option<String>,
    pub firmware: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Disconnected,
    Connected,
    /// Device reported or caused an error; the engine may retry `connect`.
    Faulted(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorReading {
    pub id: String,
    pub name: String,
    pub unit: String,
    pub value: Option<f64>,
    pub age_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("transport: {0}")]
    Transport(#[from] TransportError),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("device is not connected")]
    NotConnected,
    #[error("frame has {got} colours, layout has {expected} LEDs")]
    FrameSize { expected: usize, got: usize },
    #[error("refused: {0}")]
    Refused(String),
}

/// What the engine needs from any RGB device, real or virtual.
pub trait RgbDevice: Send {
    /// Cached USB keyboard usage -> layout LED index; empty when unknown.
    /// Must never infer these indices from RGB wire positions or perform I/O.
    fn key_map(&self) -> std::collections::BTreeMap<u16, usize> {
        Default::default()
    }
    /// Cached sensor values only; must not perform transport I/O.
    fn sensors(&self) -> Vec<SensorReading> {
        Vec::new()
    }
    fn info(&self) -> &DeviceInfo;

    /// Opens the device and puts it into a state where frames can be pushed.
    /// Must be idempotent.
    fn connect(&mut self) -> Result<(), DeviceError>;

    fn layout(&self) -> &LedLayout;

    /// Sends one frame in layout order.
    fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError>;

    fn status(&self) -> DeviceStatus;

    /// Optionally shows `final_color`, then hands the device back to its
    /// hardware/firmware mode and releases it.
    fn disconnect(&mut self, final_color: Option<Rgb>) -> Result<(), DeviceError>;
}

impl DeviceError {
    /// The device was unplugged: its handle is dead and a fresh one must be
    /// opened (the engine drops the device; the next hot-plug scan re-adds it).
    pub fn is_gone(&self) -> bool {
        matches!(self, DeviceError::Transport(TransportError::Disconnected))
    }
}

/// Helper for implementations: checks a frame against the layout size.
pub fn check_frame(layout: &LedLayout, frame: &Frame) -> Result<(), DeviceError> {
    if frame.len() != layout.len() {
        return Err(DeviceError::FrameSize {
            expected: layout.len(),
            got: frame.len(),
        });
    }
    Ok(())
}
