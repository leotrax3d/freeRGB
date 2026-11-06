//! Host-owned, read-only platform identity. Missing/denied values stay absent.
use freergb_module_api::PlatformIdentity;
use std::io::{Read, Result};
use std::sync::OnceLock;

/// Maximum UTF-8 bytes per public platform label. Reject rather than truncate:
/// a truncated model could accidentally match a different board.
pub const MAX_LABEL: usize = 256;

fn label(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.len() <= MAX_LABEL && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}

/// Injectable reader; only these three allowlisted names are ever requested.
/// No serial, UUID, BIOS blob or OS path crosses the module boundary.
pub fn read_with(mut read: impl FnMut(&str) -> Result<String>) -> PlatformIdentity {
    PlatformIdentity {
        board_vendor: read("board_vendor").ok().and_then(label),
        board_name: read("board_name").ok().and_then(label),
        product: read("product_name").ok().and_then(label),
    }
}

/// Read Linux sysfs labels from an explicit root (also usable with fixture files).
/// Files are bounded before decoding; unreadable or malformed fields are absent.
pub fn read_sysfs(root: &std::path::Path) -> PlatformIdentity {
    read_with(|key| {
        let mut bytes = Vec::new();
        std::fs::File::open(root.join(key))?
            .take((MAX_LABEL + 2) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_LABEL + 1 {
            return Err(std::io::Error::other("platform label too large"));
        }
        String::from_utf8(bytes).map_err(std::io::Error::other)
    })
}

#[cfg(windows)]
fn read_native() -> PlatformIdentity {
    use winreg::{
        enums::{HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE},
        RegKey,
    };
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"HARDWARE\DESCRIPTION\System\BIOS", KEY_QUERY_VALUE);
    read_with(|field| {
        let key = key
            .as_ref()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        key.get_value(match field {
            "board_vendor" => "BaseBoardManufacturer",
            "board_name" => "BaseBoardProduct",
            "product_name" => "SystemProductName",
            _ => unreachable!("read_with only requests allowlisted labels"),
        })
    })
}

#[cfg(target_os = "linux")]
fn read_native() -> PlatformIdentity {
    read_sysfs(std::path::Path::new("/sys/class/dmi/id"))
}

#[cfg(not(any(windows, target_os = "linux")))]
fn read_native() -> PlatformIdentity {
    PlatformIdentity::default()
}

/// One snapshot per daemon process, read without elevation, subprocesses or writes.
pub fn identity() -> PlatformIdentity {
    static IDENTITY: OnceLock<PlatformIdentity> = OnceLock::new();
    IDENTITY.get_or_init(read_native).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlisted_fields_are_independent_and_missing_values_stay_absent() {
        let mut requested = Vec::new();
        let result = read_with(|field| {
            requested.push(field.to_owned());
            match field {
                "board_vendor" => Ok("  Micro-Star International Co., Ltd.\n".into()),
                "board_name" => Ok("MAG X870 TOMAHAWK WIFI (MS-7E51)\n".into()),
                _ => Err(std::io::ErrorKind::PermissionDenied.into()),
            }
        });
        assert_eq!(requested, ["board_vendor", "board_name", "product_name"]);
        assert_eq!(
            result.board_vendor.as_deref(),
            Some("Micro-Star International Co., Ltd.")
        );
        assert_eq!(
            result.board_name.as_deref(),
            Some("MAG X870 TOMAHAWK WIFI (MS-7E51)")
        );
        assert_eq!(result.product, None);
    }

    #[test]
    fn malformed_labels_are_rejected_without_prefix_matching() {
        for value in [
            "".into(),
            " \n".into(),
            "bad\0name".into(),
            "bad\nname".into(),
            "x".repeat(MAX_LABEL + 1),
        ] {
            assert_eq!(
                read_with(|_| Ok(value.clone())),
                PlatformIdentity::default()
            );
        }
        assert!(label("x".repeat(MAX_LABEL)).is_some());
    }

    #[test]
    fn sysfs_fixture_is_bounded_and_strict_utf8() {
        let root = std::env::temp_dir().join(format!("freergb-dmi-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("board_vendor"), "MSI\n").unwrap();
        std::fs::write(root.join("board_name"), [0xff]).unwrap();
        std::fs::write(root.join("product_name"), "x".repeat(1024)).unwrap();
        let result = read_sysfs(&root);
        assert_eq!(result.board_vendor.as_deref(), Some("MSI"));
        assert_eq!(result.board_name, None);
        assert_eq!(result.product, None);
        std::fs::remove_file(root.join("board_vendor")).unwrap();
        std::fs::remove_file(root.join("board_name")).unwrap();
        std::fs::remove_file(root.join("product_name")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
