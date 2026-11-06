// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_mountain_keyboard::devices_table::DETECTORS;
use freergb_module_mountain_keyboard::table;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        let devices: Vec<_> = DETECTORS
            .iter()
            .filter_map(|d| {
                let interface = freergb_module_api::InterfaceInfo {
                    vid: d.vid,
                    pid: d.pid,
                    interface: d.interface,
                    usage_page: d.usage_page,
                    usage: d.usage,
                    product: None,
                };
                let id = table::identify(&interface);
                id.supported.then(|| {
                    serde_json::json!({
                        "name": d.name, "kind": id.kind.unwrap(),
                        "vid": d.vid, "pid": d.pid, "interface": d.interface,
                    })
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&devices).unwrap());
        return;
    }
    freergb_module_api::sdk::run(freergb_module_mountain_keyboard::MountainKeyboard);
}
