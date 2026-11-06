// SPDX-License-Identifier: GPL-2.0-or-later

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Patriot Viper V550","vid":0x0C45u16,"pid":0x7E18u16,"usage_page":0xFF18u16,"usage":0x01u16,"kind":"mouse","leds":7}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_patriot_viper_mouse::PatriotViperMouse::default());
}
