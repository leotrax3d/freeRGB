//! Glue between the hardware scan, the engine and the module setup service.
//!
//! The scan hands every enumerated HID interface to the setup manager (which
//! decides what becomes a card) and starts a [`ModuleDevice`] for each
//! configured device whose installed module verifies. Nothing here needs the
//! network: installed modules work offline.
use crate::{lock, SharedEngine};
use freergb_device_api::{DeviceError, DeviceId, DeviceStatus, HidTransport};
use freergb_hid::{HidInterfaceInfo, HidapiTransport};
use freergb_module_api::InterfaceInfo;
use freergb_modules::host::{Launch, ModuleDevice};
use freergb_modules::setup::{device_key, Manager, Runtime, Seen};
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn interface(info: &HidInterfaceInfo) -> InterfaceInfo {
    InterfaceInfo {
        vid: info.vid,
        pid: info.pid,
        interface: info.interface,
        usage_page: info.usage_page,
        usage: info.usage,
        product: info.product.clone(),
    }
}

/// Stable engine id without exposing the OS path or serial number.
fn engine_id(module: &str, info: &HidInterfaceInfo) -> DeviceId {
    let hash = info.path.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    DeviceId(format!(
        "module:{module}:{:04x}:{:04x}:{:08x}",
        info.vid, info.pid, hash as u32
    ))
}

/// Module devices currently in the engine, keyed by setup device key.
fn runtime(engine: &SharedEngine) -> BTreeMap<String, Runtime> {
    let mut out = BTreeMap::new();
    for device in lock(engine).devices() {
        // Keys are remembered when `attach` creates the device.
        let Some(key) = KEYS.with_key(&device.info.id.0) else {
            continue;
        };
        out.insert(
            key,
            Runtime {
                engine_id: device.info.id.0.clone(),
                connected: device.status == DeviceStatus::Connected,
                error: match &device.status {
                    DeviceStatus::Faulted(e) => Some(e.clone()),
                    _ => device.last_error.clone(),
                },
            },
        );
    }
    out
}

/// engine id -> device key, for devices created by [`attach`].
struct Keys(std::sync::Mutex<BTreeMap<String, String>>);

impl Keys {
    fn with_key(&self, id: &str) -> Option<String> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned()
    }
    fn insert(&self, id: String, key: String) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, key);
    }
    fn ids_for(&self, select: impl Fn(&str) -> bool) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|(id, key)| select(key) || select(id))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

static KEYS: Keys = Keys(std::sync::Mutex::new(BTreeMap::new()));

/// Called by every hardware scan. `builtin` names interfaces a built-in
/// adapter handles; `allow_start` is false while other RGB software runs.
pub fn scan(
    engine: &SharedEngine,
    list: &[HidInterfaceInfo],
    builtin: &BTreeMap<String, String>,
    allow_start: bool,
) {
    let Some(manager) = lock(engine).modules.clone() else {
        return;
    };
    let seen: Vec<Seen> = list
        .iter()
        .map(|info| Seen {
            interface: interface(info),
            manufacturer: info.manufacturer.clone(),
            builtin: builtin.get(&info.path).cloned(),
        })
        .collect();
    if allow_start {
        attach(engine, &manager, list, builtin);
    }
    manager.observe(&seen, &runtime(engine));
}

fn attach(
    engine: &SharedEngine,
    manager: &Arc<Manager>,
    list: &[HidInterfaceInfo],
    builtin: &BTreeMap<String, String>,
) {
    let configured = manager.configured();
    if configured.is_empty() {
        return;
    }
    for info in list.iter().filter(|i| !builtin.contains_key(&i.path)) {
        let interface = interface(info);
        let key = device_key(&interface);
        let Some(module) = configured.get(&key) else {
            continue;
        };
        let id = engine_id(module, info);
        if lock(engine).has_device(&id) {
            continue;
        }
        let verified = match manager.store().verified(module) {
            Ok(v) => v,
            Err(error) => {
                tracing::warn!(%module, %error, "module not started");
                continue;
            }
        };
        if !verified
            .manifest
            .devices
            .iter()
            .any(|d| d.matches(&interface))
        {
            continue;
        }
        let name = verified
            .manifest
            .devices
            .iter()
            .find(|d| d.matches(&interface))
            .map(|d| d.name.clone())
            .unwrap_or_default();
        let launch = Launch {
            module_id: module.clone(),
            executable: verified.executable,
            directory: verified.directory,
        };
        let path = info.clone();
        // The daemon owns the handle; the module only receives report bytes.
        let open: freergb_modules::host::Opener = Box::new(move || {
            HidapiTransport::open(&path)
                .map(|t| Box::new(t) as Box<dyn HidTransport>)
                .map_err(|e| {
                    let message = e.to_string();
                    let lower = message.to_lowercase();
                    if lower.contains("access")
                        || lower.contains("denied")
                        || lower.contains("permission")
                    {
                        DeviceError::Refused(format!(
                            "Permission: cannot open the device ({message})"
                        ))
                    } else {
                        DeviceError::Refused(format!("Busy or unavailable: {message}"))
                    }
                })
        });
        KEYS.insert(id.0.clone(), key.clone());
        tracing::info!(device = %id, %module, "starting module device");
        let mut device = ModuleDevice::new(id, name, launch, interface, open);
        device.set_probe_policy(verified.manifest.probe);
        lock(engine).add_device(Box::new(device));
    }
}

/// Stops module devices for a setup key (retry) or a module (rollback).
pub fn stop(engine: &SharedEngine, select: impl Fn(&str) -> bool) -> usize {
    let ids = KEYS.ids_for(|s| select(s));
    lock(engine).remove_devices(|id| ids.contains(&id.0))
}
