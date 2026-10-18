//! Platform-independent audio analysis: overall level and logarithmic
//! frequency bands, with automatic gain and attack/decay smoothing so the
//! output stays in `0..=1` for quiet and loud sources alike.

pub use freergb_core::audio::{AudioFeatures, BANDS};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

const LOW_HZ: f32 = 40.0;
const HIGH_HZ: f32 = 16_000.0;
/// Below this input RMS the signal counts as silence (no gain boost).
const SILENCE_RMS: f32 = 1e-4;

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    size: usize,
    window: Vec<f32>,
    /// FFT bin range `[start, end)` per band.
    band_bins: [(usize, usize); BANDS],
    /// Slowly decaying peaks used as automatic gain reference. All bands share
    /// one reference so their relative strength is preserved.
    level_peak: f32,
    band_peak: f32,
    smoothed: AudioFeatures,
    scratch: Vec<Complex<f32>>,
}

impl Analyzer {
    /// `size` must be a power of two; 2048 at 48 kHz (≈ 43 ms) resolves the
    /// bass bands well.
    pub fn new(sample_rate: u32, size: usize) -> Self {
        assert!(
            size.is_power_of_two() && size >= 64,
            "FFT size must be a power of two ≥ 64"
        );
        let fft = FftPlanner::new().plan_fft_forward(size);
        let window = (0..size)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / size as f32).cos())
            .collect();
        let hz_per_bin = sample_rate as f32 / size as f32;
        let nyquist_bin = size / 2;
        // Log-spaced edges, forced strictly increasing so low bands never
        // share FFT bins when the resolution is coarse.
        let ratio = (HIGH_HZ / LOW_HZ).powf(1.0 / BANDS as f32);
        let mut edges = [0usize; BANDS + 1];
        for (i, e) in edges.iter_mut().enumerate() {
            let hz = LOW_HZ * ratio.powi(i as i32);
            *e = ((hz / hz_per_bin).round() as usize).clamp(1, nyquist_bin);
        }
        for i in 1..=BANDS {
            edges[i] = edges[i].max(edges[i - 1] + 1).min(nyquist_bin);
        }
        let mut band_bins = [(0, 0); BANDS];
        for (b, slot) in band_bins.iter_mut().enumerate() {
            *slot = (edges[b], edges[b + 1].max(edges[b] + 1));
        }
        Self {
            fft,
            size,
            window,
            band_bins,
            level_peak: 0.0,
            band_peak: 0.0,
            smoothed: AudioFeatures::default(),
            scratch: vec![Complex::default(); size],
        }
    }

    pub fn window_size(&self) -> usize {
        self.size
    }

    /// Analyses the newest `window_size()` mono samples (`-1..=1`); shorter
    /// input is zero-padded. Call at a steady rate (e.g. every 10–20 ms).
    pub fn process(&mut self, mono: &[f32]) -> AudioFeatures {
        let n = self.size;
        let tail = &mono[mono.len().saturating_sub(n)..];
        for (i, c) in self.scratch.iter_mut().enumerate() {
            let s = tail.get(i).copied().unwrap_or(0.0);
            *c = Complex::new(s * self.window[i], 0.0);
        }
        self.fft.process(&mut self.scratch);

        let rms = (tail.iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
        let mut raw_bands = [0.0f32; BANDS];
        for (b, &(start, end)) in self.band_bins.iter().enumerate() {
            let sum: f32 = self.scratch[start..end].iter().map(|c| c.norm()).sum();
            raw_bands[b] = sum / (end - start) as f32;
        }

        // Automatic gain: compare against slowly decaying peaks.
        const PEAK_DECAY: f32 = 0.997;
        let silent = rms < SILENCE_RMS;
        self.level_peak = (self.level_peak * PEAK_DECAY)
            .max(rms)
            .max(SILENCE_RMS * 4.0);
        let level = if silent {
            0.0
        } else {
            (rms / self.level_peak).clamp(0.0, 1.0)
        };
        let loudest = raw_bands.iter().copied().fold(0.0, f32::max);
        self.band_peak = (self.band_peak * PEAK_DECAY).max(loudest).max(1e-6);
        let bands = raw_bands.map(|r| {
            if silent {
                0.0
            } else {
                (r / self.band_peak).clamp(0.0, 1.0)
            }
        });

        // Smoothing: fast attack, slow decay.
        let ease = |old: f32, new: f32| {
            let k = if new > old { 0.6 } else { 0.15 };
            old + (new - old) * k
        };
        self.smoothed.level = ease(self.smoothed.level, level);
        for (s, &b) in self.smoothed.bands.iter_mut().zip(&bands) {
            *s = ease(*s, b);
        }
        self.smoothed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn sine(hz: f32, amp: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (std::f32::consts::TAU * hz * i as f32 / RATE as f32).sin())
            .collect()
    }

    fn settle(a: &mut Analyzer, signal: &[f32]) -> AudioFeatures {
        let mut f = AudioFeatures::default();
        for _ in 0..30 {
            f = a.process(signal);
        }
        f
    }

    fn loudest(f: &AudioFeatures) -> usize {
        (0..BANDS)
            .max_by(|&a, &b| f.bands[a].total_cmp(&f.bands[b]))
            .unwrap()
    }

    #[test]
    fn bands_cover_the_spectrum_in_order() {
        let a = Analyzer::new(RATE, 2048);
        for w in a.band_bins.windows(2) {
            assert!(w[0].0 < w[1].0, "bands ascending: {:?}", a.band_bins);
        }
        assert!(a.band_bins[BANDS - 1].1 <= 1024);
    }

    #[test]
    fn low_and_high_tones_land_in_low_and_high_bands() {
        let mut a = Analyzer::new(RATE, 2048);
        let low = settle(&mut a, &sine(80.0, 0.5, 2048));
        assert!(
            loudest(&low) <= 1,
            "80 Hz -> band {} ({:?})",
            loudest(&low),
            low.bands
        );

        let mut a = Analyzer::new(RATE, 2048);
        let high = settle(&mut a, &sine(8_000.0, 0.5, 2048));
        assert!(
            loudest(&high) >= BANDS - 2,
            "8 kHz -> band {} ({:?})",
            loudest(&high),
            high.bands
        );
        assert!(high.bands[0] < 0.05, "no energy in the bass band");
        assert!(low.bands[BANDS - 1] < 0.05, "no energy in the treble band");
    }

    #[test]
    fn silence_is_zero_and_gain_normalises_volume() {
        let mut a = Analyzer::new(RATE, 2048);
        let s = settle(&mut a, &vec![0.0; 2048]);
        assert_eq!(s.level, 0.0);
        assert!(s.bands.iter().all(|&b| b == 0.0));

        let mut quiet = Analyzer::new(RATE, 2048);
        let mut loud = Analyzer::new(RATE, 2048);
        let q = settle(&mut quiet, &sine(440.0, 0.02, 2048));
        let l = settle(&mut loud, &sine(440.0, 0.8, 2048));
        assert!(
            (q.level - l.level).abs() < 0.05,
            "auto gain: {} vs {}",
            q.level,
            l.level
        );
        assert!(q.level > 0.8);
    }

    #[test]
    fn smoothing_rises_fast_and_falls_slowly() {
        let mut a = Analyzer::new(RATE, 2048);
        let loud = sine(440.0, 0.5, 2048);
        let first = a.process(&loud).level;
        assert!(first > 0.5, "attack: {first}");
        settle(&mut a, &loud);
        let after_drop = a.process(&vec![0.0; 2048]).level;
        assert!(after_drop > 0.7, "decay is slow: {after_drop}");
    }
}
