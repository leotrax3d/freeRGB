//! Hardware scan: opens every HID interface whose adapter is implemented
//! (support level Experimental or Validated). Unknown and merely identified
//! devices are left untouched. Adapters that touch cooling hardware (the iCUE
//! LINK hub) additionally need an explicit opt-in.

use crate::{lock, SharedEngine};
use freergb_device_api::{DeviceId, RgbDevice};
use freergb_hid::HidapiTransport;
use freergb_protocols::corsair_bragi::link_hub_device::LinkHubDevice;
use freergb_protocols::corsair_bragi::{BragiDevice, BragiModel};
use freergb_protocols::descriptions::{self, DeviceDescription};
use freergb_protocols::registry::{self, SupportLevel};

/// Builds a device for an opened interface (transport, serial).
type DeviceFactory = Box<dyn FnOnce(HidapiTransport, Option<String>) -> Box<dyn RgbDevice>>;

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Drive the iCUE LINK hub (keeps it in software mode while running).
    pub enable_link_hub: bool,
    /// Device descriptions loaded from the user directory (opt-in, always
    /// experimental). Empty unless `--user-devices` is given.
    pub user_devices: std::sync::Arc<Vec<DeviceDescription>>,
}

/// Adds newly attached supported devices to the engine. Returns how many
/// were added.
pub fn scan(engine: &SharedEngine, opts: &ScanOptions) -> usize {
    let conflicts = crate::conflicts::running();
    {
        let mut e = lock(engine);
        if e.conflicts() != conflicts.as_slice() {
            if conflicts.is_empty() {
                tracing::info!("conflicting RGB software gone, resuming hardware");
            } else {
                let released = e.release_hardware();
                tracing::warn!(
                    ?conflicts,
                    released,
                    "other RGB software is running, leaving hardware alone"
                );
            }
            e.set_conflicts(conflicts.clone());
        }
    }
    if !conflicts.is_empty() {
        return 0;
    }
    let list = match freergb_hid::enumerate() {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(error = %e, "HID enumeration failed");
            return 0;
        }
    };
    lock(engine).prune_finished();
    let mut added = 0;
    for info in list {
        let Some(m) = registry::match_interface_with(
            &opts.user_devices,
            info.vid,
            info.pid,
            info.usage_page,
            info.usage,
        ) else {
            continue;
        };
        if m.level < SupportLevel::Experimental {
            continue;
        }
        let serial = info.serial.as_deref().unwrap_or("noserial");
        let (id, make): (DeviceId, DeviceFactory) = match m.adapter {
            "corsair-bragi" => {
                let model = BragiModel::for_pid_in(descriptions::builtin(), info.pid)
                    .or_else(|| BragiModel::for_pid_in(&opts.user_devices, info.pid));
                let Some(model) = model else {
                    continue;
                };
                (
                    DeviceId(format!("hid:1b1c:{:04x}:{serial}", model.pids[0])),
                    Box::new(move |t, s| {
                        Box::new(BragiDevice::new(t, model, s)) as Box<dyn RgbDevice>
                    }),
                )
            }
            "corsair-icue-link" if opts.enable_link_hub => (
                DeviceId(format!("hid:1b1c:0c3f:{serial}")),
                Box::new(|t, s| Box::new(LinkHubDevice::new(t, s)) as Box<dyn RgbDevice>),
            ),
            _ => continue,
        };
        if lock(engine).has_device(&id) {
            continue;
        }
        match HidapiTransport::open(&info) {
            Ok(t) => {
                lock(engine).add_device(make(t, info.serial.clone()));
                added += 1;
            }
            Err(e) => tracing::warn!(path = %info.path, error = %e, "open failed"),
        }
    }
    added
}
