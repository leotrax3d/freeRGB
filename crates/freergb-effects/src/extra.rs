//! Additional built-in effects. Like the basic ones they are pure functions of
//! time: no internal state, no randomness that depends on wall-clock or
//! frame rate, so `render(t)` is reproducible.

use crate::Effect;
use freergb_core::{Canvas, Hsv, Rgb};
use std::time::Duration;

fn phase(t: Duration, period: Duration) -> f32 {
    (t.as_secs_f64() / period.as_secs_f64().max(1e-3)).fract() as f32
}

/// Static linear gradient from `from` to `to` along `angle_deg`
/// (0° = left → right, 90° = top → bottom).
pub struct Gradient {
    pub from: Rgb,
    pub to: Rgb,
    pub angle_deg: f32,
}

impl Effect for Gradient {
    fn name(&self) -> &str {
        "Gradient"
    }

    fn render(&mut self, _t: Duration, canvas: &mut Canvas) {
        let (s, c) = self.angle_deg.to_radians().sin_cos();
        // Project the centred point onto the direction and map to 0..=1.
        let span = c.abs() + s.abs();
        let (from, to) = (self.from, self.to);
        canvas.paint(|x, y| {
            let p = ((x - 0.5) * c + (y - 0.5) * s) / span.max(1e-6) + 0.5;
            from.lerp(to, p)
        });
    }
}

/// Whole canvas in one colour, cycling through all hues once per period.
pub struct ColorCycle {
    pub period: Duration,
}

impl Effect for ColorCycle {
    fn name(&self) -> &str {
        "Color Cycle"
    }

    fn render(&mut self, t: Duration, canvas: &mut Canvas) {
        canvas.fill(Hsv::new(phase(t, self.period) * 360.0, 1.0, 1.0).to_rgb());
    }
}

/// Hue depends on the distance from the centre and moves outwards.
pub struct RadialRainbow {
    /// Rings per second.
    pub speed: f32,
    /// Hue cycles from the centre to a corner.
    pub scale: f32,
}

impl Effect for RadialRainbow {
    fn name(&self) -> &str {
        "Radial Rainbow"
    }

    fn render(&mut self, t: Duration, canvas: &mut Canvas) {
        let offset = t.as_secs_f32() * self.speed;
        let scale = self.scale / std::f32::consts::FRAC_1_SQRT_2;
        canvas.paint(|x, y| {
            let d = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
            Hsv::new((d * scale - offset) * 360.0, 1.0, 1.0).to_rgb()
        });
    }
}

/// A soft vertical bar sweeping left → right → left ("Knight Rider").
pub struct Scanner {
    pub color: Rgb,
    pub background: Rgb,
    pub period: Duration,
    /// Bar half-width as a fraction of the canvas width.
    pub width: f32,
}

impl Effect for Scanner {
    fn name(&self) -> &str {
        "Scanner"
    }

    fn render(&mut self, t: Duration, canvas: &mut Canvas) {
        let p = phase(t, self.period);
        // Triangle wave 0 → 1 → 0, eased so the bar slows down at the edges.
        let tri = 1.0 - (2.0 * p - 1.0).abs();
        let pos = 0.5 - 0.5 * (tri * std::f32::consts::PI).cos();
        let w = self.width.max(0.01);
        let (fg, bg) = (self.color, self.background);
        canvas.paint(|x, _| {
            let k = (1.0 - (x - pos).abs() / w).clamp(0.0, 1.0);
            bg.lerp(fg, k * k)
        });
    }
}

/// Random-looking pixels light up and fade out. Deterministic: whether and
/// when a pixel twinkles is a hash of its position and the time slot.
pub struct Twinkle {
    pub color: Rgb,
    pub background: Rgb,
    /// Fraction of pixels that light up per slot (0..=1).
    pub density: f32,
    /// Length of one twinkle.
    pub period: Duration,
}

