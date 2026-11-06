//! freeRGB hardware module protocol, version [`MODULE_API`].
//!
//! A hardware module is a separate executable that knows one controller
//! family. The daemon owns every device handle; the module only *asks* the
//! daemon to move reports for the one interface it was assigned. This keeps
//! the process-wide HID serialization intact, needs no device permissions in
//! the module and avoids any shared Rust ABI.
//!
//! Transport: UTF-8 JSON lines over the child's stdin/stdout, one line per
//! message, at most [`MAX_MESSAGE`] bytes. See `docs/HARDWARE_MODULES.md`.
//!
//! * Host request:   `{"v":1,"id":7,"method":"frame","params":{...}}`
//! * Module answer:  `{"v":1,"id":7,"result":...}` or `{"v":1,"id":7,"error":{...}}`
//! * While a request is open the module may issue transport calls:
//!   `{"v":1,"call":3,"op":"send_feature","data":"<base64>"}` which the host
//!   answers with `{"v":1,"call":3,"data":"<base64>","len":91}` or
//!   `{"v":1,"call":3,"error":"..."}`. Nothing else is sent unsolicited.

pub mod bus;
pub mod cooling;
pub mod sdk;

use base64::Engine as _;
use freergb_core::{LedLayout, Rgb};
use serde::{Deserialize, Serialize};

/// Protocol version spoken by this crate. Modules declare the versions they
/// accept in their package manifest; the host refuses any other.
pub const MODULE_API: u32 = 1;
pub const MAX_MESSAGE: usize = 256 * 1024;
pub const MAX_LEDS: usize = 4096;
/// Largest single report a module may move through the host.
pub const MAX_REPORT: usize = 4096;
/// Transport calls allowed while one host request is open.
pub const MAX_CALLS_PER_REQUEST: u32 = 512;
/// Timing extension v1 bounds, enforced by the host.
pub const MIN_TICK_MS: u64 = 500;
pub const MAX_TICK_MS: u64 = 10_000;
pub const MAX_DELAY_MS: u64 = 100;
pub const MAX_SEQUENCE_DELAY_MS: u64 = 1_000;

/// Read-only platform labels supplied by the host. Never contains serials,
/// UUIDs, firmware blobs, registry paths or raw hardware handles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformIdentity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_vendor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
}

/// What the daemon knows about one HID interface. No serial number or path is
/// ever sent to a module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub vid: u16,
    pub pid: u16,
    pub interface: i32,
    pub usage_page: u16,
    pub usage: u16,
    #[serde(default)]
    pub product: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Keyboard,
    Mouse,
    Mousemat,
    Headset,
    HeadsetStand,
    Keypad,
    LightingController,
    Speaker,
    Accessory,
    Laptop,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleInfo {
    pub id: String,
    pub version: String,
    pub family: String,
    pub api: u32,
}

/// Result of `identify`: pure table lookup, no I/O.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identification {
    pub supported: bool,
    pub model: Option<String>,
    pub kind: Option<DeviceKind>,
    /// Human readable reason when unsupported.
    #[serde(default)]
    pub reason: Option<String>,
}

/// How a device can be handed back on shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Restore {
    /// The module restores the firmware's own effect.
    Firmware,
    /// The last colours remain until the device is re-plugged or reconfigured.
    LastFrame,
}

/// Returned by `initialize` once the device answered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capabilities {
    pub name: String,
    pub kind: DeviceKind,
    pub layout: LedLayout,
    #[serde(default)]
    pub firmware: Option<String>,
    pub restore: Restore,
    /// Zones in layout order: name and LED count.
    #[serde(default)]
    pub zones: Vec<Zone>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Zone {
    pub name: String,
    pub leds: usize,
}

/// Machine readable error class, so the GUI can explain a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unsupported,
    Busy,
    Permission,
    Timeout,
    Disconnected,
    Protocol,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleError {
    pub code: ErrorCode,
    pub message: String,
}

impl ModuleError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ModuleError {}

/// Status snapshot for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Status {
    pub ok: bool,
    #[serde(default)]
    pub detail: Option<String>,
}

/// Host -> module request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub v: u32,
    pub id: u64,
    #[serde(flatten)]
    pub method: Method,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Method {
    Hello,
    Identify {
        interface: InterfaceInfo,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        platform: Option<PlatformIdentity>,
    },
    Initialize {
        interface: InterfaceInfo,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        platform: Option<PlatformIdentity>,
        /// Optional user configuration for variable-sized zone families.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        zones: Option<Vec<Zone>>,
    },
    /// RGB bytes in layout order, base64, exactly `3 * leds` bytes.
    Frame {
        rgb: String,
    },
    /// Timing extension v1, sent only after initialize advertises an interval.
    Tick,
    Status,
    Shutdown {
        final_color: Option<Rgb>,
    },
}

/// Transport operations a module may request for its assigned interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Write,
    Read,
    SendFeature,
    GetFeature,
    /// Timing extension v1: timeout_ms is a delay without device I/O.
    Delay,
}

