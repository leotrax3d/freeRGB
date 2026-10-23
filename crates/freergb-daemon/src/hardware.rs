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
    /// Explicit experimental RGB-only control of known ASUS Aura USB interfaces.
    pub enable_asus_aura: bool,
    /// Manual LED count per 5V ARGB header; zero leaves that header out of frames.
    pub asus_argb_leds: [u16; 4],
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
    let list = match freergb_hid::enumerate() {
        Ok(l) => l,
        Err(e) => {
            lock(engine).observe_error(
                "hid-scan",
                "USB device discovery failed. freeRGB will retry automatically.",
                Some(&e.to_string()),
            );
            return 0;
        }
    };
    lock(engine).observe_error("hid-scan", "", None);
    if !conflicts.is_empty() || lock(engine).has_hardware_plugins() {
        // Setup still shows what is attached; nothing is opened.
        crate::modules::scan(engine, &list, &Default::default(), false);
        return 0;
    }
    lock(engine).prune_finished();
    let mut added = 0;
    // HID path -> built-in adapter's device name; modules never claim these.
    let mut builtin = std::collections::BTreeMap::new();
    for info in &list {
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
            "asus-aura-usb" if opts.enable_asus_aura => {
                let counts = opts.asus_argb_leds;
                (
                    DeviceId(format!("hid:0b05:19af:{serial}")),
                    Box::new(move |transport, serial| {
                        Box::new(freergb_protocols::asus_aura::AuraDevice::new(
                            transport, serial, counts,
                        )) as Box<dyn RgbDevice>
                    }),
                )
            }
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
        builtin.insert(info.path.clone(), m.device_name.clone());
        if lock(engine).has_device(&id) {
            continue;
        }
        match HidapiTransport::open(info) {
            Ok(t) => {
                lock(engine).observe_error(&format!("hid-open:{}", info.path), "", None);
                lock(engine).add_device(make(t, info.serial.clone()));
                added += 1;
            }
            Err(e) => lock(engine).observe_error(
                &format!("hid-open:{}", info.path),
                &format!(
                    "{} could not be opened. Check device access; freeRGB will retry.",
                    m.device_name
                ),
                Some(&e.to_string()),
            ),
        }
    }
    crate::modules::scan(engine, &list, &builtin, true);
    added
}
