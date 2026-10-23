//! Live preview of the selected effect: the shared effect engine renders
//! into a small canvas, which is uploaded as a top-down 32-bpp DIB and
//! stretched into the window. What you see is what the daemon will draw.

use crate::ffi::*;
use crate::ui::Rect;
use freergb_core::Canvas;
use freergb_effects::{Effect, EffectSpec};
use std::time::Instant;

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 32;

#[derive(Default)]
pub struct Preview {
    spec: Option<EffectSpec>,
    effect: Option<Box<dyn Effect>>,
    canvas: Canvas,
    started: Option<Instant>,
    /// BGRA rows, 32-bpp top-down.
    bits: Vec<u32>,
}

impl Preview {
    /// Renders the current frame for `spec` and stretches it into `dest`
    /// (logical rect) on `dc`.
    pub fn blit(&mut self, dc: HDC, scale: f32, dest: &Rect, spec: &EffectSpec) {
        if self.spec.as_ref() != Some(spec) {
            self.effect = spec.build();
            self.spec = Some(spec.clone());
            self.started = Some(Instant::now());
        }
        if let (Some(started), Some(effect)) = (self.started, &mut self.effect) {
            effect.render(started.elapsed(), &mut self.canvas);
        }
        if self.bits.is_empty() {
            self.bits = vec![0xFF00_0000; WIDTH * HEIGHT];
        }
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let rgb = self.canvas.get(x, y).unwrap_or_default().to_array();
                let pixel = 0xFF00_0000
                    | u32::from(rgb[2]) << 16
                    | u32::from(rgb[1]) << 8
                    | u32::from(rgb[0]);
                self.bits[y * WIDTH + x] = pixel;
            }
        }
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: WIDTH as i32,
            biHeight: -(HEIGHT as i32), // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let info = BITMAPINFO {
            bmiHeader: header,
            bmiColors: [unsafe { std::mem::zeroed() }],
        };
        let s = |v: f32| (v * scale).round() as i32;
        unsafe {
            StretchDIBits(
                dc,
                s(dest.x),
                s(dest.y),
                s(dest.w),
                s(dest.h),
                0,
                0,
                WIDTH as i32,
                HEIGHT as i32,
                self.bits.as_ptr().cast::<core::ffi::c_void>(),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    }
}