//! Pinned LibreHardwareMonitor runtime. The only source is
//! `packaging/windows/mainboard-packages.json` (read by `build.rs`); an upgrade
//! is one manifest change plus a re-audit of the private LHM metadata used by
//! the native mainboard control adapter.

/// Pinned LHM release, e.g. `0.9.6`.
pub const VERSION: &str = env!("FREERGB_LHM_VERSION");
/// Install-relative runtime directory written by the mainboard installer.
pub const LIBRARY_DIRECTORY: &str = concat!("providers/lhm-", env!("FREERGB_LHM_VERSION"));

#[cfg(test)]
mod tests {
    #[test]
    fn pin_matches_manifest_layout() {
        assert_eq!(super::VERSION.split('.').count(), 3);
        assert_eq!(
            super::LIBRARY_DIRECTORY,
            format!("providers/lhm-{}", super::VERSION)
        );
    }
}
