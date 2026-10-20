//! Bounded keyboard lighting overlay. No OS hooks, text, device I/O or GUI types.
use freergb_core::{Frame, Rgb};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReactiveConfig {
    #[serde(default = "version")]
    pub version: u32,
    pub enabled: bool,
    pub color: Rgb,
    pub fade_ms: u32,
    /// ISO's Enter-adjacent key uses usage 0x32 rather than ANSI's 0x31.
    pub iso: bool,
    /// Explicit target keyboard IDs; never implicitly illuminate other devices.
    pub devices: BTreeSet<String>,
}
fn version() -> u32 {
    1
}
impl Default for ReactiveConfig {
    fn default() -> Self {
        Self {
            version: 1,
            enabled: false,
            color: Rgb::WHITE,
            fade_ms: 650,
            iso: true,
            devices: BTreeSet::new(),
        }
    }
}
impl ReactiveConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !(50..=5000).contains(&self.fade_ms)
            || self.devices.len() > 64
            || self
                .devices
                .iter()
                .any(|id| id.is_empty() || id.len() > 1024 || id.chars().any(char::is_control))
        {
            return Err("invalid keyboard reactive configuration (version 1, fade 50..5000 ms, at most 64 device IDs)".into());
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct KeyPulses {
    keys: BTreeMap<u16, u64>,
}
impl KeyPulses {
    pub fn clear(&mut self) {
        self.keys.clear();
    }
    pub fn press(&mut self, usage: u16, now_ms: u64) {
        if (4..=0x73).contains(&usage) || (0xe0..=0xe7).contains(&usage) {
            self.keys.insert(usage, now_ms);
        }
    }
    pub fn expire(&mut self, now_ms: u64, fade_ms: u32) {
        self.keys
            .retain(|_, at| now_ms >= *at && now_ms - *at < u64::from(fade_ms));
    }
    pub fn apply(
        &self,
        config: &ReactiveConfig,
        map: &BTreeMap<u16, usize>,
        frame: &mut Frame,
        now_ms: u64,
    ) {
        for (&usage, &at) in &self.keys {
            let usage = if config.iso && usage == 0x31 {
                0x32
            } else {
                usage
            };
            let Some(&index) = map.get(&usage) else {
                continue;
            };
            let Some(age) = now_ms.checked_sub(at) else {
                continue;
            };
            if age >= u64::from(config.fade_ms) {
                continue;
            }
            if let Some(color) = frame.colors_mut().get_mut(index) {
                let alpha = 1.0 - age as f32 / config.fade_ms.max(1) as f32;
                *color = color.lerp(config.color, alpha);
            }
        }
    }
}

/// Physical Windows set-1 scan positions -> USB keyboard page usages. Never VK
/// letters: those would swap physical Y/Z or A/Q on different language layouts.
pub fn scan_to_usage(scan: u16, extended: bool) -> Option<u16> {
    if extended {
        return Some(match scan {
            0x1c => 88,
            0x1d => 0xe4,
            0x35 => 84,
            0x37 => 70,
            0x38 => 0xe6,
            0x47 => 74,
            0x48 => 82,
            0x49 => 75,
            0x4b => 80,
            0x4d => 79,
            0x4f => 77,
            0x50 => 81,
            0x51 => 78,
            0x52 => 73,
            0x53 => 76,
            0x5b => 0xe3,
            0x5c => 0xe7,
            0x5d => 101,
            _ => return None,
        });
    }
    if (0x02..=0x0b).contains(&scan) {
        return Some(30 + scan - 2);
    }
    if (0x3b..=0x44).contains(&scan) {
        return Some(58 + scan - 0x3b);
    }
    Some(match scan {
        0xe1 => 72,
        0x01 => 41,
        0x0c => 45,
        0x0d => 46,
        0x0e => 42,
        0x0f => 43,
        0x10 => 20,
        0x11 => 26,
        0x12 => 8,
        0x13 => 21,
        0x14 => 23,
        0x15 => 28,
        0x16 => 24,
        0x17 => 12,
        0x18 => 18,
        0x19 => 19,
        0x1a => 47,
        0x1b => 48,
        0x1c => 40,
        0x1d => 0xe0,
        0x1e => 4,
        0x1f => 22,
        0x20 => 7,
        0x21 => 9,
        0x22 => 10,
        0x23 => 11,
        0x24 => 13,
        0x25 => 14,
        0x26 => 15,
        0x27 => 51,
        0x28 => 52,
        0x29 => 53,
        0x2a => 0xe1,
        0x2b => 49,
        0x2c => 29,
        0x2d => 27,
        0x2e => 6,
        0x2f => 25,
        0x30 => 5,
        0x31 => 17,
        0x32 => 16,
        0x33 => 54,
        0x34 => 55,
        0x35 => 56,
        0x36 => 0xe5,
        0x37 => 85,
        0x38 => 0xe2,
        0x39 => 44,
        0x3a => 57,
        0x45 => 83,
        0x46 => 71,
        0x47 => 95,
        0x48 => 96,
        0x49 => 97,
        0x4a => 86,
        0x4b => 92,
        0x4c => 93,
        0x4d => 94,
        0x4e => 87,
        0x4f => 89,
        0x50 => 90,
        0x51 => 91,
        0x52 => 98,
        0x53 => 99,
        0x56 => 100,
        0x57 => 68,
        0x58 => 69,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_positions_modifiers_and_keypad_are_distinct() {
        assert_eq!(scan_to_usage(0x15, false), Some(28)); // physical Y, even on DE
        assert_eq!(scan_to_usage(0x2c, false), Some(29));
        assert_eq!(scan_to_usage(0x1c, false), Some(40));
        assert_eq!(scan_to_usage(0x1c, true), Some(88));
        assert_eq!(scan_to_usage(0x1d, true), Some(0xe4));
        assert_eq!(scan_to_usage(0x56, false), Some(100));
        assert_eq!(scan_to_usage(0xe1, false), Some(72));
        assert_eq!(scan_to_usage(0x45, false), Some(83));
        assert_eq!(scan_to_usage(0xff, false), None);
    }
    #[test]
    fn overlay_fades_only_mapped_keys_and_clears_on_clock_regression() {
        let mut pulses = KeyPulses::default();
        let config = ReactiveConfig {
            color: Rgb::RED,
            fade_ms: 100,
            ..Default::default()
        };
        let map = BTreeMap::from([(4, 0), (5, 1)]);
        pulses.press(4, 10);
        let mut frame = Frame::solid(2, Rgb::BLUE);
        pulses.apply(&config, &map, &mut frame, 60);
        assert_eq!(frame.colors(), &[Rgb::new(128, 0, 128), Rgb::BLUE]);
        pulses.expire(110, 100);
        let mut frame = Frame::solid(2, Rgb::BLUE);
        pulses.apply(&config, &map, &mut frame, 110);
        assert_eq!(frame, Frame::solid(2, Rgb::BLUE));
        pulses.press(4, 1000);
        pulses.expire(1, 100);
        pulses.apply(&config, &map, &mut frame, 1000);
        assert_eq!(frame, Frame::solid(2, Rgb::BLUE));
    }
    #[test]
    fn iso_and_ansi_enter_adjacent_keys_select_different_leds() {
        let map = BTreeMap::from([(49, 0), (50, 1)]);
        let mut pulses = KeyPulses::default();
        pulses.press(49, 0);
        let mut config = ReactiveConfig {
            color: Rgb::RED,
            ..Default::default()
        };
        let mut frame = Frame::solid(2, Rgb::BLUE);
        pulses.apply(&config, &map, &mut frame, 0);
        assert_eq!(frame.colors(), &[Rgb::BLUE, Rgb::RED]);
        config.iso = false;
        let mut frame = Frame::solid(2, Rgb::BLUE);
        pulses.apply(&config, &map, &mut frame, 0);
        assert_eq!(frame.colors(), &[Rgb::RED, Rgb::BLUE]);
    }
}
