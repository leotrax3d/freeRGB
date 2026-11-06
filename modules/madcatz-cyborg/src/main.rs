// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"MadCatz Cyborg Gaming Light","vid":0x06A3u16,"pid":0x0DC5u16,"kind":"accessory","leds":1}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_madcatz_cyborg::MadcatzCyborg::default());
}
