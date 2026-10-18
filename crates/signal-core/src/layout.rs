//! LED layouts: named LEDs at integer grid positions inside a device rectangle.
//!
//! A layout describes *where* LEDs are, independent of how a protocol addresses
//! them. Protocols keep their own mapping from layout index to wire index.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Led {
    pub name: String,
    /// Grid position `(x, y)` with `x < width`, `y < height`.
    pub pos: (u16, u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedLayout {
    pub width: u16,
    pub height: u16,
    pub leds: Vec<Led>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    #[error("layout has zero width or height")]
    EmptyGrid,
    #[error("LED {index} ({name}) at {pos:?} lies outside the {width}x{height} grid")]
    OutOfBounds {
        index: usize,
        name: String,
        pos: (u16, u16),
        width: u16,
        height: u16,
    },
    #[error("LED name {0:?} is used more than once")]
    DuplicateName(String),
}

impl LedLayout {
    /// Creates and validates a layout.
    pub fn new(width: u16, height: u16, leds: Vec<Led>) -> Result<Self, LayoutError> {
        let layout = Self {
            width,
            height,
            leds,
        };
        layout.validate()?;
        Ok(layout)
    }

    /// A single row of `count` LEDs, as used by strips and fans.
    pub fn strip(count: u16) -> Self {
        let leds = (0..count)
            .map(|i| Led {
                name: format!("LED {}", i + 1),
                pos: (i, 0),
            })
            .collect();
        Self {
            width: count.max(1),
            height: 1,
            leds,
        }
    }

    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.width == 0 || self.height == 0 {
            return Err(LayoutError::EmptyGrid);
        }
        let mut seen = std::collections::HashSet::new();
        for (index, led) in self.leds.iter().enumerate() {
            if led.pos.0 >= self.width || led.pos.1 >= self.height {
                return Err(LayoutError::OutOfBounds {
                    index,
                    name: led.name.clone(),
                    pos: led.pos,
                    width: self.width,
                    height: self.height,
                });
            }
            if !seen.insert(led.name.as_str()) {
                return Err(LayoutError::DuplicateName(led.name.clone()));
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.leds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.leds.is_empty()
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.leds.iter().position(|l| l.name == name)
    }

    /// Normalised LED centre in `0..=1` on both axes (cell centres).
    pub fn normalized_pos(&self, index: usize) -> Option<(f32, f32)> {
        let led = self.leds.get(index)?;
        Some((
            (f32::from(led.pos.0) + 0.5) / f32::from(self.width),
            (f32::from(led.pos.1) + 0.5) / f32::from(self.height),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn led(name: &str, x: u16, y: u16) -> Led {
        Led {
            name: name.into(),
            pos: (x, y),
        }
    }

    #[test]
    fn strip_is_valid() {
        let s = LedLayout::strip(8);
        assert_eq!(s.len(), 8);
        s.validate().unwrap();
        assert_eq!(s.normalized_pos(0), Some((0.5 / 8.0, 0.5)));
    }

    #[test]
    fn rejects_out_of_bounds_and_duplicates() {
        assert!(matches!(
            LedLayout::new(2, 1, vec![led("a", 2, 0)]),
            Err(LayoutError::OutOfBounds { index: 0, .. })
        ));
        assert!(matches!(
            LedLayout::new(2, 1, vec![led("a", 0, 0), led("a", 1, 0)]),
            Err(LayoutError::DuplicateName(_))
        ));
        assert_eq!(LedLayout::new(0, 1, vec![]), Err(LayoutError::EmptyGrid));
    }

    #[test]
    fn serde_round_trip() {
        let l = LedLayout::new(3, 2, vec![led("x", 0, 0), led("y", 2, 1)]).unwrap();
        let json = serde_json::to_string(&l).unwrap();
        let back: LedLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(l, back);
        assert_eq!(back.index_of("y"), Some(1));
    }
}
