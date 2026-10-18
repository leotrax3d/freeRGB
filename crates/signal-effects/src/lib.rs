//! Built-in effects. Effects are pure functions of time onto the canvas, which
//! keeps them deterministic and testable: the same `t` always produces the
//! same pixels.

use serde::{Deserialize, Serialize};
use signal_core::{Canvas, Hsv, Rgb};
use std::time::Duration;

pub trait Effect: Send {
    fn name(&self) -> &str;
    /// Renders the state at time `t` since the effect started.
    fn render(&mut self, t: Duration, canvas: &mut Canvas);
}

/// Serializable effect selection, used by profiles and the local API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EffectSpec {
    Solid { color: Rgb },
    RainbowWave { speed: f32, scale: f32 },
    Breathing { color: Rgb, period_ms: u32 },
}

impl EffectSpec {
    pub fn build(&self) -> Box<dyn Effect> {
        match *self {
            EffectSpec::Solid { color } => Box::new(Solid { color }),
            EffectSpec::RainbowWave { speed, scale } => Box::new(RainbowWave { speed, scale }),
            EffectSpec::Breathing { color, period_ms } => Box::new(Breathing {
                color,
                period: Duration::from_millis(u64::from(period_ms.max(1))),
            }),
        }
    }
}

pub struct Solid {
    pub color: Rgb,
}

impl Effect for Solid {
    fn name(&self) -> &str {
        "Solid"
    }

    fn render(&mut self, _t: Duration, canvas: &mut Canvas) {
        canvas.fill(self.color);
    }
}

/// Horizontal hue gradient scrolling to the right.
pub struct RainbowWave {
    /// Canvas widths per second.
    pub speed: f32,
    /// Number of full hue cycles across the canvas width.
    pub scale: f32,
}

impl Effect for RainbowWave {
    fn name(&self) -> &str {
        "Rainbow Wave"
    }

    fn render(&mut self, t: Duration, canvas: &mut Canvas) {
        let offset = t.as_secs_f32() * self.speed;
        let scale = self.scale;
        canvas.paint(|x, _| Hsv::new((x * scale - offset) * 360.0, 1.0, 1.0).to_rgb());
    }
}

/// Sine-shaped fade between black and `color`.
pub struct Breathing {
    pub color: Rgb,
    pub period: Duration,
}

impl Effect for Breathing {
    fn name(&self) -> &str {
        "Breathing"
    }

    fn render(&mut self, t: Duration, canvas: &mut Canvas) {
        let phase = (t.as_secs_f64() / self.period.as_secs_f64()).fract();
        let level = 0.5 - 0.5 * (phase * std::f64::consts::TAU).cos();
        canvas.fill(self.color.scaled(level as f32));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_fills() {
        let mut c = Canvas::new(4, 4);
        Solid { color: Rgb::GREEN }.render(Duration::ZERO, &mut c);
        assert_eq!(c.sample(0.3, 0.9), Rgb::GREEN);
    }

    #[test]
    fn breathing_bounds() {
        let mut c = Canvas::new(1, 1);
        let mut e = Breathing {
            color: Rgb::WHITE,
            period: Duration::from_secs(2),
        };
        e.render(Duration::ZERO, &mut c);
        assert_eq!(c.get(0, 0), Some(Rgb::BLACK));
        e.render(Duration::from_secs(1), &mut c);
        assert_eq!(c.get(0, 0), Some(Rgb::WHITE));
    }

    #[test]
    fn rainbow_is_deterministic_and_moves() {
        let spec = EffectSpec::RainbowWave {
            speed: 0.5,
            scale: 1.0,
        };
        let (mut a, mut b) = (Canvas::new(16, 1), Canvas::new(16, 1));
        spec.build().render(Duration::from_millis(250), &mut a);
        spec.build().render(Duration::from_millis(250), &mut b);
        assert_eq!(a, b);
        let mut later = Canvas::new(16, 1);
        spec.build().render(Duration::from_millis(500), &mut later);
        assert_ne!(a, later);
    }

    #[test]
    fn spec_serde() {
        let s = EffectSpec::Breathing {
            color: Rgb::RED,
            period_ms: 3000,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"type\":\"breathing\""));
        assert_eq!(serde_json::from_str::<EffectSpec>(&json).unwrap(), s);
    }
}
