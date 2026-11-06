// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Cooler Master MM712","vid":0x2516u16,"pid":0x0169u16,"usage_page":0xFF0Au16,"usage":0x0002u16,"interface":3,"kind":"mouse","leds":1}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_coolermaster_mm712::CoolerMasterMm712::default());
}