fn hash(a: u64) -> u64 {
    // splitmix64 finaliser
    let mut z = a.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Effect for Twinkle {
    fn name(&self) -> &str {
        "Twinkle"
    }

    fn render(&mut self, t: Duration, canvas: &mut Canvas) {
        let period = self.period.as_secs_f64().max(0.05);
        let slots = t.as_secs_f64() / period;
        let threshold = (f64::from(self.density.clamp(0.0, 1.0)) * u32::MAX as f64) as u64;
        let (fg, bg) = (self.color, self.background);
        let w = canvas.width();
        let h = canvas.height();
        for y in 0..h {
            for x in 0..w {
                // Each pixel gets its own phase offset so pixels do not all
                // switch at the same instant.
                let cell = (y * w + x) as u64;
                let offset = (hash(cell) % 1000) as f64 / 1000.0;
                let local = slots + offset;
                let slot = local.floor() as u64;
                let lit = (hash(cell ^ slot.rotate_left(32)) & 0xFFFF_FFFF) < threshold;
                let c = if lit {
                    // Fast rise, slow fade inside the slot.
                    let f = local.fract() as f32;
                    let level = if f < 0.15 {
                        f / 0.15
                    } else {
                        1.0 - (f - 0.15) / 0.85
                    };
                    bg.lerp(fg, level)
                } else {
                    bg
                };
                canvas.set(x, y, c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_endpoints_follow_angle() {
        let mut c = Canvas::new(10, 10);
        let mut g = Gradient {
            from: Rgb::RED,
            to: Rgb::BLUE,
            angle_deg: 0.0,
        };
        g.render(Duration::ZERO, &mut c);
        assert!(c.sample(0.0, 0.5).r > 200 && c.sample(0.0, 0.5).b < 50);
        assert!(c.sample(1.0, 0.5).b > 200 && c.sample(1.0, 0.5).r < 50);
        assert_eq!(
            c.sample(0.0, 0.0),
            c.sample(0.0, 1.0),
            "horizontal: rows equal"
        );

        g.angle_deg = 90.0;
        g.render(Duration::ZERO, &mut c);
        assert!(c.sample(0.5, 0.0).r > 200);
        assert!(c.sample(0.5, 1.0).b > 200);
    }

    #[test]
    fn color_cycle_wraps_per_period() {
        let mut c = Canvas::new(2, 2);
        let mut e = ColorCycle {
            period: Duration::from_secs(3),
        };
        e.render(Duration::ZERO, &mut c);
        assert_eq!(c.get(0, 0), Some(Rgb::RED));
        e.render(Duration::from_secs(1), &mut c);
        assert_eq!(c.get(1, 1), Some(Rgb::GREEN));
        e.render(Duration::from_secs(3), &mut c);
        assert_eq!(c.get(0, 0), Some(Rgb::RED));
    }

    #[test]
    fn radial_rainbow_is_symmetric() {
        let mut c = Canvas::new(20, 20);
        RadialRainbow {
            speed: 0.3,
            scale: 1.0,
        }
        .render(Duration::from_millis(700), &mut c);
        assert_eq!(c.get(0, 0), c.get(19, 19));
        assert_eq!(c.get(0, 19), c.get(19, 0));
    }

    #[test]
    fn scanner_moves_between_edges() {
        let mut c = Canvas::new(100, 1);
        let mut s = Scanner {
            color: Rgb::RED,
            background: Rgb::BLACK,
            period: Duration::from_secs(2),
            width: 0.1,
        };
        // Column with the strongest red = bar centre.
        let peak = |c: &Canvas| (0..100).max_by_key(|&x| c.get(x, 0).unwrap().r).unwrap();
        s.render(Duration::ZERO, &mut c);
        assert_eq!(peak(&c), 0, "starts at the left edge");
        assert_eq!(c.sample(1.0, 0.5), Rgb::BLACK);
        s.render(Duration::from_secs(1), &mut c);
        assert_eq!(peak(&c), 99, "right edge at half period");
        s.render(Duration::from_millis(500), &mut c);
        assert!((49..=50).contains(&peak(&c)), "centre at a quarter period");
    }

    #[test]
    fn twinkle_is_deterministic_and_respects_density() {
        let make = |density| Twinkle {
            color: Rgb::WHITE,
            background: Rgb::BLACK,
            density,
            period: Duration::from_millis(800),
        };
        let t = Duration::from_millis(1234);
        let (mut a, mut b) = (Canvas::new(40, 25), Canvas::new(40, 25));
        make(0.3).render(t, &mut a);
        make(0.3).render(t, &mut b);
        assert_eq!(a, b);

        let lit = |c: &Canvas| {
            (0..25)
                .flat_map(|y| (0..40).map(move |x| (x, y)))
                .filter(|&(x, y)| c.get(x, y) != Some(Rgb::BLACK))
                .count()
        };
        let mut none = Canvas::new(40, 25);
        make(0.0).render(t, &mut none);
        assert_eq!(lit(&none), 0);
        let n = lit(&a);
        assert!((150..450).contains(&n), "~30 % of 1000 pixels lit, got {n}");
    }
}
