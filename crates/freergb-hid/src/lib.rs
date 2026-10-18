//! HID discovery and transport via hidapi (native backends on Windows and
//! Linux, no C toolchain dependency for hidapi itself).

pub mod descriptor;

use descriptor::ReportSizes;
use freergb_device_api::{HidTransport, TransportError};
use serde::Serialize;
use std::ffi::CString;

/// One HID interface (Windows: top-level collection) as enumerated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HidInterfaceInfo {
    pub vid: u16,
    pub pid: u16,
    pub interface: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum HidError {
    #[error("hidapi: {0}")]
    Api(#[from] hidapi::HidError),
    #[error("interface not found: {0}")]
    NotFound(String),
    #[error("could not determine report sizes: {0}")]
    ReportSizes(String),
}

/// Enumerates all HID interfaces. Read-only.
pub fn enumerate() -> Result<Vec<HidInterfaceInfo>, HidError> {
    let api = hidapi::HidApi::new()?;
    let mut v: Vec<_> = api
        .device_list()
        .map(|d| HidInterfaceInfo {
            vid: d.vendor_id(),
            pid: d.product_id(),
            interface: d.interface_number(),
            usage_page: d.usage_page(),
            usage: d.usage(),
            manufacturer: d.manufacturer_string().map(str::to_owned),
            product: d.product_string().map(str::to_owned),
            serial: d
                .serial_number()
                .map(str::to_owned)
                .filter(|s| !s.is_empty()),
            path: d.path().to_string_lossy().into_owned(),
        })
        .collect();
    v.sort_by(|a, b| {
        (a.vid, a.pid, a.interface, a.usage_page, a.usage).cmp(&(
            b.vid,
            b.pid,
            b.interface,
            b.usage_page,
            b.usage,
        ))
    });
    Ok(v)
}

/// Finds the first interface with the given VID/PID/usage page/usage.
pub fn find(vid: u16, pid: u16, usage_page: u16, usage: u16) -> Result<HidInterfaceInfo, HidError> {
    enumerate()?
        .into_iter()
        .find(|i| i.vid == vid && i.pid == pid && i.usage_page == usage_page && i.usage == usage)
        .ok_or_else(|| {
            HidError::NotFound(format!(
                "{vid:04x}:{pid:04x} up {usage_page:#06x} u {usage:#06x}"
            ))
        })
}

/// An opened HID interface implementing [`HidTransport`].
pub struct HidapiTransport {
    dev: hidapi::HidDevice,
    sizes: ReportSizes,
}

impl HidapiTransport {
    /// Opens `info.path` and derives report lengths from the report descriptor.
    pub fn open(info: &HidInterfaceInfo) -> Result<Self, HidError> {
        let api = hidapi::HidApi::new()?;
        let path =
            CString::new(info.path.clone()).map_err(|e| HidError::NotFound(e.to_string()))?;
        let dev = api.open_path(&path)?;
        let mut buf = vec![0u8; hidapi::MAX_REPORT_DESCRIPTOR_SIZE];
        let n = dev.get_report_descriptor(&mut buf)?;
        let sizes = descriptor::parse_report_sizes(&buf[..n]).map_err(HidError::ReportSizes)?;
        tracing::debug!(path = %info.path, ?sizes, "opened HID interface");
        Ok(Self { dev, sizes })
    }

    pub fn report_sizes(&self) -> ReportSizes {
        self.sizes
    }
}

fn io(e: hidapi::HidError) -> TransportError {
    TransportError::Io(e.to_string())
}

impl HidTransport for HidapiTransport {
    fn write(&mut self, report: &[u8]) -> Result<usize, TransportError> {
        let len = self.output_report_len();
        if report.len() > len {
            return Err(TransportError::Io(format!(
                "report {} > output length {len}",
                report.len()
            )));
        }
        let mut buf = report.to_vec();
        buf.resize(len, 0);
        self.dev.write(&buf).map_err(io)
    }

    fn read_timeout(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, TransportError> {
        self.dev.read_timeout(buf, timeout_ms).map_err(io)
    }

    fn output_report_len(&self) -> usize {
        self.sizes.output_with_id()
    }

    fn input_report_len(&self) -> usize {
        self.sizes.input_with_id()
    }
}
