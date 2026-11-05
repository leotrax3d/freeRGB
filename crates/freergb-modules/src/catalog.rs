//! Signed module catalog: what modules exist, which devices they claim and
//! where their packages are. Matching runs locally; the catalog is fetched as
//! one file, so the server never learns which hardware is attached.
use crate::trust::TrustedKeys;
use crate::{invalid, Result};
use freergb_module_api::{DeviceKind, InterfaceInfo};
use serde::{Deserialize, Serialize};

pub const CATALOG_SCHEMA: u32 = 1;
pub const BUS_CATALOG_SCHEMA: u32 = 2;
/// Adds API 3 cooling plugins (no device matches; policy in the package).
pub const COOLING_CATALOG_SCHEMA: u32 = 3;
pub const MAX_CATALOG: usize = 1024 * 1024;
pub const MAX_PACKAGE: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema: u32,
    /// Monotonic; a lower sequence than the last accepted one is rejected.
    pub sequence: u64,
    pub generated: String,
    pub modules: Vec<ModuleEntry>,
    /// Vendors whose unmatched devices are shown as "not yet supported".
    #[serde(default)]
    pub relevant_vendors: Vec<Vendor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Vendor {
    pub vid: u16,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModuleEntry {
    pub id: String,
    pub version: String,
    pub family: String,
    pub name: String,
    /// Module protocol versions the module speaks.
    pub api: Vec<u32>,
    pub license: String,
    pub origin: Origin,
    pub devices: Vec<DeviceMatch>,
    pub artifacts: Vec<Artifact>,
    /// Complete corresponding source for the artifacts (GPL section 3).
    pub source: Artifact,
    /// Other module ids that must be installed as well.
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub project: String,
    pub url: String,
    pub commit: String,
}

/// Identification criteria. `None` means "any"; every given value must match
/// (same rule as OpenRGB's HID detector registration).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeviceMatch {
    pub name: String,
    pub kind: DeviceKind,
    pub vid: u16,
    pub pid: u16,
    #[serde(default)]
    pub interface: Option<i32>,
    #[serde(default)]
    pub usage_page: Option<u16>,
    #[serde(default)]
    pub usage: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// Rust target triple, or "source" for the source package.
    pub target: String,
    /// Asset name inside the configured source, or an absolute https URL.
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

impl DeviceMatch {
    pub fn matches(&self, i: &InterfaceInfo) -> bool {
        self.vid == i.vid
            && self.pid == i.pid
            && self.interface.is_none_or(|v| v == i.interface)
            && self.usage_page.is_none_or(|v| v == i.usage_page)
            && self.usage.is_none_or(|v| v == i.usage)
    }
}

pub fn host_target() -> &'static str {
    if cfg!(all(windows, target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else {
        "unsupported"
    }
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn valid_version(v: &str) -> bool {
    let parts: Vec<_> = v.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 6 && p.bytes().all(|b| b.is_ascii_digit()))
}

pub fn version_key(v: &str) -> (u64, u64, u64) {
    let mut it = v.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    (
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
    )
}

pub(crate) fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn valid_asset(a: &Artifact) -> bool {
    valid_sha(&a.sha256)
        && a.size > 0
        && a.size <= MAX_PACKAGE
        && !a.url.is_empty()
        && a.url.len() <= 512
        && (a.url.starts_with("https://")
            || a.url
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)))
}

