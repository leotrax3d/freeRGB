//! Hardware scan: opens every HID interface whose adapter is implemented
//! (support level Experimental or Validated). Unknown and merely identified
//! devices are left untouched. Adapters that touch cooling hardware (the iCUE
//! LINK hub) additionally need an explicit opt-in. Motherboard RGB controllers
//! open automatically unless the global switch in `mainboard-rgb.json` is off
//! or vendor lighting software is running.

use crate::{lock, SharedEngine};
use freergb_device_api::{DeviceId, RgbDevice};
use freergb_hid::HidapiTransport;
use freergb_protocols::corsair_bragi::link_hub_device::LinkHubDevice;
use freergb_protocols::corsair_bragi::{BragiDevice, BragiModel};
use freergb_protocols::descriptions::{self, DeviceDescription};
use freergb_protocols::registry::{self, SupportLevel};

use freergb_protocols::gigabyte_fusion2::Fusion2Device;

/// Builds a device for an opened interface (transport, serial).
type DeviceFactory = Box<dyn FnOnce(HidapiTransport, Option<String>) -> Box<dyn RgbDevice>>;

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Test override: ASUS Aura ARGB LED counts from `--asus-argb-leds` instead
    /// of the saved motherboard RGB settings (`mainboard-rgb.json`).
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
    let mut conflicts = crate::conflicts::running();
    if lock(engine).lighting_paused() {
        conflicts.push(crate::engine::PAUSED_BY_USER);
    }
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
            tracing::warn!(error = %e, "HID enumeration failed");
            return 0;
        }
    };
    if !conflicts.is_empty() || lock(engine).has_hardware_plugins() {
        // Setup still shows what is attached; nothing is opened.
        crate::modules::scan(engine, &list, &Default::default(), false);
        return 0;
    }
    lock(engine).prune_finished();
    let (mainboard, vendors) = {
        let mut e = lock(engine);
        let vendors = crate::conflicts::mainboard_vendors_running();
        if !vendors.is_empty() && e.mainboard_rgb.vendor_software() != vendors.as_slice() {
            tracing::warn!(
                ?vendors,
                "motherboard vendor lighting software running; motherboard RGB left alone"
            );
        }
        e.mainboard_rgb.set_vendor_software(vendors.clone());
        (e.mainboard_rgb.settings().clone(), vendors)
    };
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
        if registry::is_mainboard_rgb(m.adapter) && !mainboard_allowed(&mainboard, &vendors, opts) {
            continue;
        }
        let serial = info.serial.as_deref().unwrap_or("noserial");
        let (vid, pid) = (info.vid, info.pid);
        let (id, make): (DeviceId, DeviceFactory) = match m.adapter {
            "asus-aura-usb" => {
                let (mut counts, names) = mainboard.headers::<4>(vid, pid);
                let (orders, fallbacks) = mainboard.header_options::<4>(vid, pid);
                let report = lock(engine).mainboard_rgb.report_cell(vid, pid);
                if opts.enable_asus_aura {
                    counts = opts.asus_argb_leds;
                }
                (
                    DeviceId(format!("hid:0b05:{pid:04x}:{serial}")),
                    Box::new(move |transport, serial| {
                        Box::new(
                            freergb_protocols::asus_aura::AuraDevice::new(
                                transport, pid, serial, counts,
                            )
                            .with_names(names)
                            .with_header_options(orders, fallbacks)
                            .with_report(report.clone()),
                        ) as Box<dyn RgbDevice>
                    }),
                )
            }
            "asrock-polychrome-usb" => {
                let (counts, names) = mainboard.headers::<3>(vid, pid);
                (
                    DeviceId(format!("hid:26ce:{pid:04x}:{serial}")),
                    Box::new(move |transport, serial| {
                        Box::new(
                            freergb_protocols::asrock_polychrome::PolychromeDevice::new(
                                transport, pid, serial, counts,
                            )
                            .with_names(names),
                        ) as Box<dyn RgbDevice>
                    }),
                )
            }
            "gigabyte-fusion2-usb" => {
                let (counts, names) = mainboard.headers::<2>(vid, pid);
                (
                    DeviceId(format!("hid:{vid:04x}:{pid:04x}:{serial}")),
                    Box::new(move |transport, serial| {
                        Box::new(
                            Fusion2Device::new(transport, vid, pid, serial, counts)
                                .with_names(names),
                        ) as Box<dyn RgbDevice>
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
                lock(engine).add_device(make(t, info.serial.clone()));
                added += 1;
            }
            Err(e) => tracing::warn!(path = %info.path, error = %e, "open failed"),
        }
    }
    crate::modules::scan(engine, &list, &builtin, true);
    let configured = lock(engine)
        .modules
        .as_ref()
        .map(|m| m.configured())
        .unwrap_or_default();
    lock(engine).cooling_plugins.scan(&list, &|info| {
        builtin.contains_key(&info.path)
            || configured.contains_key(&freergb_modules::setup::device_key(
                &crate::modules::interface(info),
            ))
    });
    added
}

/// Motherboard controllers open by default; the global switch turns them off
/// and running vendor software (warned about in the GUI) leaves them alone.
/// The test override flag always opens ASUS Aura.
fn mainboard_allowed(
    settings: &crate::mainboard_rgb::Settings,
    vendors: &[&str],
    opts: &ScanOptions,
) -> bool {
    opts.enable_asus_aura || (settings.enabled && vendors.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mainboard_rgb_is_on_by_default_and_yields_to_switch_and_vendor_software() {
        let mut settings = crate::mainboard_rgb::Settings::default();
        let opts = ScanOptions::default();
        assert!(mainboard_allowed(&settings, &[], &opts));
        assert!(!mainboard_allowed(&settings, &["MSI Center"], &opts));
        settings.enabled = false;
        assert!(!mainboard_allowed(&settings, &[], &opts));
        let forced = ScanOptions {
            enable_asus_aura: true,
            ..Default::default()
        };
        assert!(mainboard_allowed(&settings, &["MSI Center"], &forced));
    }
}
