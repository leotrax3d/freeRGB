// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([
                {"name":"Ionico Keyboard","vid":0x048Du16,"pid":0xCE00u16,"usage_page":0xFF12u16,"usage":0x0001u16,"kind":"keyboard","leds":4},
                {"name":"Ionico Light Bar","vid":0x048Du16,"pid":0x6005u16,"usage_page":0xFF03u16,"usage":0x0001u16,"kind":"keyboard","leds":22}
            ])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_ionico::Ionico::default());
}
