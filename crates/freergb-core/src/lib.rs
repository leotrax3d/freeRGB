//! Platform-independent RGB core of freeRGB.
//!
//! This crate has no I/O and no platform dependencies. It defines the colour
//! model, LED layouts, the effect canvas and per-device frames that every other
//! crate builds on.

pub mod audio;
pub mod canvas;
pub mod color;
pub mod frame;
pub mod layout;
pub mod screen;

pub use audio::AudioFeatures;
pub use canvas::{Canvas, Region};
pub use color::{ColorCorrection, Hsv, Rgb};
pub use frame::Frame;
pub use layout::{LayoutError, Led, LedLayout};
pub use screen::ScreenFrame;
