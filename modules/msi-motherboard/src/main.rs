// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_msi_motherboard::devices_table::DETECTORS;
use freergb_module_msi_motherboard::table;
fn main() {
    if std::env::args().any(|a| a == "--list-devices") {
        let out: Vec<_> = DETECTORS.iter().map(|d| serde_json::json!({"name": d.name, "vid": d.vid, "pid": d.pid, "interface": d.interface, "usage_page": d.usage_page, "kind": "lighting_controller"})).collect();
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
        return;
    }
    let _ = table::identify;
    freergb_module_api::sdk::run(freergb_module_msi_motherboard::MsiMotherboard::default());
}
