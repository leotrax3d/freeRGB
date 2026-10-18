//! A virtual RGB device that accepts frames and records them instead of
//! touching hardware. Used for tests, demos and as a debugging sink.

use crate::{check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, RgbDevice};
use freergb_core::{Frame, LedLayout, Rgb};
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Shared view on the frames a [`VirtualDevice`] received.
#[derive(Debug, Clone, Default)]
pub struct FrameLog {
    inner: Arc<Mutex<FrameLogInner>>,
}

#[derive(Debug, Default)]
struct FrameLogInner {
    frames: VecDeque<Frame>,
    total: u64,
}

impl FrameLog {
    const CAPACITY: usize = 1024;

    fn push(&self, f: Frame) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if g.frames.len() == Self::CAPACITY {
            g.frames.pop_front();
        }
        g.frames.push_back(f);
        g.total += 1;
    }

    /// Number of frames received since creation (not capped).
    pub fn total(&self) -> u64 {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).total
    }

    pub fn last(&self) -> Option<Frame> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .frames
            .back()
            .cloned()
    }

    /// The most recent frames (up to 1024), oldest first.
    pub fn snapshot(&self) -> Vec<Frame> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .frames
            .iter()
            .cloned()
            .collect()
    }
}

pub struct VirtualDevice {
    info: DeviceInfo,
    layout: LedLayout,
    connected: bool,
    log: FrameLog,
    sink: Option<Box<dyn Write + Send>>,
}

impl VirtualDevice {
    pub fn new(name: &str, layout: LedLayout) -> Self {
        Self {
            info: DeviceInfo {
                id: DeviceId(format!("virtual:{}", name.to_lowercase().replace(' ', "-"))),
                name: name.to_owned(),
                vendor: "freeRGB".to_owned(),
                kind: DeviceKind::Virtual,
                usb_vid: None,
                usb_pid: None,
                serial: None,
                firmware: None,
            },
            layout,
            connected: false,
            log: FrameLog::default(),
            sink: None,
        }
    }

    /// Additionally writes every frame as one JSON line to `sink`.
    pub fn with_sink(mut self, sink: impl Write + Send + 'static) -> Self {
        self.sink = Some(Box::new(sink));
        self
    }

    pub fn frame_log(&self) -> FrameLog {
        self.log.clone()
    }
}

impl RgbDevice for VirtualDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn connect(&mut self) -> Result<(), DeviceError> {
        self.connected = true;
        tracing::debug!(device = %self.info.id, "virtual device connected");
        Ok(())
    }

    fn layout(&self) -> &LedLayout {
        &self.layout
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        if !self.connected {
            return Err(DeviceError::NotConnected);
        }
        check_frame(&self.layout, frame)?;
        if let Some(sink) = self.sink.as_mut() {
            let line =
                serde_json::to_string(frame).map_err(|e| DeviceError::Protocol(e.to_string()))?;
            writeln!(sink, "{line}").map_err(|e| DeviceError::Protocol(e.to_string()))?;
        }
        self.log.push(frame.clone());
        Ok(())
    }

    fn status(&self) -> DeviceStatus {
        if self.connected {
            DeviceStatus::Connected
        } else {
            DeviceStatus::Disconnected
        }
    }

    fn disconnect(&mut self, final_color: Option<Rgb>) -> Result<(), DeviceError> {
        if let Some(c) = final_color {
            let f = Frame::solid(self.layout.len(), c);
            self.push_frame(&f)?;
        }
        self.connected = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);
    impl Write for SharedBuf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn lifecycle_and_logging() {
        let buf = SharedBuf::default();
        let mut d = VirtualDevice::new("Test Strip", LedLayout::strip(3)).with_sink(buf.clone());
        let log = d.frame_log();

        assert!(matches!(
            d.push_frame(&Frame::solid(3, Rgb::RED)),
            Err(DeviceError::NotConnected)
        ));
        d.connect().unwrap();
        assert_eq!(d.status(), DeviceStatus::Connected);
        assert!(matches!(
            d.push_frame(&Frame::solid(2, Rgb::RED)),
            Err(DeviceError::FrameSize {
                expected: 3,
                got: 2
            })
        ));
        d.push_frame(&Frame::solid(3, Rgb::RED)).unwrap();
        d.disconnect(Some(Rgb::BLACK)).unwrap();

        assert_eq!(log.total(), 2);
        assert_eq!(log.last(), Some(Frame::solid(3, Rgb::BLACK)));
        assert_eq!(d.status(), DeviceStatus::Disconnected);
        assert_eq!(d.info().id.0, "virtual:test-strip");

        let text = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
        assert_eq!(text.lines().count(), 2);
        let first: Frame = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(first, Frame::solid(3, Rgb::RED));
    }
}
