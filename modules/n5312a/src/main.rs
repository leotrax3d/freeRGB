// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"N5312A USB Optical Mouse","vid":0x4E53u16,"pid":0x5406u16,"usage_page":0xFF01u16,"usage":0x0001u16,"interface":1,"kind":"mouse","leds":1}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_n5312a::N5312a::default());
}
