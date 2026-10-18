//! The effect canvas: a fixed-resolution RGB image that effects draw into and
//! devices sample from at their LED positions.

use crate::color::Rgb;
use crate::frame::Frame;
use crate::layout::LedLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<Rgb>,
}

impl Canvas {
    /// Creates a black canvas. Width and height are clamped to at least 1.
    pub fn new(width: usize, height: usize) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        Self {
            width,
            height,
            pixels: vec![Rgb::BLACK; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn fill(&mut self, c: Rgb) {
        self.pixels.fill(c);
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Rgb> {
        (x < self.width && y < self.height).then(|| self.pixels[y * self.width + x])
    }

    pub fn set(&mut self, x: usize, y: usize, c: Rgb) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = c;
        }
    }

    /// Calls `f(x_norm, y_norm)` for every pixel centre and stores the result.
    pub fn paint(&mut self, mut f: impl FnMut(f32, f32) -> Rgb) {
        for y in 0..self.height {
            let yn = (y as f32 + 0.5) / self.height as f32;
            for x in 0..self.width {
                let xn = (x as f32 + 0.5) / self.width as f32;
                self.pixels[y * self.width + x] = f(xn, yn);
            }
        }
    }

    /// Nearest-pixel sample at normalised coordinates `0..=1`.
    pub fn sample(&self, xn: f32, yn: f32) -> Rgb {
        let x = ((xn.clamp(0.0, 1.0) * self.width as f32) as usize).min(self.width - 1);
        let y = ((yn.clamp(0.0, 1.0) * self.height as f32) as usize).min(self.height - 1);
        self.pixels[y * self.width + x]
    }

    /// Samples one colour per LED of `layout`, assuming the device spans the
    /// whole canvas. Placement of devices on a shared canvas is handled by the
    /// caller via `region`.
    pub fn sample_layout(&self, layout: &LedLayout, region: Region) -> Frame {
        let colors = (0..layout.len())
            .map(|i| {
                let (lx, ly) = layout.normalized_pos(i).unwrap_or((0.0, 0.0));
                self.sample(region.x + lx * region.w, region.y + ly * region.h)
            })
            .collect();
        Frame::from_colors(colors)
    }
}

/// Normalised rectangle on the canvas that a device occupies.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Region {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Region {
    /// Checks that the rectangle is finite, starts inside the canvas and has a
    /// usable size. Parts reaching past the canvas edge sample the border.
    pub fn is_valid(&self) -> bool {
        [self.x, self.y, self.w, self.h]
            .iter()
            .all(|v| v.is_finite())
            && (0.0..=1.0).contains(&self.x)
            && (0.0..=1.0).contains(&self.y)
            && (0.01..=1.0).contains(&self.w)
            && (0.01..=1.0).contains(&self.h)
    }

    pub const FULL: Region = Region {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };
}

impl Default for Region {
    fn default() -> Self {
        Self::FULL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_corners() {
        let mut c = Canvas::new(4, 2);
        c.set(0, 0, Rgb::RED);
        c.set(3, 1, Rgb::BLUE);
        assert_eq!(c.sample(0.0, 0.0), Rgb::RED);
        assert_eq!(c.sample(1.0, 1.0), Rgb::BLUE);
        assert_eq!(c.get(4, 0), None);
    }

    #[test]
    fn layout_sampling_is_deterministic() {
        let mut c = Canvas::new(8, 1);
        c.paint(|x, _| if x < 0.5 { Rgb::RED } else { Rgb::GREEN });
        let layout = LedLayout::strip(4);
        let f = c.sample_layout(&layout, Region::FULL);
        assert_eq!(f.colors(), &[Rgb::RED, Rgb::RED, Rgb::GREEN, Rgb::GREEN]);

        let left_half = Region {
            w: 0.5,
            ..Region::FULL
        };
        let f = c.sample_layout(&layout, left_half);
        assert!(f.colors().iter().all(|&c| c == Rgb::RED));
    }
}
