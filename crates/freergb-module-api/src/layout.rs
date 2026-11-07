//! Shared keyboard geometry, regional key names and sparse protocol matrix maps.
//!
//! All indices refer to frame/LED order, never to a device handle. A module
//! supplies its own verified matrix and wire addresses; geometry does not imply
//! a particular controller's report format. See `docs/KEYBOARD_LAYOUTS.md`.

use std::collections::{BTreeMap, HashSet};

use freergb_core::{Led, LedLayout};
use serde::{Deserialize, Serialize};

use crate::{ErrorCode, ModuleError, MAX_LEDS};

/// Physical key arrangement, separate from the language printed on the keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalLayout {
    Ansi,
    Iso,
    Jis,
    Custom(String),
}

/// A key in frame order. `id` identifies the physical key across regional
/// variants (for example `KeyY`); `name` is its displayed regional name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    pub id: String,
    pub name: String,
    /// Visual grid `(column, row)`, independent of the protocol matrix.
    pub position: (u16, u16),
}

/// A row-major controller matrix. `None` is a hole, including cells covered by
/// a wide key. Each present value is an index into the keyboard's key vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixMap {
    pub rows: u16,
    pub columns: u16,
    pub cells: Vec<Option<usize>>,
}

fn invalid(message: impl Into<String>) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

impl MatrixMap {
    /// Requires every LED index exactly once; duplicate cells for wide keys
    /// must be represented as holes instead.
    pub fn validate(&self, led_count: usize) -> Result<(), ModuleError> {
        if self.rows == 0 || self.columns == 0 {
            return Err(invalid("keyboard matrix dimensions must be nonzero"));
        }
        let size = usize::from(self.rows) * usize::from(self.columns);
        if size > MAX_LEDS || self.cells.len() != size || led_count > MAX_LEDS {
            return Err(invalid(
                "keyboard matrix size is invalid or exceeds MAX_LEDS",
            ));
        }
        let mut seen = vec![false; led_count];
        for index in self.cells.iter().flatten() {
            let entry = seen
                .get_mut(*index)
                .ok_or_else(|| invalid("keyboard matrix LED index is out of range"))?;
            if *entry {
                return Err(invalid("keyboard matrix repeats an LED index"));
            }
            *entry = true;
        }
        if seen.contains(&false) {
            return Err(invalid("keyboard matrix omits an LED index"));
        }
        Ok(())
    }

    /// Looks up a matrix cell without wrapping an out-of-range column.
    pub fn index_at(&self, row: u16, column: u16) -> Option<usize> {
        if row >= self.rows || column >= self.columns {
            return None;
        }
        self.cells
            .get(usize::from(row) * usize::from(self.columns) + usize::from(column))
            .copied()
            .flatten()
    }
}

/// A complete reusable layout. Keys and wire addresses share frame order.
/// Public data can be deserialized, so validate it before using it directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardLayout {
    /// Unique catalog ID, including model/size and region where appropriate.
    pub id: String,
    pub physical: PhysicalLayout,
    /// Regional identifier chosen by the module, for example `en-US` or `de-DE`.
    pub region: String,
    pub width: u16,
    pub height: u16,
    pub keys: Vec<Key>,
    pub matrix: MatrixMap,
    /// Device-specific LED address for each frame index. Addresses may be
    /// sparse, but must be unique. No sentinel address is reserved.
    pub wire_indices: Vec<u16>,
}

impl KeyboardLayout {
    pub fn validate(&self) -> Result<(), ModuleError> {
        if self.id.trim().is_empty() || self.region.trim().is_empty() {
            return Err(invalid("keyboard layout ID and region must be nonempty"));
        }
        if matches!(&self.physical, PhysicalLayout::Custom(name) if name.trim().is_empty()) {
            return Err(invalid("custom physical layout name must be nonempty"));
        }
        if self.keys.is_empty() || self.keys.len() > MAX_LEDS {
            return Err(invalid("keyboard key count must be in 1..=MAX_LEDS"));
        }
        if self.width == 0 || self.height == 0 {
            return Err(invalid("keyboard visual dimensions must be nonzero"));
        }
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        let mut positions = HashSet::new();
        for key in &self.keys {
            if key.id.trim().is_empty() || key.name.trim().is_empty() {
                return Err(invalid("keyboard key ID and name must be nonempty"));
            }
            if !ids.insert(&key.id) || !names.insert(&key.name) {
                return Err(invalid("keyboard key IDs and names must be unique"));
            }
            if key.position.0 >= self.width || key.position.1 >= self.height {
                return Err(invalid("keyboard key position is outside the visual grid"));
            }
            if !positions.insert(key.position) {
                return Err(invalid("keyboard keys share a visual position"));
            }
        }
        self.matrix.validate(self.keys.len())?;
        if self.wire_indices.len() != self.keys.len()
            || self.wire_indices.iter().collect::<HashSet<_>>().len() != self.keys.len()
        {
            return Err(invalid(
                "keyboard wire indices must match keys and be unique",
            ));
        }
        Ok(())
    }

