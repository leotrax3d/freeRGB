//! Library of named custom effects from the editor.
//!
//! Each entry is `<effects dir>/<name>.json` holding one validated
//! [`EffectSpec`]. Names follow the profile rules (letters, digits, space,
//! `-`, `_`; 1–64 characters), so they can never leave the directory.

use crate::profiles::{validate_name, ProfileError};
use freergb_effects::EffectSpec;
use std::path::{Path, PathBuf};

/// `effects/` next to the state file.
pub fn dir_for_state(state_path: &Path) -> Option<PathBuf> {
    state_path.parent().map(|d| d.join("effects"))
}

fn path(dir: &Path, name: &str) -> Result<PathBuf, ProfileError> {
    Ok(dir.join(format!("{}.json", validate_name(name)?)))
}

pub fn list(dir: &Path) -> Result<Vec<String>, ProfileError> {
    crate::profiles::list(dir)
}

pub fn save(dir: &Path, name: &str, spec: &EffectSpec) -> Result<(), ProfileError> {
    spec.validate()
        .map_err(|e| ProfileError::Corrupt(name.to_owned(), e))?;
    let p = path(dir, name)?;
    std::fs::create_dir_all(dir)?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(spec)
            .map_err(|e| ProfileError::Corrupt(name.to_owned(), e.to_string()))?,
    )?;
    std::fs::rename(tmp, p)?;
    Ok(())
}

pub fn load(dir: &Path, name: &str) -> Result<EffectSpec, ProfileError> {
    let p = path(dir, name)?;
    let text = std::fs::read_to_string(&p).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ProfileError::NotFound(name.to_owned()),
        _ => e.into(),
    })?;
    let spec: EffectSpec = serde_json::from_str(&text)
        .map_err(|e| ProfileError::Corrupt(name.to_owned(), e.to_string()))?;
    spec.validate()
        .map_err(|e| ProfileError::Corrupt(name.to_owned(), e))?;
    Ok(spec)
}

pub fn delete(dir: &Path, name: &str) -> Result<(), ProfileError> {
    crate::profiles::delete(dir, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_list_delete_and_validation() {
        let dir = std::env::temp_dir().join(format!("freergb-custom-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let spec = EffectSpec::default_for("layers").unwrap();
        save(&dir, "Disco", &spec).unwrap();
        assert_eq!(list(&dir).unwrap(), vec!["Disco"]);
        assert_eq!(load(&dir, "Disco").unwrap(), spec);

        let invalid = EffectSpec::Layers { layers: vec![] };
        assert!(matches!(
            save(&dir, "Empty", &invalid),
            Err(ProfileError::Corrupt(..))
        ));
        assert!(matches!(
            save(&dir, "../x", &spec),
            Err(ProfileError::InvalidName(_))
        ));

        delete(&dir, "Disco").unwrap();
        assert!(list(&dir).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
