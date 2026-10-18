//! Layered custom effects: the building block of the effect editor.
//!
//! A custom effect is a stack of built-in effects. Each layer renders over the
//! whole canvas (so patterns line up across layers) and is then blended onto
//! the result below it – optionally only inside a region.

use crate::{Effect, EffectContext, EffectSpec};
use freergb_core::{Canvas, Region, Rgb};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MAX_LAYERS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    /// Layer covers what is below (respecting opacity).
    #[default]
    Normal,
    /// Channels are added (lighten, saturates at 255).
    Add,
    /// Channels are multiplied (darken / tint).
    Multiply,
    /// Inverse multiply (soft lighten).
    Screen,
    /// Per-channel maximum.
    Max,
}

impl BlendMode {
    pub const ALL: [BlendMode; 5] = [
        BlendMode::Normal,
        BlendMode::Add,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Max,
    ];

    fn channel(self, below: u8, above: u8) -> u8 {
        let (a, b) = (u16::from(below), u16::from(above));
        match self {
            BlendMode::Normal => above,
            BlendMode::Add => (a + b).min(255) as u8,
            BlendMode::Multiply => (a * b / 255) as u8,
            BlendMode::Screen => (255 - (255 - a) * (255 - b) / 255) as u8,
            BlendMode::Max => below.max(above),
        }
    }

    pub fn blend(self, below: Rgb, above: Rgb, opacity: f32) -> Rgb {
        let mixed = Rgb::new(
            self.channel(below.r, above.r),
            self.channel(below.g, above.g),
            self.channel(below.b, above.b),
        );
        below.lerp(mixed, opacity)
    }
}

fn default_opacity() -> f32 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub effect: EffectSpec,
    #[serde(default)]
    pub blend: BlendMode,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    /// Restricts the layer to this part of the canvas; `None` = everywhere.
    #[serde(default)]
    pub region: Option<Region>,
}

impl Layer {
    pub fn new(effect: EffectSpec) -> Self {
        Self {
            effect,
            blend: BlendMode::Normal,
            opacity: 1.0,
            region: None,
        }
    }
}

/// Checks a layer stack: 1..=MAX_LAYERS layers, no nested layer effects,
/// opacity in 0..=1, valid regions.
pub fn validate(layers: &[Layer]) -> Result<(), String> {
    if layers.is_empty() || layers.len() > MAX_LAYERS {
        return Err(format!(
            "a custom effect needs 1..={MAX_LAYERS} layers, got {}",
            layers.len()
        ));
    }
    for (i, l) in layers.iter().enumerate() {
        if matches!(l.effect, EffectSpec::Layers { .. }) {
            return Err(format!("layer {i}: layer effects cannot be nested"));
        }
        if !(0.0..=1.0).contains(&l.opacity) {
            return Err(format!("layer {i}: opacity must be 0..=1"));
        }
        if l.region.is_some_and(|r| !r.is_valid()) {
            return Err(format!("layer {i}: invalid region"));
        }
    }
    Ok(())
}

pub struct Layered {
    layers: Vec<(Box<dyn Effect>, Layer)>,
    scratch: Option<Canvas>,
}

impl Layered {
    /// Builds the stack; invalid stacks render black (the API validates first).
    pub fn new(layers: &[Layer], ctx: &EffectContext) -> Self {
        let layers = if validate(layers).is_ok() {
            layers
                .iter()
                .map(|l| (l.effect.build_with(ctx), l.clone()))
                .collect()
        } else {
            Vec::new()
        };
        Self {
            layers,
            scratch: None,
        }
    }
}

impl Effect for Layered {
    fn name(&self) -> &str {
        "Custom"
    }

