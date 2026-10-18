//! A small, downscaled picture of the screen for ambience effects.

use crate::color::Rgb;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScreenFrame {
    pub width: usize,
    pub height: usize,
    /// Row-major, `width * height` pixels.
    pub pixels: Vec<Rgb>,
}

impl ScreenFrame {
    /// Box-filters an RGBA image (`src_w * src_h * 4` bytes) down to
    /// `width x height`. Every output pixel is the mean of its source block.
    pub fn downscale_rgba(
        rgba: &[u8],
        src_w: usize,
        src_h: usize,
        width: usize,
        height: usize,
    ) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        let mut pixels = Vec::with_capacity(width * height);
        if src_w == 0 || src_h == 0 || rgba.len() < src_w * src_h * 4 {
            return Self {
                width,
                height,
                pixels: vec![Rgb::BLACK; width * height],
            };
        }
        for oy in 0..height {
            let y0 = oy * src_h / height;
            let y1 = ((oy + 1) * src_h / height).max(y0 + 1);
            for ox in 0..width {
                let x0 = ox * src_w / width;
                let x1 = ((ox + 1) * src_w / width).max(x0 + 1);
                // Sample every 2nd pixel in large blocks: same mean, half the work.
                let step = if (x1 - x0) * (y1 - y0) > 64 { 2 } else { 1 };
                let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
                for y in (y0..y1).step_by(step) {
                    let row = y * src_w * 4;
                    for x in (x0..x1).step_by(step) {
                        let i = row + x * 4;
                        r += u64::from(rgba[i]);
                        g += u64::from(rgba[i + 1]);
                        b += u64::from(rgba[i + 2]);
                        n += 1;
                    }
                }
                let n = n.max(1);
                pixels.push(Rgb::new((r / n) as u8, (g / n) as u8, (b / n) as u8));
            }
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Nearest pixel at normalised coordinates `0..=1`.
    pub fn sample(&self, xn: f32, yn: f32) -> Rgb {
        if self.pixels.is_empty() {
            return Rgb::BLACK;
        }
        let x = ((xn.clamp(0.0, 1.0) * self.width as f32) as usize).min(self.width - 1);
        let y = ((yn.clamp(0.0, 1.0) * self.height as f32) as usize).min(self.height - 1);
        self.pixels[y * self.width + x]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscale_averages_blocks() {
        // 4x2 image: left half red, right half blue.
        let mut rgba = Vec::new();
        for _y in 0..2 {
            for x in 0..4 {
                rgba.extend_from_slice(if x < 2 {
                    &[255, 0, 0, 255]
                } else {
                    &[0, 0, 255, 255]
                });
            }
        }
        let f = ScreenFrame::downscale_rgba(&rgba, 4, 2, 2, 1);
        assert_eq!(f.pixels, vec![Rgb::RED, Rgb::BLUE]);
        let mixed = ScreenFrame::downscale_rgba(&rgba, 4, 2, 1, 1);
        assert_eq!(mixed.pixels, vec![Rgb::new(127, 0, 127)]);
        assert_eq!(f.sample(0.1, 0.5), Rgb::RED);
        assert_eq!(f.sample(0.9, 0.5), Rgb::BLUE);
    }

    #[test]
    fn bad_input_gives_black_frame() {
        let f = ScreenFrame::downscale_rgba(&[1, 2, 3], 4, 4, 2, 2);
        assert_eq!(f.pixels, vec![Rgb::BLACK; 4]);
        assert_eq!(ScreenFrame::default().sample(0.5, 0.5), Rgb::BLACK);
    }
}
