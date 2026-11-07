//! Optional hardware-mode extension v1 inside the HID API-1 envelope.
//! Numeric flags, directions and colour types correspond to OpenRGB's
//! RGBControllerInterface.h at bbc77470a7cba56af60989e54ba910d5ac66a3ac.
//! This is an independently implemented data contract, not a controller port.
use crate::{ErrorCode, ModuleError, MAX_LEDS};
use freergb_core::Rgb;
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const SPEED: u32 = 1;
pub const DIRECTION_LR: u32 = 1 << 1;
pub const DIRECTION_UD: u32 = 1 << 2;
pub const DIRECTION_HV: u32 = 1 << 3;
pub const BRIGHTNESS: u32 = 1 << 4;
pub const PER_LED_COLOR: u32 = 1 << 5;
pub const MODE_COLOR: u32 = 1 << 6;
pub const RANDOM_COLOR: u32 = 1 << 7;
pub const MANUAL_SAVE: u32 = 1 << 8;
pub const AUTOMATIC_SAVE: u32 = 1 << 9;
pub const ENTIRE_DEVICE: u32 = 1 << 10;
pub const DIRECTION_DIAG: u32 = 1 << 11;

fn invalid(message: &str) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Range {
    pub min: u32,
    pub max: u32,
}

impl Range {
    fn accepts(&self, value: u32) -> bool {
        (self.min.min(self.max)..=self.min.max(self.max)).contains(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    /// Stable module-defined id; `value` is the upstream device mode value.
    pub id: String,
    pub name: String,
    pub value: u32,
    pub flags: u32,
    pub speed: Option<Range>,
    pub brightness: Option<Range>,
    pub colors: Range,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub version: u32,
    pub modes: Vec<Mode>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self {
            version: VERSION,
            modes: Vec::new(),
        }
    }
}

impl Catalog {
    pub fn validate(&self) -> Result<(), ModuleError> {
        if self.version != VERSION || self.modes.len() > 128 {
            return Err(invalid("unsupported or oversized mode catalog"));
        }
        let mut ids = std::collections::BTreeSet::new();
        for m in &self.modes {
            if m.id.is_empty()
                || m.id.len() > 128
                || m.name.is_empty()
                || m.name.len() > 256
                || !ids.insert(&m.id)
                || m.flags & !0xfff != 0
                || m.flags & AUTOMATIC_SAVE != 0
                || m.speed.is_some() != (m.flags & SPEED != 0)
                || m.brightness.is_some() != (m.flags & BRIGHTNESS != 0)
                || m.colors.min > m.colors.max
                || m.colors.max as usize > MAX_LEDS
                || (m.flags & (PER_LED_COLOR | MODE_COLOR) == 0 && m.colors.max != 0)
            {
                return Err(invalid(
                    "invalid mode descriptor (implicit persistence is forbidden)",
                ));
            }
        }
        Ok(())
    }

