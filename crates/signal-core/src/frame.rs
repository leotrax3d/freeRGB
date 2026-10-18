//! A frame: one colour per LED of a device layout, in layout order.

use crate::color::{ColorCorrection, Rgb};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Frame {
    colors: Vec<Rgb>,
}

impl Frame {
    pub fn solid(len: usize, c: Rgb) -> Self {
        Self {
            colors: vec![c; len],
        }
    }

    pub fn from_colors(colors: Vec<Rgb>) -> Self {
        Self { colors }
    }

    pub fn len(&self) -> usize {
        self.colors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    pub fn colors(&self) -> &[Rgb] {
        &self.colors
    }

    pub fn colors_mut(&mut self) -> &mut [Rgb] {
        &mut self.colors
    }

    pub fn corrected(&self, correction: &ColorCorrection) -> Frame {
        if correction.is_identity() {
            return self.clone();
        }
        Frame::from_colors(self.colors.iter().map(|&c| correction.apply(c)).collect())
    }

    /// Simple stable checksum (FNV-1a over RGB bytes) used by tests and
    /// diagnostics to compare frames without dumping them.
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for c in &self.colors {
            for b in c.to_array() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_differs_on_change() {
        let a = Frame::solid(3, Rgb::RED);
        let mut b = a.clone();
        assert_eq!(a.checksum(), b.checksum());
        b.colors_mut()[2] = Rgb::GREEN;
        assert_ne!(a.checksum(), b.checksum());
    }

    #[test]
    fn correction_applies_per_led() {
        let f = Frame::solid(2, Rgb::WHITE);
        let c = ColorCorrection {
            brightness: 0.0,
            ..Default::default()
        };
        assert_eq!(f.corrected(&c), Frame::solid(2, Rgb::BLACK));
    }
}
