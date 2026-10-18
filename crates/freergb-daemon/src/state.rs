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
}

impl Default for State {
    fn default() -> Self {
        Self {
            effect: EffectSpec::RainbowWave {
                speed: 0.2,
                scale: 1.0,
            },
            brightness: 1.0,
        }
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
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            tracing::warn!(path = %path.display(), error = %e, "ignoring invalid state file");
            State::default()
        }),
        Err(_) => State::default(),
    }
}

/// Writes atomically (temp file + rename) so a crash never leaves a torn file.
pub fn save(path: &Path, state: &State) -> std::io::Result<()> {
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
    fn round_trip_and_fallbacks() {
        let dir = std::env::temp_dir().join(format!("freergb-state-test-{}", std::process::id()));
        let path = dir.join("state.json");
        assert_eq!(load(&path), State::default());

        let s = State {
            effect: EffectSpec::Solid { color: Rgb::GREEN },
            brightness: 0.25,
        };
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);

        std::fs::write(&path, "{broken").unwrap();
        assert_eq!(load(&path), State::default());
        let _ = std::fs::remove_dir_all(dir);
    }
}
