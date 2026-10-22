// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_asus_aura_core::devices_table::DETECTORS;
fn main() {
    if std::env::args().any(|a| a == "--list-devices") {
        println!("{}", serde_json::to_string_pretty(&DETECTORS.iter().map(|d| serde_json::json!({"name":d.name,"vid":d.vid,"pid":d.pid,"interface":d.interface,"usage_page":d.usage_page,"kind":"lighting_controller"})).collect::<Vec<_>>()).unwrap());
        return;
    }
    freergb_module_api::sdk::run(freergb_module_asus_aura_core::AsusAuraUsb::default());
}
