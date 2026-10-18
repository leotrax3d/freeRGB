//! freeRGB daemon library: engine, JSON-RPC API and IPC transport.
//!
//! The binary `freergbd` wires these together; the pieces are kept in a library
//! so they can be tested without hardware or sockets.

pub mod api;
pub mod conflicts;
pub mod engine;
pub mod hardware;
pub mod ipc;
pub mod profiles;
pub mod state;

use std::sync::{Arc, Mutex, MutexGuard};

pub type SharedEngine = Arc<Mutex<engine::Engine>>;

/// Locks the engine, recovering from a poisoned mutex (a panicking render
/// tick must not take the API down with it).
pub fn lock(engine: &SharedEngine) -> MutexGuard<'_, engine::Engine> {
    engine.lock().unwrap_or_else(|e| e.into_inner())
}
