//! Persisted user state (current effect and brightness), so lighting survives a
//! daemon restart. Stored as JSON in the per-user config directory:
//! Windows `%APPDATA%\freeRGB\state.json`, Linux `$XDG_CONFIG_HOME/freergb/state.json`
//! (fallback `~/.config/freergb`).

use freergb_effects::EffectSpec;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub effect: EffectSpec,
    pub brightness: f32,
    /// Per-device brightness (0..=1) by device id; missing = 1.0.
    #[serde(default)]
    pub device_brightness: std::collections::BTreeMap<String, f32>,
    /// Where each device sits on the effect canvas, by device id; missing = full canvas.
    #[serde(default)]
    pub device_regions: std::collections::BTreeMap<String, freergb_core::Region>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            effect: EffectSpec::RainbowWave {
                speed: 0.2,
                scale: 1.0,
            },
            brightness: 1.0,
            device_brightness: Default::default(),
            device_regions: Default::default(),
        }
    }
}

impl State {
    pub fn validate(&self) -> Result<(), String> {
        self.effect.validate()?;
        if !(0.0..=1.0).contains(&self.brightness)
            || self
                .device_brightness
                .values()
                .any(|v| !(0.0..=1.0).contains(v))
        {
            return Err("brightness must be finite and in 0..=1".into());
        }
        if self.device_regions.values().any(|r| !r.is_valid()) {
            return Err("invalid device region".into());
        }
        Ok(())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let state: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        state.validate()?;
        Ok(state)
    }
}

pub fn default_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let dir = std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("freeRGB"));
    #[cfg(not(windows))]
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|d| d.join("freergb"));
    dir.map(|d| d.join("state.json"))
}

/// Loads state; a missing or unreadable file yields the default (logged).
pub fn load(path: &Path) -> State {
    match std::fs::read_to_string(path) {
        Ok(s) => State::from_json(&s).unwrap_or_else(|e| {
            tracing::warn!(path = %path.display(), error = %e, "ignoring invalid state file");
            State::default()
        }),
        Err(_) => State::default(),
    }
}

/// Writes atomically (temp file + rename) so a crash never leaves a torn file.
pub fn save(path: &Path, state: &State) -> std::io::Result<()> {
    state
        .validate()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(state)?)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;

    #[test]
    fn rejects_semantically_invalid_state_without_overwriting_good_state() {
        let dir =
            std::env::temp_dir().join(format!("freergb-state-invalid-{}", std::process::id()));
        let path = dir.join("state.json");
        save(&path, &State::default()).unwrap();
        let mut bad = State {
            brightness: 1.1,
            ..State::default()
        };
        assert!(save(&path, &bad).is_err());
        assert_eq!(load(&path), State::default());
        assert!(State::from_json(&serde_json::to_string(&bad).unwrap()).is_err());
        bad.brightness = 1.0;
        bad.device_brightness.insert("test".into(), -1.0);
        assert!(bad.validate().is_err());
        bad.device_brightness.clear();
        bad.effect = EffectSpec::Layers { layers: vec![] };
        assert!(bad.validate().is_err());
        std::fs::write(&path, serde_json::to_string(&bad).unwrap()).unwrap();
        assert_eq!(load(&path), State::default());
        bad.effect = State::default().effect;
        bad.device_regions.insert(
            "test".into(),
            freergb_core::Region {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 1.0,
            },
        );
        assert!(bad.validate().is_err());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn round_trip_and_fallbacks() {
        let dir = std::env::temp_dir().join(format!("freergb-state-test-{}", std::process::id()));
        let path = dir.join("state.json");
        assert_eq!(load(&path), State::default());

        let s = State {
            effect: EffectSpec::Solid { color: Rgb::GREEN },
            brightness: 0.25,
            ..Default::default()
        };
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);

        std::fs::write(&path, "{broken").unwrap();
        assert_eq!(load(&path), State::default());
        let _ = std::fs::remove_dir_all(dir);
    }
}
