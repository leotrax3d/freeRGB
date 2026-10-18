//! Corsair "Bragi" protocol (modern Corsair peripherals, vendor usage page
//! `0xFF42`).
//!
//! Evidence: static analysis of SignalRGB 2.5.80 plugin
//! `Corsair/Modern_Corsair/Corsair_Bragi_Device.js` (behaviour description
//! only, no code reused) and HID enumeration of a K70 CORE (`1b1c:1bfd`).
//! See `docs/PROTOCOLS.md#corsair-bragi` for the confidence level of every
//! field.
//!
//! Wire format of a request (bytes after the HID report ID `0x00`):
//! `[target, command, args...]` where `target = 0x08 | child_index`.
//! Responses echo the command and carry an error code:
//! `[target', command, error, data...]`.

pub mod device;
pub mod k70_core;
pub mod link_hub;
pub mod packet;

pub use device::{BragiDevice, BragiModel, LightingScheme};
pub use packet::{Command, Endpoint, Handle, Mode, Property, Response, ResponseError};
