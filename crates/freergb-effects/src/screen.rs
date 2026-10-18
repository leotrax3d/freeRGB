//! Screen ambience: the canvas shows a downscaled copy of the screen, so each
//! device lights up with the colours of "its" part of the screen (placement via
//! the layout editor / `devices.set_region`).

use crate::{Effect, EffectContext};
use freergb_core::{Canvas, Hsv, ScreenFrame};
use std::time::Duration;

pub struct ScreenAmbience {
    /// Saturation multiplier (1 = unchanged); LEDs look washed out without a boost.
    pub saturation: f32,
    pub ctx: EffectContext,
}

impl Effect for ScreenAmbience {
    fn name(&self) -> &str {
        "Screen Ambience"
    }

    fn render(&mut self, _t: Duration, canvas: &mut Canvas) {
        let frame: ScreenFrame = self
            .ctx
            .screen
            .as_ref()
            .map(|s| s.lock().unwrap_or_else(|e| e.into_inner()).clone())
            .unwrap_or_default();
        let sat = self.saturation.clamp(0.0, 3.0);
        canvas.paint(|x, y| {
            let c = frame.sample(x, y);
            if (sat - 1.0).abs() < f32::EPSILON {
                return c;
            }
            let mut hsv = c.to_hsv();
            hsv.s = (hsv.s * sat).clamp(0.0, 1.0);
            Hsv::new(hsv.h, hsv.s, hsv.v).to_rgb()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;
    use std::sync::{Arc, Mutex};

    #[test]
    fn mirrors_screen_and_boosts_saturation() {
        let frame = ScreenFrame {
            width: 2,
            height: 1,
            pixels: vec![Rgb::new(200, 100, 100), Rgb::BLUE],
        };
        let ctx = EffectContext {
            audio: None,
            screen: Some(Arc::new(Mutex::new(frame))),
        };
        let mut e = ScreenAmbience {
            saturation: 1.0,
            ctx,
        };
        let mut c = Canvas::new(4, 2);
        e.render(Duration::ZERO, &mut c);
        assert_eq!(c.sample(0.1, 0.5), Rgb::new(200, 100, 100));
        assert_eq!(c.sample(0.9, 0.5), Rgb::BLUE);

        e.saturation = 2.0;
        e.render(Duration::ZERO, &mut c);
        let boosted = c.sample(0.1, 0.5);
        assert_eq!(boosted.r, 200, "value kept");
        assert!(boosted.g < 100, "less grey: {boosted}");

        let mut none = ScreenAmbience {
            saturation: 1.0,
            ctx: EffectContext::default(),
        };
        none.render(Duration::ZERO, &mut c);
        assert_eq!(c.sample(0.5, 0.5), Rgb::BLACK, "no capture = dark");
    }
}