    fn render(&mut self, t: Duration, canvas: &mut Canvas) {
        canvas.fill(Rgb::BLACK);
        let (w, h) = (canvas.width(), canvas.height());
        let scratch = self.scratch.get_or_insert_with(|| Canvas::new(w, h));
        if scratch.width() != w || scratch.height() != h {
            *scratch = Canvas::new(w, h);
        }
        for (effect, layer) in &mut self.layers {
            effect.render(t, scratch);
            let region = layer.region.unwrap_or(Region::FULL);
            let opacity = layer.opacity.clamp(0.0, 1.0);
            for y in 0..h {
                let yn = (y as f32 + 0.5) / h as f32;
                if yn < region.y || yn > region.y + region.h {
                    continue;
                }
                for x in 0..w {
                    let xn = (x as f32 + 0.5) / w as f32;
                    if xn < region.x || xn > region.x + region.w {
                        continue;
                    }
                    let below = canvas.get(x, y).unwrap_or(Rgb::BLACK);
                    let above = scratch.get(x, y).unwrap_or(Rgb::BLACK);
                    canvas.set(x, y, layer.blend.blend(below, above, opacity));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(c: Rgb) -> EffectSpec {
        EffectSpec::Solid { color: c }
    }

    fn render(layers: Vec<Layer>) -> Canvas {
        let mut c = Canvas::new(10, 10);
        Layered::new(&layers, &EffectContext::default()).render(Duration::ZERO, &mut c);
        c
    }

    #[test]
    fn blend_modes() {
        let (r, b) = (Rgb::new(200, 0, 0), Rgb::new(100, 0, 200));
        assert_eq!(BlendMode::Normal.blend(r, b, 1.0), b);
        assert_eq!(BlendMode::Normal.blend(r, b, 0.0), r);
        assert_eq!(BlendMode::Add.blend(r, b, 1.0), Rgb::new(255, 0, 200));
        assert_eq!(BlendMode::Multiply.blend(Rgb::WHITE, b, 1.0), b);
        assert_eq!(BlendMode::Multiply.blend(r, b, 1.0), Rgb::new(78, 0, 0));
        assert_eq!(BlendMode::Screen.blend(Rgb::BLACK, b, 1.0), b);
        assert_eq!(BlendMode::Max.blend(r, b, 1.0), Rgb::new(200, 0, 200));
    }

    #[test]
    fn regions_and_opacity() {
        let mut top = Layer::new(solid(Rgb::RED));
        top.region = Some(Region {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 0.5,
        });
        let mut half_blue = Layer::new(solid(Rgb::BLUE));
        half_blue.opacity = 0.5;
        half_blue.region = Some(Region {
            x: 0.0,
            y: 0.5,
            w: 1.0,
            h: 0.5,
        });
        let c = render(vec![Layer::new(solid(Rgb::WHITE)), top, half_blue]);
        assert_eq!(c.sample(0.5, 0.1), Rgb::RED, "top half red");
        assert_eq!(
            c.sample(0.5, 0.9),
            Rgb::new(128, 128, 255),
            "white + 50 % blue"
        );
    }

    #[test]
    fn validation_rejects_bad_stacks() {
        assert!(validate(&[]).is_err());
        assert!(validate(&vec![Layer::new(solid(Rgb::RED)); MAX_LAYERS + 1]).is_err());
        let nested = Layer::new(EffectSpec::Layers {
            layers: vec![Layer::new(solid(Rgb::RED))],
        });
        assert!(validate(&[nested]).unwrap_err().contains("nested"));
        let mut bad = Layer::new(solid(Rgb::RED));
        bad.opacity = 1.5;
        assert!(validate(&[bad]).is_err());
        assert!(validate(&[Layer::new(solid(Rgb::RED))]).is_ok());
        // Invalid stacks render black instead of panicking.
        assert_eq!(render(vec![]).sample(0.5, 0.5), Rgb::BLACK);
    }

    #[test]
    fn layered_spec_round_trips_and_defaults_apply() {
        let json = r#"{"type":"layers","layers":[{"effect":{"type":"solid","color":{"r":1,"g":2,"b":3}}}]}"#;
        let spec: EffectSpec = serde_json::from_str(json).unwrap();
        let EffectSpec::Layers { layers } = &spec else {
            panic!("expected layers")
        };
        assert_eq!(layers[0].blend, BlendMode::Normal);
        assert_eq!(layers[0].opacity, 1.0);
        let again: EffectSpec =
            serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
        assert_eq!(again, spec);
    }
}
