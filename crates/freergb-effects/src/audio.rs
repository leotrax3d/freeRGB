//! Audio-reactive effects. They read the latest [`AudioFeatures`] from a
//! shared handle in the [`EffectContext`]; without one they render silence.

use crate::Effect;
use freergb_core::audio::BANDS;
use freergb_core::{AudioFeatures, Canvas, Rgb};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Shared, continuously updated audio analysis.
pub type AudioHandle = Arc<Mutex<AudioFeatures>>;

/// Shared, continuously updated downscaled screen picture.
pub type ScreenHandle = Arc<Mutex<freergb_core::ScreenFrame>>;

/// Inputs an effect may use besides time.
#[derive(Clone, Default)]
pub struct EffectContext {
    pub audio: Option<AudioHandle>,
    pub screen: Option<ScreenHandle>,
}

impl EffectContext {
    fn audio(&self) -> AudioFeatures {
        self.audio
            .as_ref()
            .map(|a| *a.lock().unwrap_or_else(|e| e.into_inner()))
            .unwrap_or_default()
    }
}

/// Whole canvas fades between `background` and `color` with the loudness.
pub struct AudioPulse {
    pub color: Rgb,
    pub background: Rgb,
    /// Amplifies quiet passages: `level^(1/sensitivity)`.
    pub sensitivity: f32,
    pub ctx: EffectContext,
}

impl Effect for AudioPulse {
    fn name(&self) -> &str {
        "Audio Pulse"
    }

    fn render(&mut self, _t: Duration, canvas: &mut Canvas) {
        let level = self.ctx.audio().level.clamp(0.0, 1.0);
        let k = level.powf(1.0 / self.sensitivity.clamp(0.2, 5.0));
        canvas.fill(self.background.lerp(self.color, k));
    }
}

/// One vertical bar per frequency band, left = bass, right = treble. Bar
/// height follows the band energy; colour goes from `low` to `high`.
pub struct AudioSpectrum {
    pub low: Rgb,
    pub high: Rgb,
    pub background: Rgb,
    pub ctx: EffectContext,
}

impl Effect for AudioSpectrum {
    fn name(&self) -> &str {
        "Audio Spectrum"
    }

    fn render(&mut self, _t: Duration, canvas: &mut Canvas) {
        let f = self.ctx.audio();
        let (low, high, bg) = (self.low, self.high, self.background);
        canvas.paint(|x, y| {
            let band = ((x * BANDS as f32) as usize).min(BANDS - 1);
            let energy = f.bands[band].clamp(0.0, 1.0);
            // Bars grow from the bottom (y = 1) upwards.
            if 1.0 - y <= energy {
                low.lerp(high, band as f32 / (BANDS - 1) as f32)
            } else {
                bg
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(level: f32, bands: [f32; BANDS]) -> EffectContext {
        EffectContext {
            audio: Some(Arc::new(Mutex::new(AudioFeatures { level, bands }))),
            screen: None,
        }
    }

    #[test]
    fn pulse_follows_level_and_is_dark_without_audio() {
        let mut c = Canvas::new(4, 4);
        let mut p = AudioPulse {
            color: Rgb::WHITE,
            background: Rgb::BLACK,
            sensitivity: 1.0,
            ctx: EffectContext::default(),
        };
        p.render(Duration::ZERO, &mut c);
        assert_eq!(c.get(0, 0), Some(Rgb::BLACK), "no audio handle = silence");

        p.ctx = ctx(1.0, [0.0; BANDS]);
        p.render(Duration::ZERO, &mut c);
        assert_eq!(c.get(3, 3), Some(Rgb::WHITE));

        p.ctx = ctx(0.25, [0.0; BANDS]);
        p.sensitivity = 2.0; // sqrt(0.25) = 0.5
        p.render(Duration::ZERO, &mut c);
        assert_eq!(c.get(1, 1), Some(Rgb::new(128, 128, 128)));
    }

    #[test]
    fn spectrum_bars_match_band_energy() {
        let mut bands = [0.0; BANDS];
        bands[0] = 1.0; // full bass bar
        bands[7] = 0.5; // half treble bar
        let mut s = AudioSpectrum {
            low: Rgb::RED,
            high: Rgb::BLUE,
            background: Rgb::BLACK,
            ctx: ctx(0.0, bands),
        };
        let mut c = Canvas::new(80, 10);
        s.render(Duration::ZERO, &mut c);
        assert_eq!(c.sample(0.02, 0.05), Rgb::RED, "bass bar reaches the top");
        assert_eq!(c.sample(0.98, 0.95), Rgb::BLUE, "treble bar bottom");
        assert_eq!(
            c.sample(0.98, 0.05),
            Rgb::BLACK,
            "treble bar only half high"
        );
        assert_eq!(c.sample(0.5, 0.95), Rgb::BLACK, "silent mid band");
    }
}
