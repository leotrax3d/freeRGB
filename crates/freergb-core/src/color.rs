//! RGB and HSV colour types plus brightness and colour correction.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// An 8-bit-per-channel sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);
    pub const RED: Rgb = Rgb::new(255, 0, 0);
    pub const GREEN: Rgb = Rgb::new(0, 255, 0);
    pub const BLUE: Rgb = Rgb::new(0, 0, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Scales all channels by `factor` (clamped to `0.0..=1.0`), rounding to nearest.
    pub fn scaled(self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        let s = |c: u8| (f32::from(c) * f).round() as u8;
        Self::new(s(self.r), s(self.g), s(self.b))
    }

    /// Linear interpolation between `self` (t = 0) and `other` (t = 1).
    pub fn lerp(self, other: Rgb, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let l = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
        Self::new(l(self.r, other.r), l(self.g, other.g), l(self.b, other.b))
    }

    pub fn to_hsv(self) -> Hsv {
        let r = f32::from(self.r) / 255.0;
        let g = f32::from(self.g) / 255.0;
        let b = f32::from(self.b) / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        let s = if max == 0.0 { 0.0 } else { delta / max };
        Hsv::new(h, s, max)
    }

    pub fn to_array(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

impl From<[u8; 3]> for Rgb {
    fn from(v: [u8; 3]) -> Self {
        Self::new(v[0], v[1], v[2])
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Error for parsing `#rrggbb` strings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid hex colour {0:?}, expected #rrggbb")]
pub struct ParseRgbError(pub String);

impl FromStr for Rgb {
    type Err = ParseRgbError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if hex.len() != 6 || !hex.is_ascii() {
            return Err(ParseRgbError(s.to_owned()));
        }
        let p = |i: usize| {
            u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ParseRgbError(s.to_owned()))
        };
        Ok(Self::new(p(0)?, p(2)?, p(4)?))
    }
}

/// Hue (degrees, `0..360`), saturation and value (`0..=1`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    pub fn new(h: f32, s: f32, v: f32) -> Self {
        Self {
            h: h.rem_euclid(360.0),
            s: s.clamp(0.0, 1.0),
            v: v.clamp(0.0, 1.0),
        }
    }

    pub fn to_rgb(self) -> Rgb {
        let c = self.v * self.s;
        let hp = self.h / 60.0;
        let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
        let (r, g, b) = match hp as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = self.v - c;
        let to8 = |f: f32| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        Rgb::new(to8(r), to8(g), to8(b))
    }
}

/// Per-device output correction applied after effects are rendered.
///
/// Order: per-channel gain (white balance) → global brightness → gamma.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorCorrection {
    /// Global brightness `0..=1`.
    pub brightness: f32,
    /// Per-channel gain `0..=1` for white balance.
    pub gain: [f32; 3],
    /// Output gamma; `1.0` disables gamma correction.
    pub gamma: f32,
}

impl Default for ColorCorrection {
    fn default() -> Self {
        Self {
            brightness: 1.0,
            gain: [1.0; 3],
            gamma: 1.0,
        }
    }
}

impl ColorCorrection {
    pub fn apply(&self, c: Rgb) -> Rgb {
        let b = self.brightness.clamp(0.0, 1.0);
        let ch = |v: u8, gain: f32| {
            let lin = f32::from(v) / 255.0 * gain.clamp(0.0, 1.0) * b;
            let out = if (self.gamma - 1.0).abs() < f32::EPSILON {
                lin
            } else {
                lin.powf(self.gamma.max(0.01))
            };
            (out * 255.0).round().clamp(0.0, 255.0) as u8
        };
        Rgb::new(
            ch(c.r, self.gain[0]),
            ch(c.g, self.gain[1]),
            ch(c.b, self.gain[2]),
        )
    }

    pub fn is_identity(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_primary_round_trip() {
        for (rgb, h) in [(Rgb::RED, 0.0), (Rgb::GREEN, 120.0), (Rgb::BLUE, 240.0)] {
            let hsv = rgb.to_hsv();
            assert!((hsv.h - h).abs() < 0.01, "{rgb} -> {hsv:?}");
            assert_eq!(hsv.to_rgb(), rgb);
        }
    }

    #[test]
    fn hsv_round_trip_exhaustive_sample() {
        for r in (0..=255).step_by(17) {
            for g in (0..=255).step_by(17) {
                for b in (0..=255).step_by(17) {
                    let c = Rgb::new(r as u8, g as u8, b as u8);
                    assert_eq!(c.to_hsv().to_rgb(), c);
                }
            }
        }
    }

    #[test]
    fn hue_wraps() {
        assert_eq!(Hsv::new(360.0, 1.0, 1.0).to_rgb(), Rgb::RED);
        assert_eq!(Hsv::new(-120.0, 1.0, 1.0).to_rgb(), Rgb::BLUE);
    }

    #[test]
    fn parse_and_display_hex() {
        let c: Rgb = "#009bde".parse().unwrap();
        assert_eq!(c, Rgb::new(0x00, 0x9b, 0xde));
        assert_eq!(c.to_string(), "#009bde");
        assert!("#12345".parse::<Rgb>().is_err());
        assert!("#gg0000".parse::<Rgb>().is_err());
    }

    #[test]
    fn scaling_and_lerp() {
        assert_eq!(Rgb::WHITE.scaled(0.5), Rgb::new(128, 128, 128));
        assert_eq!(Rgb::WHITE.scaled(2.0), Rgb::WHITE);
        assert_eq!(Rgb::BLACK.lerp(Rgb::WHITE, 0.5), Rgb::new(128, 128, 128));
        assert_eq!(Rgb::RED.lerp(Rgb::BLUE, 1.0), Rgb::BLUE);
    }

    #[test]
    fn correction_identity_and_brightness() {
        let id = ColorCorrection::default();
        assert!(id.is_identity());
        assert_eq!(id.apply(Rgb::new(10, 200, 77)), Rgb::new(10, 200, 77));

        let half = ColorCorrection {
            brightness: 0.5,
            ..Default::default()
        };
        assert_eq!(half.apply(Rgb::WHITE), Rgb::new(128, 128, 128));

        let gain = ColorCorrection {
            gain: [1.0, 0.0, 0.5],
            ..Default::default()
        };
        assert_eq!(gain.apply(Rgb::WHITE), Rgb::new(255, 0, 128));
    }
}
