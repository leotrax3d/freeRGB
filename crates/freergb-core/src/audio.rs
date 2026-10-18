//! Audio analysis result shared between the audio capture and the effects.
//! Plain data, no I/O: effects can be tested with synthetic values.

use serde::{Deserialize, Serialize};

/// Number of logarithmic frequency bands.
pub const BANDS: usize = 8;

/// Audio analysis result, every value in `0..=1`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioFeatures {
    /// Overall loudness.
    pub level: f32,
    /// Energy per band, low → high frequencies (≈ 40 Hz … 16 kHz).
    pub bands: [f32; BANDS],
}
