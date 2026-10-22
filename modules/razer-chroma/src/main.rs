// SPDX-License-Identifier: GPL-2.0-or-later
//! Module executable: speaks the freeRGB module protocol on stdin/stdout.
//! `--list-devices` prints the supported device matches for the catalog.
use freergb_module_razer_chroma::devices_table::DETECTORS;
use freergb_module_razer_chroma::table;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        let devices: Vec<_> = DETECTORS
            .iter()
            .filter_map(|d| {
                let interface = freergb_module_api::InterfaceInfo {
                    vid: d.vid,
                    pid: d.pid,
                    interface: d.interface.unwrap_or(0),
                    usage_page: d.usage_page.unwrap_or(0),
                    usage: d.usage.unwrap_or(0),
                    product: None,
                };
                let (device, id) = table::identify(&interface);
                id.supported.then(|| {
                    serde_json::json!({
                        "name": d.name, "kind": device.unwrap().kind,
                        "vid": d.vid, "pid": d.pid, "interface": d.interface,
                        "usage_page": d.usage_page, "usage": d.usage,
                    })
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&devices).unwrap());
        return;
    }
    freergb_module_api::sdk::run(freergb_module_razer_chroma::RazerChroma::default());
}