    pub fn selection(&self, s: &Selection, leds: usize) -> Result<&Mode, ModuleError> {
        let m = self
            .modes
            .iter()
            .find(|m| m.id == s.id)
            .ok_or_else(|| invalid("unknown hardware mode"))?;
        for (range, value) in [(&m.speed, s.speed), (&m.brightness, s.brightness)] {
            if !match (range, value) {
                (None, None) => true,
                (Some(r), Some(v)) => r.accepts(v),
                _ => false,
            } {
                return Err(invalid(
                    "missing, unsupported or out-of-range mode parameter",
                ));
            }
        }
        let directions = m.flags & (DIRECTION_LR | DIRECTION_UD | DIRECTION_HV | DIRECTION_DIAG);
        let direction_flag = match s.direction {
            Some(0..=1) => DIRECTION_LR,
            Some(2..=3) => DIRECTION_UD,
            Some(4..=5) => DIRECTION_HV,
            Some(6..=9) => DIRECTION_DIAG,
            None if directions == 0 => 0,
            _ => return Err(invalid("invalid mode direction")),
        };
        if direction_flag != 0 && directions & direction_flag == 0 {
            return Err(invalid("unsupported mode direction"));
        }
        let valid_colors = match s.color_type {
            0 => m.flags & (PER_LED_COLOR | MODE_COLOR | RANDOM_COLOR) == 0 && s.colors.is_empty(),
            1 => m.flags & PER_LED_COLOR != 0 && s.colors.len() == leds,
            2 => {
                m.flags & MODE_COLOR != 0
                    && (m.colors.min..=m.colors.max).contains(&(s.colors.len() as u32))
            }
            3 => m.flags & RANDOM_COLOR != 0 && s.colors.is_empty(),
            _ => false,
        };
        if !valid_colors || s.colors.len() > MAX_LEDS {
            return Err(invalid("unsupported color type or color count"));
        }
        Ok(m)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub id: String,
    pub speed: Option<u32>,
    pub direction: Option<u32>,
    pub brightness: Option<u32>,
    /// OpenRGB: 0 none, 1 per LED, 2 mode-specific, 3 random.
    pub color_type: u32,
    pub colors: Vec<Rgb>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    List,
    Set { selection: Selection },
    Direct,
    Save { confirmed: bool },
}

/// Shared validation/state for host and SDK. State changes only after an ack.
#[derive(Debug, Clone, Default, Serialize)]
pub struct State {
    pub catalog: Catalog,
    pub active: Option<Selection>,
}

impl State {
    pub fn check(&self, command: &Command, leds: usize) -> Result<(), ModuleError> {
        if self.catalog.modes.is_empty() {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                "hardware modes unavailable",
            ));
        }
        match command {
            Command::Set { selection } => {
                self.catalog.selection(selection, leds)?;
            }
            Command::Save { confirmed } => {
                if !confirmed {
                    return Err(ModuleError::new(
                        ErrorCode::Permission,
                        "save requires explicit user confirmation",
                    ));
                }
                let active = self
                    .active
                    .as_ref()
                    .ok_or_else(|| invalid("no active hardware mode"))?;
                if self.catalog.selection(active, leds)?.flags & MANUAL_SAVE == 0 {
                    return Err(ModuleError::new(
                        ErrorCode::Unsupported,
                        "mode cannot be saved",
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn acknowledge(&mut self, command: &Command) {
        match command {
            Command::Set { selection } => self.active = Some(selection.clone()),
            Command::Direct => self.active = None,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog {
            version: VERSION,
            modes: vec![Mode {
                id: "wave".into(),
                name: "Wave".into(),
                value: 9,
                flags: SPEED | BRIGHTNESS | DIRECTION_LR | MODE_COLOR | RANDOM_COLOR | MANUAL_SAVE,
                speed: Some(Range { min: 5, max: 1 }),
                brightness: Some(Range { min: 0, max: 100 }),
                colors: Range { min: 1, max: 2 },
            }],
        }
    }
    fn selection() -> Selection {
        Selection {
            id: "wave".into(),
            speed: Some(3),
            brightness: Some(50),
            direction: Some(0),
            color_type: 2,
            colors: vec![Rgb { r: 1, g: 2, b: 3 }],
        }
    }

    #[test]
    fn validates_flags_ranges_directions_color_types_and_save_consent() {
        let c = catalog();
        c.validate().unwrap();
        c.selection(&selection(), 2).unwrap();
        for s in [
            Selection {
                speed: None,
                ..selection()
            },
            Selection {
                speed: Some(6),
                ..selection()
            },
            Selection {
                brightness: Some(101),
                ..selection()
            },
            Selection {
                direction: Some(2),
                ..selection()
            },
            Selection {
                direction: None,
                ..selection()
            },
            Selection {
                colors: vec![],
                ..selection()
            },
            Selection {
                colors: vec![Rgb::default(); 3],
                ..selection()
            },
            Selection {
                color_type: 0,
                ..selection()
            },
            Selection {
                id: "unknown".into(),
                ..selection()
            },
        ] {
            assert!(c.selection(&s, 2).is_err(), "{s:?}");
        }
        c.selection(
            &Selection {
                color_type: 3,
                colors: vec![],
                ..selection()
            },
            2,
        )
        .unwrap();
        let mut state = State {
            catalog: c,
            active: None,
        };
        assert!(state.check(&Command::Save { confirmed: true }, 2).is_err());
        state.acknowledge(&Command::Set {
            selection: selection(),
        });
        assert_eq!(
            state
                .check(&Command::Save { confirmed: false }, 2)
                .unwrap_err()
                .code,
            ErrorCode::Permission
        );
        state.check(&Command::Save { confirmed: true }, 2).unwrap();
        state.catalog.modes[0].flags &= !MANUAL_SAVE;
        assert!(state.check(&Command::Save { confirmed: true }, 2).is_err());
        state.acknowledge(&Command::Direct);
        assert!(state.active.is_none());
    }

    #[test]
    fn rejects_bad_catalogs_and_implicit_persistence() {
        for mutate in [
            |c: &mut Catalog| c.version = 2,
            |c: &mut Catalog| c.modes.push(c.modes[0].clone()),
            |c: &mut Catalog| c.modes[0].flags |= AUTOMATIC_SAVE,
            |c: &mut Catalog| c.modes[0].flags |= 1 << 12,
            |c: &mut Catalog| c.modes[0].speed = None,
            |c: &mut Catalog| c.modes[0].colors.max = MAX_LEDS as u32 + 1,
            |c: &mut Catalog| c.modes[0].colors.min = 3,
        ] {
            let mut c = catalog();
            mutate(&mut c);
            assert!(c.validate().is_err());
        }
        assert!(serde_json::from_value::<Command>(serde_json::json!({"op":"save"})).is_err());
        assert!(serde_json::from_value::<Command>(
            serde_json::json!({"op":"set", "selection":selection(), "save":true})
        )
        .is_err());
    }

    #[test]
    fn flag_values_match_the_pinned_openrgb_interface_contract() {
        assert_eq!(
            [
                SPEED,
                DIRECTION_LR,
                DIRECTION_UD,
                DIRECTION_HV,
                BRIGHTNESS,
                PER_LED_COLOR,
                MODE_COLOR,
                RANDOM_COLOR,
                MANUAL_SAVE,
                AUTOMATIC_SAVE,
                ENTIRE_DEVICE,
                DIRECTION_DIAG
            ],
            [0x001, 0x002, 0x004, 0x008, 0x010, 0x020, 0x040, 0x080, 0x100, 0x200, 0x400, 0x800]
        );
    }

    #[test]
    fn per_led_and_all_direction_groups_match_openrgb_semantics() {
        let mut c = catalog();
        c.modes[0].flags =
            PER_LED_COLOR | DIRECTION_LR | DIRECTION_UD | DIRECTION_HV | DIRECTION_DIAG;
        c.modes[0].speed = None;
        c.modes[0].brightness = None;
        for direction in 0..10 {
            let s = Selection {
                speed: None,
                brightness: None,
                direction: Some(direction),
                color_type: 1,
                colors: vec![Rgb::default(); 2],
                ..selection()
            };
            c.selection(&s, 2).unwrap();
            assert!(c.selection(&s, 3).is_err());
        }
    }
}