    /// Builds the existing capability layout without changing frame order.
    pub fn led_layout(&self) -> Result<LedLayout, ModuleError> {
        self.validate()?;
        Ok(LedLayout {
            width: self.width,
            height: self.height,
            leds: self
                .keys
                .iter()
                .map(|key| Led {
                    name: key.name.clone(),
                    pos: key.position,
                })
                .collect(),
        })
    }

    pub fn index_of(&self, key_id: &str) -> Option<usize> {
        self.keys.iter().position(|key| key.id == key_id)
    }

    pub fn wire_index_at(&self, row: u16, column: u16) -> Option<u16> {
        self.wire_indices
            .get(self.matrix.index_at(row, column)?)
            .copied()
    }

    /// Creates a regional variant without modifying geometry, matrix or wire
    /// order. Unknown key IDs and invalid resulting names are rejected.
    pub fn regional_variant(
        &self,
        id: impl Into<String>,
        region: impl Into<String>,
        names: &[(&str, &str)],
    ) -> Result<Self, ModuleError> {
        self.validate()?;
        let mut variant = self.clone();
        variant.id = id.into();
        variant.region = region.into();
        let mut seen = HashSet::new();
        for &(key_id, name) in names {
            if !seen.insert(key_id) {
                return Err(invalid("regional variant repeats a key ID"));
            }
            let index = variant
                .index_of(key_id)
                .ok_or_else(|| invalid(format!("unknown regional key ID: {key_id}")))?;
            variant.keys[index].name = name.into();
        }
        variant.validate()?;
        Ok(variant)
    }
}

/// Validated, deterministic catalog shared by module families. Registration
/// owns its data; callers cannot mutate a registered layout through a lookup.
#[derive(Debug, Clone, Default)]
pub struct KeyboardLayoutManager {
    layouts: BTreeMap<String, KeyboardLayout>,
}

