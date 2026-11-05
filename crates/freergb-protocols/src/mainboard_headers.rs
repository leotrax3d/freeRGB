//! Shared per-header types of the motherboard RGB adapters (`asus_aura`,
//! `asrock_polychrome`, `gigabyte_fusion2`): user colour order and release
//! fallback, and the detection report an adapter fills while it reads the
//! controller's own tables. Wire facts: docs/MAINBOARD_RGB.md.
use freergb_core::Rgb;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::{Arc, Mutex};

/// Order in which an addressable strip reads the three colour bytes.
///
/// `Grb` means the strip takes byte 0 as green, byte 1 as red and byte 2 as
/// blue, so the adapter sends `[g, r, b]`. Reading it off the test pattern:
/// the colours that a red, a green and a blue test LED actually show, in that
/// order, spell the setting (red shows green, green shows red, blue stays
/// blue: `grb`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorOrder {
    #[default]
    Rgb,
    Rbg,
    Grb,
    Gbr,
    Brg,
    Bgr,
}

impl ColorOrder {
    pub const ALL: [ColorOrder; 6] = [
        ColorOrder::Rgb,
        ColorOrder::Rbg,
        ColorOrder::Grb,
        ColorOrder::Gbr,
        ColorOrder::Brg,
        ColorOrder::Bgr,
    ];
    /// Source channel (0 = r, 1 = g, 2 = b) for each wire byte.
    pub const fn channels(self) -> [usize; 3] {
        match self {
            ColorOrder::Rgb => [0, 1, 2],
            ColorOrder::Rbg => [0, 2, 1],
            ColorOrder::Grb => [1, 0, 2],
            ColorOrder::Gbr => [1, 2, 0],
            ColorOrder::Brg => [2, 0, 1],
            ColorOrder::Bgr => [2, 1, 0],
        }
    }
    /// The three wire bytes for `color`.
    pub fn bytes(self, color: Rgb) -> [u8; 3] {
        let rgb = color.to_array();
        self.channels().map(|channel| rgb[channel])
    }
}

/// What an addressable header shows once freeRGB releases it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase", deny_unknown_fields)]
pub enum Fallback {
    /// Firmware spectrum-cycle effect (volatile; the previous effect cannot be read).
    #[default]
    Spectrum,
    /// Firmware "off" effect.
    Off,
    /// One last direct frame in this colour; the header stays in direct mode.
    Static {
        #[serde(serialize_with = "hex_out", deserialize_with = "hex_in")]
        color: Rgb,
    },
}

fn hex_out<S: Serializer>(color: &Rgb, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&color.to_string())
}

fn hex_in<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Rgb, D::Error> {
    let text = String::deserialize(deserializer)?;
    if !text.starts_with('#') {
        return Err(serde::de::Error::custom("expected #rrggbb"));
    }
    text.parse().map_err(serde::de::Error::custom)
}

/// One addressable header as the controller describes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DetectedHeader {
    /// The controller reports this header.
    pub present: bool,
    /// LED count stored in the controller (ASRock Polychrome: the vendor
    /// tool's configured length, not a physical measurement).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_leds: Option<u16>,
    /// Firmware strip-length class limit (Gigabyte RGB Fusion 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_max_leds: Option<u16>,
    /// Largest LED count the adapter accepts for this header.
    pub max_leds: u16,
    /// First layout index of this header in the connected device, if driven.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    /// Driven LEDs (0 = left to the firmware effect).
    pub count: usize,
}

/// What an adapter learned from the controller on its last connect attempt.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HeaderReport {
    /// Non-addressable LEDs (onboard, 12 V RGB headers) driven as fixed slots.
    pub fixed_leds: u16,
    /// Addressable headers in settings order.
    pub headers: Vec<DetectedHeader>,
}

/// Shared slot the daemon reads; adapters overwrite it on every acquire.
pub type ReportCell = Arc<Mutex<Option<HeaderReport>>>;

pub(crate) fn publish(cell: &Option<ReportCell>, report: &HeaderReport) {
    if let Some(cell) = cell {
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(report.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_orders_permute_bytes_and_round_trip_as_lowercase() {
        let color = Rgb::new(1, 2, 3);
        let bytes: Vec<[u8; 3]> = ColorOrder::ALL.iter().map(|o| o.bytes(color)).collect();
        assert_eq!(
            bytes,
            vec![
                [1, 2, 3],
                [1, 3, 2],
                [2, 1, 3],
                [2, 3, 1],
                [3, 1, 2],
                [3, 2, 1]
            ]
        );
        assert_eq!(serde_json::to_string(&ColorOrder::Grb).unwrap(), "\"grb\"");
        assert!(serde_json::from_str::<ColorOrder>("\"GRB\"").is_err());
    }

    #[test]
    fn fallback_json_is_tagged_with_hex_colour() {
        let value = Fallback::Static {
            color: Rgb::new(0xff, 0x88, 0),
        };
        let text = serde_json::to_string(&value).unwrap();
        assert_eq!(text, r##"{"mode":"static","color":"#ff8800"}"##);
        assert_eq!(serde_json::from_str::<Fallback>(&text).unwrap(), value);
        assert_eq!(
            serde_json::from_str::<Fallback>(r#"{"mode":"off"}"#).unwrap(),
            Fallback::Off
        );
        for bad in [
            r#"{"mode":"static"}"#,
            r#"{"mode":"static","color":"ff8800"}"#,
            r##"{"mode":"static","color":"#ff88"}"##,
            r#"{"mode":"blink"}"#,
        ] {
            assert!(serde_json::from_str::<Fallback>(bad).is_err(), "{bad}");
        }
    }
}