/// Module -> host transport call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Call {
    pub v: u32,
    pub call: u64,
    pub op: Op,
    /// Report bytes (write/send_feature) or the request prefix (get_feature:
    /// first byte = report ID). Empty for `read`.
    #[serde(default)]
    pub data: String,
    /// Buffer length for `read`/`get_feature`.
    #[serde(default)]
    pub len: usize,
    #[serde(default)]
    pub timeout_ms: i32,
}

/// Host -> module answer to a [`Call`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallReply {
    pub v: u32,
    pub call: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub len: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Module -> host final answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub v: u32,
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ModuleError>,
}

pub fn encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn decode(text: &str) -> Result<Vec<u8>, ModuleError> {
    base64::engine::general_purpose::STANDARD
        .decode(text)
        .map_err(|e| ModuleError::new(ErrorCode::Protocol, format!("invalid base64: {e}")))
}

/// Packs colours as `r g b` bytes, the frame wire format.
pub fn pack_rgb(colors: &[Rgb]) -> String {
    let mut bytes = Vec::with_capacity(colors.len() * 3);
    for c in colors {
        bytes.extend_from_slice(&[c.r, c.g, c.b]);
    }
    encode(&bytes)
}

pub fn unpack_rgb(text: &str, leds: usize) -> Result<Vec<Rgb>, ModuleError> {
    let bytes = decode(text)?;
    if bytes.len() != leds * 3 {
        return Err(ModuleError::new(
            ErrorCode::Protocol,
            format!("frame has {} bytes, expected {}", bytes.len(), leds * 3),
        ));
    }
    Ok(bytes
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| Rgb {
            r: c[0],
            g: c[1],
            b: c[2],
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_platform_is_backward_compatible_and_contains_only_labels() {
        let old = r#"{"v":1,"id":1,"method":"identify","params":{"interface":{"vid":1,"pid":2,"interface":0,"usage_page":1,"usage":2}}}"#;
        let request: Request = serde_json::from_str(old).unwrap();
        assert!(matches!(
            request.method,
            Method::Identify { platform: None, .. }
        ));
        let mut wire: serde_json::Value = serde_json::from_str(old).unwrap();
        wire["params"]["platform"] = serde_json::json!({
            "board_vendor": "Fixture Vendor", "board_name": "Fixture Board A", "product": "Fixture Product"
        });
        let request: Request = serde_json::from_value(wire.clone()).unwrap();
        let Method::Identify {
            platform: Some(platform),
            ..
        } = request.method
        else {
            panic!("missing metadata")
        };
        assert_eq!(platform.board_name.as_deref(), Some("Fixture Board A"));
        assert_eq!(
            serde_json::to_value(platform)
                .unwrap()
                .as_object()
                .unwrap()
                .len(),
            3
        );
        // The previous API-1 params shape ignores additive fields using the
        // same serde behaviour as already shipped modules.
        #[derive(Deserialize)]
        struct LegacyParams {
            interface: InterfaceInfo,
        }
        let legacy: LegacyParams = serde_json::from_value(wire["params"].clone()).unwrap();
        assert_eq!(legacy.interface.pid, 2);
        wire["params"]["platform"]["board_name"] = serde_json::json!(7);
        assert!(serde_json::from_value::<Request>(wire).is_err());
    }

    #[test]
    fn methods_round_trip_with_stable_wire_names() {
        let request = Request {
            v: MODULE_API,
            id: 3,
            method: Method::Frame {
                rgb: pack_rgb(&[Rgb { r: 1, g: 2, b: 3 }]),
            },
        };
        let text = serde_json::to_string(&request).unwrap();
        assert_eq!(
            text,
            r#"{"v":1,"id":3,"method":"frame","params":{"rgb":"AQID"}}"#
        );
        let back: Request = serde_json::from_str(&text).unwrap();
        assert_eq!(back.method, request.method);
        let hello: Request = serde_json::from_str(r#"{"v":1,"id":1,"method":"hello"}"#).unwrap();
        assert_eq!(hello.method, Method::Hello);
        let initialize = Request {
            v: MODULE_API,
            id: 4,
            method: Method::Initialize {
                platform: None,
                interface: InterfaceInfo {
                    vid: 1,
                    pid: 2,
                    interface: 0,
                    usage_page: 1,
                    usage: 1,
                    product: None,
                },
                zones: Some(vec![Zone {
                    name: "Header 1".into(),
                    leds: 12,
                }]),
            },
        };
        let text = serde_json::to_string(&initialize).unwrap();
        assert!(text.contains("\"zones\":[{\"name\":\"Header 1\",\"leds\":12}]"));
        let back: Request = serde_json::from_str(&text).unwrap();
        assert_eq!(back.method, initialize.method);
    }

    #[test]
    fn frames_must_match_led_count() {
        let text = pack_rgb(&[Rgb { r: 9, g: 8, b: 7 }; 2]);
        assert_eq!(unpack_rgb(&text, 2).unwrap()[1], Rgb { r: 9, g: 8, b: 7 });
        assert!(unpack_rgb(&text, 3).is_err());
        assert!(unpack_rgb("***", 1).is_err());
    }
}
