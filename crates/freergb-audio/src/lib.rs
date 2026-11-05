//! Audio input for audio-reactive effects: platform-independent analysis
//! (`analysis`) and system-audio capture (`capture`).

pub mod analysis;
pub mod capture;
#[cfg(any(target_os = "linux", test))]
mod linux;

pub use analysis::{Analyzer, AudioFeatures, BANDS};
pub use capture::{AudioInput, CaptureError};
