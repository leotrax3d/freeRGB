//! Small i18n layer for the classic GUI. It reads the same flat locale JSON
//! files as the desktop app (embedded by `build.rs`), selects one by the OS UI
//! language and always falls back to English.

use std::collections::HashMap;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

type Catalog = HashMap<String, String>;

fn parse(json: &str) -> Catalog {
    serde_json::from_str(json).unwrap_or_default()
}

fn english() -> &'static Catalog {
    static EN: OnceLock<Catalog> = OnceLock::new();
    EN.get_or_init(|| {
        LOCALES
            .iter()
            .find(|(code, _)| *code == "en")
            .map(|(_, json)| parse(json))
            .unwrap_or_default()
    })
}

/// Picks an embedded locale code for a BCP-47 tag such as `de-DE`; exact tag
/// first, then the primary language, otherwise `en`.
pub fn pick_locale(tag: &str) -> &'static str {
    let tag = tag.replace('_', "-").to_ascii_lowercase();
    let primary = tag.split('-').next().unwrap_or("");
    let find = |wanted: &str| {
        LOCALES
            .iter()
            .map(|(code, _)| *code)
            .find(|code| code.eq_ignore_ascii_case(wanted))
    };
    find(&tag).or_else(|| find(primary)).unwrap_or("en")
}

#[cfg(windows)]
fn os_ui_language() -> Option<String> {
    use windows_sys::Win32::Globalization::{GetUserDefaultUILanguage, LCIDToLocaleName};
    let mut buf = [0u16; 85];
    // SAFETY: plain query calls; the buffer length is passed alongside it.
    let len = unsafe {
        let langid = GetUserDefaultUILanguage();
        LCIDToLocaleName(u32::from(langid), buf.as_mut_ptr(), buf.len() as i32, 0)
    };
    (len > 1).then(|| String::from_utf16_lossy(&buf[..len as usize - 1]))
}

#[cfg(not(windows))]
fn os_ui_language() -> Option<String> {
    None
}

fn active() -> &'static (&'static str, Catalog) {
    static ACTIVE: OnceLock<(&'static str, Catalog)> = OnceLock::new();
    ACTIVE.get_or_init(|| {
        let code = os_ui_language().map_or("en", |tag| pick_locale(&tag));
        let json = LOCALES
            .iter()
            .find(|(c, _)| *c == code)
            .map_or("{}", |(_, j)| j);
        (code, parse(json))
    })
}

/// Translated text for `key`: active locale, then English, then the key.
pub fn tr(key: &str) -> String {
    active()
        .1
        .get(key)
        .or_else(|| english().get(key))
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

/// Fan-safety warnings are always also shown in English, so the wording the
/// user accepts is identical for everyone.
#[allow(dead_code)] // used as fan-safety page texts get localized
pub fn safety(key: &str, english_text: &str) -> String {
    let local = tr(key);
    if active().0 == "en" || local == english_text || local == key {
        english_text.to_owned()
    } else {
        format!("{local}\n{english_text}")
    }
}

/// Stable daemon RPC error code -> locale key.
pub fn error_key(code: i64) -> Option<&'static str> {
    Some(match code {
        -32003 => "error.daemon.profile",
        -32004 => "error.daemon.deviceNotFound",
        -32005 => "error.daemon.refused",
        -32006 => "error.daemon.modulesDisabled",
        -32601 => "error.daemon.unknownMethod",
        -32602 => "error.daemon.invalidParams",
        _ => return None,
    })
}

/// User-facing text for a daemon error. Unknown codes keep the English
/// daemon message; refusals append it as detail (it names the exact reason).
pub fn rpc_error(code: i64, message: &str) -> String {
    match error_key(code) {
        None => format!("daemon error {code}: {message}"),
        Some(key) if matches!(code, -32005 | -32602) && !message.is_empty() => {
            format!("{}: {message}", tr(key))
        }
        Some(key) => tr(key),
    }
}

/// Like [`rpc_error`] for any client error.
pub fn client_error(error: &freergb_client::ClientError) -> String {
    match error {
        freergb_client::ClientError::Rpc { code, message } => rpc_error(*code, message),
        freergb_client::ClientError::Connect(_) => tr("error.daemon.unreachable"),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_selection_falls_back_to_english() {
        assert_eq!(pick_locale("de-DE"), "de");
        assert_eq!(pick_locale("de_AT"), "de");
        assert_eq!(pick_locale("en-US"), "en");
        assert_eq!(pick_locale("xx-YY"), "en");
        assert_eq!(pick_locale(""), "en");
    }

    #[test]
    fn known_codes_map_to_existing_keys() {
        for code in [-32003, -32004, -32005, -32006, -32601, -32602] {
            let key = error_key(code).expect("mapped");
            assert!(english().contains_key(key), "missing en key {key}");
            assert!(parse(LOCALES.iter().find(|(c, _)| *c == "de").unwrap().1).contains_key(key));
        }
        assert!(english().contains_key("error.daemon.unreachable"));
    }

    #[test]
    fn unknown_code_uses_english_text() {
        assert_eq!(rpc_error(-1, "boom"), "daemon error -1: boom");
    }

    #[test]
    fn missing_key_returns_key() {
        assert_eq!(tr("no.such.key"), "no.such.key");
    }

    #[test]
    fn safety_text_is_always_english_too() {
        let text = safety("error.daemon.refused", "Fan control refused.");
        assert!(text.ends_with("Fan control refused."));
    }
}
