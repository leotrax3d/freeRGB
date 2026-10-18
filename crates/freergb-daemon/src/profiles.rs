//! Named profiles: saved effect + brightness combinations.
//!
//! Each profile is `<profiles dir>/<name>.json` with the same content as the
//! persisted state. Names are restricted to letters, digits, space, `-` and
//! `_` (1–64 chars) so a name can never escape the profiles directory.

use crate::state::{self, State};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("invalid profile name {0:?} (1-64 chars: letters, digits, space, '-', '_')")]
    InvalidName(String),
    #[error("profile {0:?} does not exist")]
    NotFound(String),
    #[error("profile {0:?} is not valid JSON: {1}")]
    Corrupt(String, String),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
}

pub fn validate_name(name: &str) -> Result<&str, ProfileError> {
    let ok = !name.is_empty()
        && name.chars().count() <= 64
        && name.trim() == name
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'));
    if ok {
        Ok(name)
    } else {
        Err(ProfileError::InvalidName(name.to_owned()))
    }
}

/// `profiles/` next to the state file.
pub fn dir_for_state(state_path: &Path) -> Option<PathBuf> {
    state_path.parent().map(|d| d.join("profiles"))
}

fn path(dir: &Path, name: &str) -> Result<PathBuf, ProfileError> {
    Ok(dir.join(format!("{}.json", validate_name(name)?)))
}

/// Sorted profile names.
pub fn list(dir: &Path) -> Result<Vec<String>, ProfileError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let p = e.path();
            (p.extension()? == "json")
                .then(|| p.file_stem()?.to_str().map(str::to_owned))
                .flatten()
        })
        .filter(|n| validate_name(n).is_ok())
        .collect();
    names.sort();
    Ok(names)
}

pub fn save(dir: &Path, name: &str, s: &State) -> Result<(), ProfileError> {
    let p = path(dir, name)?;
    state::save(&p, s)?;
    Ok(())
}

pub fn load(dir: &Path, name: &str) -> Result<State, ProfileError> {
    let p = path(dir, name)?;
    let text = std::fs::read_to_string(&p).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ProfileError::NotFound(name.to_owned()),
        _ => e.into(),
    })?;
    serde_json::from_str(&text).map_err(|e| ProfileError::Corrupt(name.to_owned(), e.to_string()))
}

pub fn delete(dir: &Path, name: &str) -> Result<(), ProfileError> {
    let p = path(dir, name)?;
    std::fs::remove_file(&p).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ProfileError::NotFound(name.to_owned()),
        _ => e.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;
    use freergb_effects::EffectSpec;

    fn tmp_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("freergb-profiles-{tag}-{}", std::process::id()))
    }

    #[test]
    fn name_validation_blocks_path_tricks() {
        for ok in ["Gaming", "Work 2", "night_mode", "Ärger-frei"] {
            assert!(validate_name(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "../x",
            "a/b",
            "a\\b",
            "C:",
            " lead",
            "trail ",
            "x.json",
            &"x".repeat(65),
        ] {
            assert!(validate_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn save_list_load_delete() {
        let dir = tmp_dir("crud");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(list(&dir).unwrap().is_empty(), "missing dir = no profiles");

        let a = State {
            effect: EffectSpec::Solid { color: Rgb::RED },
            brightness: 0.3,
            ..Default::default()
        };
        save(&dir, "Zeta", &a).unwrap();
        save(&dir, "Alpha", &State::default()).unwrap();
        assert_eq!(list(&dir).unwrap(), vec!["Alpha", "Zeta"]);
        assert_eq!(load(&dir, "Zeta").unwrap(), a);

        assert!(matches!(load(&dir, "nope"), Err(ProfileError::NotFound(_))));
        assert!(matches!(
            save(&dir, "../evil", &a),
            Err(ProfileError::InvalidName(_))
        ));

        std::fs::write(dir.join("Broken.json"), "{").unwrap();
        assert!(matches!(
            load(&dir, "Broken"),
            Err(ProfileError::Corrupt(..))
        ));

        delete(&dir, "Zeta").unwrap();
        assert!(matches!(
            delete(&dir, "Zeta"),
            Err(ProfileError::NotFound(_))
        ));
        assert_eq!(list(&dir).unwrap(), vec!["Alpha", "Broken"]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
