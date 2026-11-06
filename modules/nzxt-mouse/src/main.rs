// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"NZXT Mouse","vid":0x1E71u16,"pid":0x2100u16,"usage_page":0xFFCAu16,"usage":0x0001u16,"kind":"mouse","leds":6}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_nzxt_mouse::NzxtMouse::default());
}
