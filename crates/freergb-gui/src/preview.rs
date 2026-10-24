//! Live preview of the selected effect. It renders with the real engine
//! (`EffectSpec::build`, the same code the daemon draws from) into a small
//! canvas, so changing parameters here immediately shows what devices will
//! get — independent of connected hardware or a running daemon.
//!
//! Audio and screen effects render silence/static in the preview; layer
//! effects play through their full stack.

use eframe::egui;
use freergb_core::Canvas;
use freergb_effects::{Effect, EffectSpec};
use std::time::{Duration, Instant};

const WIDTH: usize = 128;
const HEIGHT: usize = 32;
const FRAME: Duration = Duration::from_millis(66);

/// Local effect render + shared texture for the preview image.
pub struct Preview {
    spec: Option<EffectSpec>,
    effect: Option<Box<dyn Effect>>,
    canvas: Canvas,
    texture: Option<egui::TextureHandle>,
    started: Option<Instant>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            spec: None,
            effect: None,
            canvas: Canvas::new(WIDTH, HEIGHT),
            texture: None,
            started: None,
        }
    }
}

impl Preview {
    /// Draws the animated preview and keeps it running by requesting ~15 fps
    /// repaints while visible.
    pub fn show(&mut self, ui: &mut egui::Ui, spec: &EffectSpec) {
        if self.spec.as_ref() != Some(spec) {
            self.effect = Some(spec.build());
            self.spec = Some(spec.clone());
            self.started = Some(Instant::now());
        }
        let mut pixels = Vec::with_capacity(WIDTH * HEIGHT);
        if let (Some(started), Some(effect)) = (self.started, &mut self.effect) {
            effect.render(started.elapsed(), &mut self.canvas);
        }
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let rgb = self.canvas.get(x, y).unwrap_or_default().to_array();
                pixels.push(egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
            }
        }
        let image = egui::ColorImage {
            size: [WIDTH, HEIGHT],
            pixels,
        };
        let texture = self.texture.get_or_insert_with(|| {
            ui.ctx().load_texture(
                "effect-preview",
                image.clone(),
                egui::TextureOptions::LINEAR,
            )
        });
        texture.set(image, egui::TextureOptions::LINEAR);
        let width = ui.available_width().clamp(120.0, 460.0);
        ui.image((
            texture.id(),
            egui::vec2(width, width * HEIGHT as f32 / WIDTH as f32),
        ));
        ui.ctx().request_repaint_after(FRAME);
    }
}