impl Catalog {
    /// Verifies the detached signature first, then parses and validates.
    pub fn verify(bytes: &[u8], signature: &[u8], keys: &TrustedKeys) -> Result<Self> {
        if bytes.len() > MAX_CATALOG {
            return Err(invalid("catalog exceeds size limit"));
        }
        keys.verify(bytes, signature)?;
        let catalog: Catalog = serde_json::from_slice(bytes).map_err(invalid)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<()> {
        if !(CATALOG_SCHEMA..=COOLING_CATALOG_SCHEMA).contains(&self.schema) {
            return Err(invalid("unsupported catalog schema"));
        }
        let mut ids = std::collections::BTreeSet::new();
        for m in &self.modules {
            if !valid_id(&m.id) || !ids.insert(m.id.as_str()) {
                return Err(invalid(format!(
                    "invalid or duplicate module id {:?}",
                    m.id
                )));
            }
            let bus = self.schema >= BUS_CATALOG_SCHEMA
                && m.api == [freergb_module_api::bus::API]
                && m.devices.is_empty();
            let cooling = self.schema == COOLING_CATALOG_SCHEMA
                && m.api == [freergb_module_api::cooling::API]
                && m.devices.is_empty();
            if !valid_version(&m.version)
                || m.api.is_empty()
                || (m.devices.is_empty() && !bus && !cooling)
                || (self.schema >= BUS_CATALOG_SCHEMA
                    && m.api.contains(&freergb_module_api::bus::API)
                    && !bus)
                || (m.api.contains(&freergb_module_api::cooling::API) && !cooling)
            {
                return Err(invalid(format!(
                    "module {} has invalid version/api/devices",
                    m.id
                )));
            }
            if !m.artifacts.iter().all(valid_asset)
                || !valid_asset(&m.source)
                || m.source.target != "source"
            {
                return Err(invalid(format!("module {} has an invalid artifact", m.id)));
            }
            if m.license.trim().is_empty() || !valid_sha_like_commit(&m.origin.commit) {
                return Err(invalid(format!("module {} lacks license/origin", m.id)));
            }
        }
        for m in &self.modules {
            if let Some(missing) = m.requires.iter().find(|r| !ids.contains(r.as_str())) {
                return Err(invalid(format!(
                    "module {} requires unknown {missing}",
                    m.id
                )));
            }
        }
        Ok(())
    }

    pub fn module(&self, id: &str) -> Option<&ModuleEntry> {
        self.modules.iter().find(|m| m.id == id)
    }

    /// First module (catalog order) that claims this interface.
    pub fn match_interface(&self, i: &InterfaceInfo) -> Option<(&ModuleEntry, &DeviceMatch)> {
        self.modules
            .iter()
            .find_map(|m| m.devices.iter().find(|d| d.matches(i)).map(|d| (m, d)))
    }

    pub fn is_relevant_vendor(&self, vid: u16) -> Option<&str> {
        self.relevant_vendors
            .iter()
            .find(|v| v.vid == vid)
            .map(|v| v.name.as_str())
    }
}

fn valid_sha_like_commit(s: &str) -> bool {
    (7..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

impl ModuleEntry {
    /// Explicit opt-in installation route; never used by HID setup/discovery.
    pub fn bus_artifact(&self) -> std::result::Result<&Artifact, String> {
        if self.api != [freergb_module_api::bus::API] || !self.devices.is_empty() {
            return Err("not a bus-only API 2 module".into());
        }
        self.artifact()
            .ok_or_else(|| "no bus package for this target".into())
    }
    /// Explicit cooling plugin route; never used by HID setup/discovery.
    pub fn cooling_artifact(&self) -> std::result::Result<&Artifact, String> {
        if self.api != [freergb_module_api::cooling::API] || !self.devices.is_empty() {
            return Err("not an API 3 cooling plugin".into());
        }
        self.artifact()
            .ok_or_else(|| "no cooling plugin package for this target".into())
    }
    pub fn artifact(&self) -> Option<&Artifact> {
        self.artifacts.iter().find(|a| a.target == host_target())
    }

    pub fn compatible(&self) -> std::result::Result<&Artifact, String> {
        if !self.api.contains(&freergb_module_api::MODULE_API) {
            return Err(format!(
                "module speaks protocol {:?}, this freeRGB needs {}",
                self.api,
                freergb_module_api::MODULE_API
            ));
        }
        self.artifact()
            .ok_or_else(|| format!("no package for {}", host_target()))
    }
}
