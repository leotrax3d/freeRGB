// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Skydimo SK0902","vid":0x1A86u16,"pid":0xE316u16,"usage_page":0xFF00u16,"usage":0x0001u16,"kind":"lighting_controller","leds":49}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_skydimo_hid::SkydimoHid::default());
}
