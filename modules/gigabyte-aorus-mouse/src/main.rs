// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Gigabyte Aorus Mouse","vid":0x1044u16,"pid":0x7A40u16,"usage_page":0xFF01u16,"usage":0x0001u16,"interface":3,"kind":"mouse","leds":1}])
        );
        return;
    }
    freergb_module_api::sdk::run(
        freergb_module_gigabyte_aorus_mouse::GigabyteAorusMouse::default(),
    );
}