impl KeyboardLayoutManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a model/region layout, rejecting duplicate IDs atomically.
    pub fn register(&mut self, layout: KeyboardLayout) -> Result<(), ModuleError> {
        layout.validate()?;
        if self.layouts.contains_key(&layout.id) {
            return Err(invalid(format!(
                "duplicate keyboard layout ID: {}",
                layout.id
            )));
        }
        self.layouts.insert(layout.id.clone(), layout);
        Ok(())
    }

    /// Exact selection only: an unknown region must never silently select a
    /// different physical or wire layout.
    pub fn get(&self, id: &str) -> Option<&KeyboardLayout> {
        self.layouts.get(id)
    }

    /// Iterates in catalog ID order.
    pub fn layouts(&self) -> impl Iterator<Item = &KeyboardLayout> {
        self.layouts.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Deliberately different visual, matrix and wire orders. These addresses
    // are synthetic and do not claim to describe a hardware controller.
    fn fixture() -> KeyboardLayout {
        KeyboardLayout {
            id: "fixture-ansi-us".into(),
            physical: PhysicalLayout::Ansi,
            region: "en-US".into(),
            width: 4,
            height: 2,
            keys: vec![
                Key {
                    id: "KeyY".into(),
                    name: "Y".into(),
                    position: (2, 0),
                },
                Key {
                    id: "KeyZ".into(),
                    name: "Z".into(),
                    position: (0, 1),
                },
                Key {
                    id: "Enter".into(),
                    name: "Enter".into(),
                    position: (3, 1),
                },
            ],
            matrix: MatrixMap {
                rows: 2,
                columns: 3,
                cells: vec![None, Some(2), Some(0), Some(1), None, None],
            },
            wire_indices: vec![127, 0, u16::MAX],
        }
    }

    #[test]
    fn sparse_matrix_preserves_frame_and_wire_order() {
        let keyboard = fixture();
        let layout = keyboard.led_layout().unwrap();
        layout.validate().unwrap();
        assert_eq!(layout.leds[0].name, "Y");
        assert_eq!(layout.leds[0].pos, (2, 0));
        assert_eq!(keyboard.index_of("Enter"), Some(2));
        assert_eq!(keyboard.index_of("Missing"), None);
        assert_eq!(keyboard.wire_index_at(0, 1), Some(u16::MAX));
        assert_eq!(keyboard.wire_index_at(0, 2), Some(127));
        assert_eq!(keyboard.wire_index_at(1, 0), Some(0));
        for (row, column) in [(0, 0), (0, 3), (2, 0), (u16::MAX, u16::MAX)] {
            assert_eq!(keyboard.wire_index_at(row, column), None);
        }
    }

    #[test]
    fn regional_names_do_not_change_physical_keys_or_addresses() {
        let original = fixture();
        let german = original
            .regional_variant("fixture-ansi-de", "de-DE", &[("KeyY", "Z"), ("KeyZ", "Y")])
            .unwrap();
        assert_eq!(german.keys[0].name, "Z");
        assert_eq!(german.keys[0].id, "KeyY");
        assert_eq!(german.matrix, original.matrix);
        assert_eq!(german.wire_indices, original.wire_indices);
        assert_eq!(original.keys[0].name, "Y");
        for names in [
            vec![("Missing", "X")],
            vec![("KeyY", "")],
            vec![("KeyY", "Z")],
            vec![("KeyY", "X"), ("KeyY", "W")],
        ] {
            assert!(original
                .regional_variant("invalid", "de-DE", &names)
                .is_err());
        }
    }

    #[test]
    fn catalog_rejects_duplicates_and_invalid_entries_without_replacement() {
        let mut manager = KeyboardLayoutManager::new();
        let original = fixture();
        manager.register(original.clone()).unwrap();
        let mut duplicate = original.clone();
        duplicate.region = "de-DE".into();
        assert!(manager.register(duplicate).is_err());
        let mut invalid = original.clone();
        invalid.id = "invalid".into();
        invalid.keys.clear();
        assert!(manager.register(invalid).is_err());
        assert_eq!(manager.get(&original.id), Some(&original));
        assert!(manager.get("invalid").is_none());
        assert!(manager.get("unknown-region").is_none());
        let variant = original.regional_variant("aaa", "en-GB", &[]).unwrap();
        manager.register(variant).unwrap();
        assert_eq!(
            manager
                .layouts()
                .map(|layout| layout.id.as_str())
                .collect::<Vec<_>>(),
            ["aaa", "fixture-ansi-us"]
        );
    }

    #[test]
    fn rejects_malformed_matrix_maps() {
        for matrix in [
            MatrixMap {
                rows: 0,
                columns: 1,
                cells: vec![],
            },
            MatrixMap {
                rows: 1,
                columns: 0,
                cells: vec![],
            },
            MatrixMap {
                rows: u16::MAX,
                columns: u16::MAX,
                cells: vec![],
            },
            MatrixMap {
                rows: 1,
                columns: 2,
                cells: vec![Some(0)],
            },
            MatrixMap {
                rows: 1,
                columns: 3,
                cells: vec![Some(0), Some(1), Some(3)],
            },
            MatrixMap {
                rows: 1,
                columns: 3,
                cells: vec![Some(0), Some(1), Some(1)],
            },
            MatrixMap {
                rows: 1,
                columns: 3,
                cells: vec![Some(0), Some(1), None],
            },
        ] {
            assert!(matrix.validate(3).is_err(), "{matrix:?}");
        }
    }

    #[test]
    fn rejects_invalid_keyboard_metadata_geometry_and_wire_maps() {
        let mutations: &[fn(&mut KeyboardLayout)] = &[
            |l| l.id.clear(),
            |l| l.region = " ".into(),
            |l| l.physical = PhysicalLayout::Custom(String::new()),
            |l| l.width = 0,
            |l| l.height = 0,
            |l| l.keys.clear(),
            |l| l.keys = vec![l.keys[0].clone(); MAX_LEDS + 1],
            |l| l.keys[0].id.clear(),
            |l| l.keys[0].name.clear(),
            |l| l.keys[0].id = l.keys[1].id.clone(),
            |l| l.keys[0].name = l.keys[1].name.clone(),
            |l| l.keys[0].position = (l.width, 0),
            |l| l.keys[0].position = (0, l.height),
            |l| l.keys[0].position = l.keys[1].position,
            |l| {
                l.wire_indices.pop();
            },
            |l| l.wire_indices[0] = l.wire_indices[1],
        ];
        for mutate in mutations {
            let mut layout = fixture();
            mutate(&mut layout);
            assert!(layout.validate().is_err(), "{layout:?}");
            assert!(layout.led_layout().is_err());
        }
    }

    #[test]
    fn serde_round_trip_and_untrusted_data_validation() {
        for physical in [
            PhysicalLayout::Ansi,
            PhysicalLayout::Iso,
            PhysicalLayout::Jis,
            PhysicalLayout::Custom("split".into()),
        ] {
            let mut layout = fixture();
            layout.physical = physical;
            let json = serde_json::to_value(&layout).unwrap();
            assert!(json["matrix"]["cells"][0].is_null());
            let back: KeyboardLayout = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(back, layout);
            back.validate().unwrap();
            let mut malformed = json;
            malformed["matrix"]["cells"][1] = serde_json::json!(999);
            let back: KeyboardLayout = serde_json::from_value(malformed).unwrap();
            assert!(KeyboardLayoutManager::new().register(back).is_err());
        }
    }
}
