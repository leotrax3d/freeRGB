// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"SayoDevice E1","vid":0x8089u16,"pid":0x0007u16,"usage_page":0xFF11u16,"usage":0x0002u16,"kind":"lighting_controller","leds":1}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_sayo_device::SayoDevice::default());
}
