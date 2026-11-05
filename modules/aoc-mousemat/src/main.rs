// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"AOC Mousemat","vid":0x3938u16,"pid":0x1162u16,"usage_page":0xFF19u16,"usage":0xFF19u16,"kind":"accessory","leds":1}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_aoc_mousemat::AocMousemat::default());
}
