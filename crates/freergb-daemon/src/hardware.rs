//! Hardware scan: opens every HID interface whose adapter is implemented
//! (support level Experimental or Validated). Unknown and merely identified
//! devices are left untouched.

use crate::{lock, SharedEngine};
use freergb_device_api::DeviceId;
use freergb_hid::HidapiTransport;
use freergb_protocols::corsair_bragi::{BragiDevice, BragiModel};
use freergb_protocols::registry::{self, SupportLevel};

/// Adds newly attached supported devices to the engine. Returns how many
/// were added.
pub fn scan(engine: &SharedEngine) -> usize {
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
        if m.adapter != "corsair-bragi" {
            continue;
        }
        let Some(model) = BragiModel::for_pid(info.pid) else {
            continue;
        };
        let id = DeviceId(format!(
            "hid:1b1c:{:04x}:{}",
            model.pids[0],
            info.serial.as_deref().unwrap_or("noserial")
        ));
        if lock(engine).has_device(&id) {
            continue;
        }
        match HidapiTransport::open(&info) {
            Ok(t) => {
                let dev = BragiDevice::new(t, model, info.serial.clone());
                lock(engine).add_device(Box::new(dev));
                added += 1;
            }
            Err(e) => tracing::warn!(path = %info.path, error = %e, "open failed"),
        }
    }
    added
}
