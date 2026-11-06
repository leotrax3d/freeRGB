// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Np93 ALPHA - Gaming Mouse","vid":0x04D8u16,"pid":0xFD0Au16,"interface":0,"usage_page":2u16,"usage":0x0001u16,"kind":"mouse","leds":1}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_lexip_mouse::LexipMouse::default());
}
