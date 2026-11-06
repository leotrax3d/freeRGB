// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream constants from `TurtleBeachKP7Controller.h`.
pub const LED_COUNT: usize = 33;
pub const REPORT_SIZE: usize = 64;
pub const REPORT_ID: u8 = 0xef;
pub const PAGE0_LED_BYTES: usize = 62;
pub const PAGE1_LED_BYTES: usize = 37;

/// Upstream: RGBController_TurtleBeachKP7.cpp static
/// `turtle_beach_kp7_led_order`: maps each LED in zone/presentation order to
/// its slot in the device color buffer.
pub const LED_ORDER: [usize; LED_COUNT] = [
    28, 29, 30, // indicators
    21, 0, 7, 14, // profile row
    8, 22, 15, 1, 11, // number grid row 1 (+ mode key)
    23, 2, 9, 4, 18, // row 2
    10, 24, 16, 25, 5, // row 3
    17, 3, 13, 12, 19, // row 4
    27, 6, 20, // 0 / . / Enter
    26, // space bar
    31, 32, // wrist rest
];

/// Upstream: TurtleBeachKP7Controller.cpp EnterDirectMode. 5-byte feature
/// report on report ID 14 (TURTLE_BEACH_KP7_ENTER_DIRECT_REPORT_ID) that
/// flips the keypad out of its onboard preset effects.
pub fn enter_direct() -> Vec<u8> {
    vec![14, 0x00, 0x01, 0x00, 0x00]
}

/// Upstream: RGBController_TurtleBeachKP7.cpp DeviceUpdateLEDs fills a
/// 99-byte [R, G, B] buffer in device order; TurtleBeachKP7Controller.cpp
/// SendFrame then writes it as two 64-byte output reports (report ID 0xEF,
/// page 0 / page 1, zero padded).
pub fn frame(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut led_data = [0u8; LED_COUNT * 3];
    for (led_idx, c) in colors.iter().enumerate().take(LED_COUNT) {
        let dev_led = LED_ORDER[led_idx];
        led_data[dev_led * 3] = c.r;
        led_data[dev_led * 3 + 1] = c.g;
        led_data[dev_led * 3 + 2] = c.b;
    }
    let mut page0 = vec![0u8; REPORT_SIZE];
    page0[0] = REPORT_ID;
    page0[1] = 0x00;
    page0[2..2 + PAGE0_LED_BYTES].copy_from_slice(&led_data[..PAGE0_LED_BYTES]);
    let mut page1 = vec![0u8; REPORT_SIZE];
    page1[0] = REPORT_ID;
    page1[1] = 0x01;
    page1[2..2 + PAGE1_LED_BYTES].copy_from_slice(&led_data[PAGE0_LED_BYTES..]);
    vec![page0, page1]
}
