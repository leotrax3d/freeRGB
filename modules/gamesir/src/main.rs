// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"GameSir RGB Device","vid":0x3537u16,"pid":0x100Fu16,"usage_page":0xFF7Au16,"usage":0x0001u16,"interface":2,"kind":"accessory","leds":1}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_gamesir::GameSir::default());
}
