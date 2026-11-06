// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Lego Dimensions Toypad Base","vid":0x0E6Fu16,"pid":0x0241u16,"kind":"lighting_controller","leds":3}])
        );
        return;
    }
    freergb_module_api::sdk::run(
        freergb_module_lego_dimensions_toypad::LegoDimensionsToypad::default(),
    );
}
