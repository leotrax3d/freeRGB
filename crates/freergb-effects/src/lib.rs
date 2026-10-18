//! Built-in effects. Effects are pure functions of time onto the canvas, which
//! keeps them deterministic and testable: the same `t` always produces the
//! same pixels.

pub mod audio;
pub mod extra;
pub mod layers;
pub mod screen;

pub use audio::{AudioHandle, EffectContext, ScreenHandle};

use freergb_core::{Canvas, Hsv, Rgb};
use serde::{Deserialize, Serialize};
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
    Solid {
        color: Rgb,
    },
    RainbowWave {
        speed: f32,
        scale: f32,
    },
    Breathing {
        color: Rgb,
        period_ms: u32,
    },
    Gradient {
        from: Rgb,
        to: Rgb,
        angle_deg: f32,
    },
    ColorCycle {
        period_ms: u32,
    },
    RadialRainbow {
        speed: f32,
        scale: f32,
    },
    Scanner {
        color: Rgb,
        background: Rgb,
        period_ms: u32,
        width: f32,
    },
    Twinkle {
        color: Rgb,
        background: Rgb,
        density: f32,
        period_ms: u32,
    },
    AudioPulse {
        color: Rgb,
        background: Rgb,
        sensitivity: f32,
    },
    AudioSpectrum {
        low: Rgb,
        high: Rgb,
        background: Rgb,
    },
    ScreenAmbience {
        saturation: f32,
    },
    /// Custom effect from the editor: a stack of built-in effects.
    Layers {
        layers: Vec<layers::Layer>,
    },
}

fn ms(v: u32) -> Duration {
    Duration::from_millis(u64::from(v.max(1)))
}

impl EffectSpec {
    /// API names of all effects, in display order.
    pub const NAMES: [&'static str; 12] = [
        "solid",
        "rainbow_wave",
        "breathing",
        "gradient",
        "color_cycle",
        "radial_rainbow",
        "scanner",
        "twinkle",
        "audio_pulse",
        "audio_spectrum",
        "screen_ambience",
        "layers",
    ];

    /// A sensible default instance for each name in [`EffectSpec::NAMES`].
    pub fn default_for(name: &str) -> Option<Self> {
        let accent = Rgb::new(0, 155, 222);
        Some(match name {
            "solid" => EffectSpec::Solid { color: accent },
            "rainbow_wave" => EffectSpec::RainbowWave {
                speed: 0.2,
                scale: 1.0,
            },
            "breathing" => EffectSpec::Breathing {
                color: accent,
                period_ms: 3000,
            },
            "gradient" => EffectSpec::Gradient {
                from: Rgb::new(255, 0, 128),
                to: accent,
                angle_deg: 0.0,
            },
            "color_cycle" => EffectSpec::ColorCycle { period_ms: 10_000 },
            "radial_rainbow" => EffectSpec::RadialRainbow {
                speed: 0.3,
                scale: 1.0,
            },
            "scanner" => EffectSpec::Scanner {
                color: Rgb::RED,
                background: Rgb::BLACK,
                period_ms: 2000,
                width: 0.12,
            },
            "twinkle" => EffectSpec::Twinkle {
                color: Rgb::WHITE,
                background: Rgb::new(0, 0, 40),
                density: 0.15,
                period_ms: 1200,
            },
            "audio_pulse" => EffectSpec::AudioPulse {
                color: accent,
                background: Rgb::BLACK,
                sensitivity: 1.5,
            },
            "audio_spectrum" => EffectSpec::AudioSpectrum {
                low: Rgb::new(255, 0, 80),
                high: accent,
                background: Rgb::BLACK,
            },
            "screen_ambience" => EffectSpec::ScreenAmbience { saturation: 1.4 },
            "layers" => EffectSpec::Layers {
                layers: vec![
                    layers::Layer::new(EffectSpec::RainbowWave {
                        speed: 0.2,
                        scale: 1.0,
                    }),
                    layers::Layer {
                        effect: EffectSpec::Scanner {
                            color: Rgb::WHITE,
                            background: Rgb::BLACK,
                            period_ms: 2000,
                            width: 0.1,
                        },
                        blend: layers::BlendMode::Add,
                        opacity: 0.8,
                        region: None,
                    },
                ],
            },
            _ => return None,
        })
    }

