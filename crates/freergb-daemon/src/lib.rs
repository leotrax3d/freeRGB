//! freeRGB daemon library: engine, JSON-RPC API and IPC transport.
//!
//! The binary `freergbd` wires these together; the pieces are kept in a library
//! so they can be tested without hardware or sockets.

pub mod api;
pub mod background;
pub mod bus_simulation;
pub mod channel_bindings;
pub mod composite_feed;
pub mod conflicts;
pub mod control;
pub mod cooling_assignments;
pub mod cooling_host;
pub mod cooling_plugins;
pub mod cooling_profiles;
pub mod cooling_runtime;
pub mod cooling_service;
pub mod cooling_sessions;
pub mod custom_effects;
pub mod device_macros;
pub mod engine;
pub mod fan_coverage;
pub mod fan_support;
pub mod gpu_cooling;
pub mod hardware;
pub mod hwmon;
pub mod ipc;
pub mod keyboard;
pub mod macro_recorder;
pub mod mainboard;
pub mod mainboard_rgb;
pub mod modules;
pub mod native_bridge;
pub mod native_session;
pub mod onboarding;
pub mod profiles;
pub mod report;
pub mod state;
pub mod storage_temps;

use std::sync::{Arc, Mutex, MutexGuard};

pub type SharedEngine = Arc<Mutex<engine::Engine>>;

/// Locks the engine, recovering from a poisoned mutex (a panicking render
/// tick must not take the API down with it).
pub fn lock(engine: &SharedEngine) -> MutexGuard<'_, engine::Engine> {
    engine.lock().unwrap_or_else(|e| e.into_inner())
}
