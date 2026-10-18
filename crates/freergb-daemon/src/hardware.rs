//! Hardware scan: opens every HID interface whose adapter is implemented
//! (support level Experimental or Validated). Unknown and merely identified
//! devices are left untouched. Adapters that touch cooling hardware (the iCUE
//! LINK hub) additionally need an explicit opt-in.

use crate::{lock, SharedEngine};
use freergb_device_api::{DeviceId, RgbDevice};
use freergb_hid::HidapiTransport;
use freergb_protocols::corsair_bragi::link_hub_device::LinkHubDevice;
use freergb_protocols::corsair_bragi::{BragiDevice, BragiModel};
use freergb_protocols::registry::{self, SupportLevel};

/// Builds a device for an opened interface (transport, serial, product id).
type DeviceFactory = fn(HidapiTransport, Option<String>, u16) -> Box<dyn RgbDevice>;

#[derive(Debug, Clone, Copy, Default)]
pub struct ScanOptions {
    /// Drive the iCUE LINK hub (keeps it in software mode while running).
    pub enable_link_hub: bool,
}

/// Adds newly attached supported devices to the engine. Returns how many
/// were added.
pub fn scan(engine: &SharedEngine, opts: ScanOptions) -> usize {
    let list = match freergb_hid::enumerate() {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(error = %e, "HID enumeration failed");
            return 0;
        }
    };
    let mut added = 0;
    for info in list {
        let Some(m) = registry::match_interface(info.vid, info.pid, info.usage_page, info.usage)
        else {
            continue;
        };
        if m.level < SupportLevel::Experimental {
            continue;
        }
        let serial = info.serial.as_deref().unwrap_or("noserial");
        let (id, make): (DeviceId, DeviceFactory) = match m.adapter {
            "corsair-bragi" => {
                let Some(model) = BragiModel::for_pid(info.pid) else {
                    continue;
                };
                (
                    DeviceId(format!("hid:1b1c:{:04x}:{serial}", model.pids[0])),
                    |t, s, pid| {
                        let model = BragiModel::for_pid(pid).expect("checked above");
                        Box::new(BragiDevice::new(t, model, s))
                    },
                )
            }
            "corsair-icue-link" if opts.enable_link_hub => {
                (DeviceId(format!("hid:1b1c:0c3f:{serial}")), |t, s, _| {
                    Box::new(LinkHubDevice::new(t, s))
                })
            }
            _ => continue,
        };
        if lock(engine).has_device(&id) {
            continue;
        }
        match HidapiTransport::open(&info) {
            Ok(t) => {
                lock(engine).add_device(make(t, info.serial.clone(), info.pid));
                added += 1;
            }
            Err(e) => tracing::warn!(path = %info.path, error = %e, "open failed"),
        }
    }
    added
}