    /// The API name of this effect.
    pub fn name(&self) -> &'static str {
        match self {
            EffectSpec::Solid { .. } => "solid",
            EffectSpec::RainbowWave { .. } => "rainbow_wave",
            EffectSpec::Breathing { .. } => "breathing",
            EffectSpec::Gradient { .. } => "gradient",
            EffectSpec::ColorCycle { .. } => "color_cycle",
            EffectSpec::RadialRainbow { .. } => "radial_rainbow",
            EffectSpec::Scanner { .. } => "scanner",
            EffectSpec::Twinkle { .. } => "twinkle",
            EffectSpec::AudioPulse { .. } => "audio_pulse",
            EffectSpec::AudioSpectrum { .. } => "audio_spectrum",
            EffectSpec::ScreenAmbience { .. } => "screen_ambience",
            EffectSpec::Layers { .. } => "layers",
        }
    }

    /// Whether this effect needs the system audio (the daemon only captures
    /// audio while such an effect is active).
    pub fn needs_audio(&self) -> bool {
        match self {
            EffectSpec::AudioPulse { .. } | EffectSpec::AudioSpectrum { .. } => true,
            EffectSpec::Layers { layers } => layers.iter().any(|l| l.effect.needs_audio()),
            _ => false,
        }
    }

    /// Whether this effect needs the screen capture.
    pub fn needs_screen(&self) -> bool {
        match self {
            EffectSpec::ScreenAmbience { .. } => true,
            EffectSpec::Layers { layers } => layers.iter().any(|l| l.effect.needs_screen()),
            _ => false,
        }
    }

    /// Checks values the type system cannot (layer stacks).
    pub fn validate(&self) -> Result<(), String> {
        match self {
            EffectSpec::Layers { layers } => layers::validate(layers),
            _ => Ok(()),
        }
    }

    /// Builds the effect without external inputs (audio effects stay dark).
    pub fn build(&self) -> Box<dyn Effect> {
        self.build_with(&EffectContext::default())
    }

    pub fn build_with(&self, ctx: &EffectContext) -> Box<dyn Effect> {
        match *self {
            EffectSpec::Layers { ref layers } => Box::new(layers::Layered::new(layers, ctx)),
            EffectSpec::ScreenAmbience { saturation } => Box::new(screen::ScreenAmbience {
                saturation,
                ctx: ctx.clone(),
            }),
            EffectSpec::AudioPulse {
                color,
                background,
                sensitivity,
            } => Box::new(audio::AudioPulse {
                color,
                background,
                sensitivity,
                ctx: ctx.clone(),
            }),
            EffectSpec::AudioSpectrum {
                low,
                high,
                background,
            } => Box::new(audio::AudioSpectrum {
                low,
                high,
                background,
                ctx: ctx.clone(),
            }),
            EffectSpec::Gradient {
                from,
                to,
                angle_deg,
            } => Box::new(extra::Gradient {
                from,
                to,
                angle_deg,
            }),
            EffectSpec::ColorCycle { period_ms } => Box::new(extra::ColorCycle {
                period: ms(period_ms),
            }),
            EffectSpec::RadialRainbow { speed, scale } => {
                Box::new(extra::RadialRainbow { speed, scale })
            }
            EffectSpec::Scanner {
                color,
                background,
                period_ms,
                width,
            } => Box::new(extra::Scanner {
                color,
                background,
                period: ms(period_ms),
                width,
            }),
            EffectSpec::Twinkle {
                color,
                background,
                density,
                period_ms,
            } => Box::new(extra::Twinkle {
                color,
                background,
                density,
                period: ms(period_ms),
            }),
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
    fn every_name_has_a_default_that_round_trips() {
        for name in EffectSpec::NAMES {
            let spec = EffectSpec::default_for(name).unwrap();
            assert_eq!(spec.name(), name);
            let json = serde_json::to_string(&spec).unwrap();
            assert!(json.contains(&format!("\"type\":\"{name}\"")), "{json}");
            assert_eq!(serde_json::from_str::<EffectSpec>(&json).unwrap(), spec);
            let mut c = Canvas::new(8, 4);
            spec.build().render(Duration::from_millis(500), &mut c);
        }
        assert!(EffectSpec::default_for("nope").is_none());
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
