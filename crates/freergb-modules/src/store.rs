//! Installed modules: `<root>/<id>/<version>/` plus `state.json`.
//!
//! Installation extracts into a staging directory next to the target, checks
//! every file and then renames it into place, so a crash leaves either the old
//! or the new version, never a half-written one. `state.json` is replaced
//! atomically. The previous and the last version that initialized a device
//! are kept for rollback; nothing updates itself.
use crate::catalog::{host_target, ModuleEntry};
use crate::package::{self, PackageManifest, MANIFEST};
use crate::{invalid, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Installed {
    pub active: String,
    #[serde(default)]
    pub previous: Option<String>,
    #[serde(default)]
    pub last_good: Option<String>,
    /// SHA-256 of each version's verified module.json.
    pub manifests: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StoreState {
    pub modules: BTreeMap<String, Installed>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledSummary {
    pub id: String,
    pub version: String,
    pub previous: Option<String>,
    pub last_good: Option<String>,
    pub family: String,
    pub license: String,
}

pub struct Store {
    root: PathBuf,
}

/// A verified module ready to start.
#[derive(Debug, Clone)]
pub struct Verified {
    pub manifest: PackageManifest,
    pub executable: PathBuf,
    pub directory: PathBuf,
}

/// API 2 package reverified by Store. Not constructible by a module/child.
pub struct VerifiedBus {
    pub(crate) verified: Verified,
}

impl VerifiedBus {
    pub fn policy(&self) -> &freergb_module_api::bus::Policy {
        self.verified
            .manifest
            .bus
            .as_ref()
            .expect("validated bus manifest")
    }
}

/// API 4 transport package reverified by Store.
pub struct VerifiedTransport {
    pub(crate) verified: Verified,
}
impl VerifiedTransport {
    pub fn policy(&self) -> &freergb_module_api::transport::Policy {
        self.verified
            .manifest
            .transport
            .as_ref()
            .expect("validated transport policy")
    }
    /// Explicit fixture/development route; no production setup integration.
    pub fn unpackaged(verified: Verified) -> Result<Self> {
        verified.manifest.validate()?;
        if verified.manifest.transport.is_none() {
            return Err(invalid("not a transport manifest"));
        }
        Ok(Self { verified })
    }
}

/// API 3 cooling plugin reverified by Store. Not constructible by a plugin.
pub struct VerifiedCooling {
    pub(crate) verified: Verified,
}

impl VerifiedCooling {
    pub fn policy(&self) -> &freergb_module_api::cooling::Policy {
        self.verified
            .manifest
            .cooling
            .as_ref()
            .expect("validated cooling manifest")
    }

    pub fn manifest(&self) -> &PackageManifest {
        &self.verified.manifest
    }

    /// Test and fixture route: a launch description without a store. The
    /// policy is still validated.
    pub fn unpackaged(verified: Verified) -> Result<Self> {
        verified.manifest.validate()?;
        if verified.manifest.cooling.is_none() {
            return Err(invalid("not a cooling plugin manifest"));
        }
        Ok(Self { verified })
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Hid,
    Bus,
    Cooling,
    Transport,
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root)?;
        let store = Self { root };
        store.cleanup();
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn downloads(&self) -> PathBuf {
        self.root.join(".downloads")
    }

    /// Removes staging directories and partial downloads of earlier runs.
    pub fn cleanup(&self) {
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(".staging-") {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(self.downloads()) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "part") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }

    pub fn state(&self) -> StoreState {
        std::fs::read(self.root.join("state.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save(&self, state: &StoreState) -> Result<()> {
        write_atomic(
            &self.root.join("state.json"),
            &serde_json::to_vec_pretty(state).map_err(invalid)?,
        )
    }

    fn version_dir(&self, id: &str, version: &str) -> PathBuf {
        self.root.join(id).join(version)
    }

    /// Installs a downloaded package whose size and SHA-256 were already
    /// checked against the signed catalog entry.
    /// Checks a package file against its signed catalog entry.
    pub fn check_package(entry: &ModuleEntry, package_path: &Path) -> Result<()> {
        Self::check_package_for(entry, package_path, Kind::Hid)
    }

    fn check_package_for(entry: &ModuleEntry, package_path: &Path, kind: Kind) -> Result<()> {
        let artifact = match kind {
            Kind::Hid => entry.compatible(),
            Kind::Bus => entry.bus_artifact(),
            Kind::Cooling => entry.cooling_artifact(),
            Kind::Transport => entry.transport_artifact(),
        }
        .map_err(invalid)?;
        if std::fs::metadata(package_path)?.len() != artifact.size
            || package::sha256_file(package_path)? != artifact.sha256
        {
            return Err(invalid("package does not match the signed catalog"));
        }
        Ok(())
    }

    pub fn install(&self, entry: &ModuleEntry, package_path: &Path) -> Result<()> {
        self.install_for(entry, package_path, Kind::Hid)
    }

    /// Caller supplies an entry from a signature-verified schema-2 catalog.
    /// Installation grants no device access and does not run the executable.
    pub fn install_bus(&self, entry: &ModuleEntry, package_path: &Path) -> Result<()> {
        self.install_for(entry, package_path, Kind::Bus)
    }

    /// Explicit simulation route from a signature-verified schema-4 catalog.
    /// Installation neither grants consent nor starts a module.
    pub fn install_transport(&self, entry: &ModuleEntry, package_path: &Path) -> Result<()> {
        self.install_for(entry, package_path, Kind::Transport)
    }

    /// Caller supplies an entry from a signature-verified schema-3 catalog.
    /// Installing does not enable or start the plugin.
    pub fn install_cooling(&self, entry: &ModuleEntry, package_path: &Path) -> Result<()> {
        self.install_for(entry, package_path, Kind::Cooling)
    }

    fn install_for(&self, entry: &ModuleEntry, package_path: &Path, kind: Kind) -> Result<()> {
        // Checked again here: the file could have changed since download.
        Self::check_package_for(entry, package_path, kind)?;
        let staging = self.root.join(format!(
            ".staging-{}-{}-{}",
            entry.id,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&staging)?;
        let result = (|| {
            let (manifest, manifest_sha) = package::extract(package_path, &staging)?;
            manifest.check_against(entry, host_target())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    staging.join(&manifest.executable),
                    std::fs::Permissions::from_mode(0o755),
                )?;
            }
            let target = self.version_dir(&entry.id, &entry.version);
            if target.exists() {
                // Same version already present: keep it only if intact.
                if self.verify_dir(&target, &manifest_sha).is_ok() {
                    std::fs::remove_dir_all(&staging)?;
                } else {
                    let broken = self.root.join(format!(".staging-broken-{}", entry.id));
                    let _ = std::fs::remove_dir_all(&broken);
                    std::fs::rename(&target, &broken)?;
                    std::fs::rename(&staging, &target)?;
                }
            } else {
                std::fs::create_dir_all(target.parent().unwrap())?;
                std::fs::rename(&staging, &target)?;
            }
            let mut state = self.state();
            let installed = state.modules.entry(entry.id.clone()).or_default();
            if installed.active != entry.version {
                if !installed.active.is_empty() {
                    installed.previous = Some(installed.active.clone());
                }
                installed.active = entry.version.clone();
            }
            installed
                .manifests
                .insert(entry.version.clone(), manifest_sha);
            self.save(&state)?;
            self.collect(&entry.id);
            Ok(())
        })();
        if staging.exists() {
            let _ = std::fs::remove_dir_all(&staging);
        }
        result
    }

    /// Deletes versions that are neither active, previous nor last good.
    fn collect(&self, id: &str) {
        let state = self.state();
        let Some(installed) = state.modules.get(id) else {
            return;
        };
        let keep: Vec<&str> = [
            Some(installed.active.as_str()),
            installed.previous.as_deref(),
            installed.last_good.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();
        if let Ok(entries) = std::fs::read_dir(self.root.join(id)) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !keep.contains(&name.as_str()) {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
    }

    fn verify_dir(&self, dir: &Path, manifest_sha: &str) -> Result<PackageManifest> {
        let bytes = std::fs::read(dir.join(MANIFEST))?;
        if hex::encode(<sha2::Sha256 as sha2::Digest>::digest(&bytes)) != manifest_sha {
            return Err(invalid("installed module.json was modified"));
        }
        let manifest: PackageManifest = serde_json::from_slice(&bytes).map_err(invalid)?;
        manifest.validate()?;
        for (name, sha) in &manifest.files {
            if &package::sha256_file(&dir.join(name))? != sha {
                return Err(invalid(format!("installed file {name} was modified")));
            }
        }
        Ok(manifest)
    }

    /// Re-verifies the active version of `id` before it is executed.
    pub fn verified(&self, id: &str) -> Result<Verified> {
        self.verified_for(id, Kind::Hid)
    }

    pub fn verified_bus(&self, id: &str) -> Result<VerifiedBus> {
        Ok(VerifiedBus {
            verified: self.verified_for(id, Kind::Bus)?,
        })
    }

    pub fn verified_transport(&self, id: &str) -> Result<VerifiedTransport> {
        Ok(VerifiedTransport {
            verified: self.verified_for(id, Kind::Transport)?,
        })
    }

    pub fn verified_cooling(&self, id: &str) -> Result<VerifiedCooling> {
        Ok(VerifiedCooling {
            verified: self.verified_for(id, Kind::Cooling)?,
        })
    }

    fn verified_for(&self, id: &str, kind: Kind) -> Result<Verified> {
        let state = self.state();
        let installed = state
            .modules
            .get(id)
            .ok_or_else(|| invalid(format!("module {id} is not installed")))?;
        let sha = installed
            .manifests
            .get(&installed.active)
            .ok_or_else(|| invalid("installed module has no recorded manifest"))?;
        let directory = self.version_dir(id, &installed.active);
        let manifest = self.verify_dir(&directory, sha)?;
        let compatible = match kind {
            Kind::Transport => {
                manifest.schema == 4
                    && manifest.api == [freergb_module_api::transport::API]
                    && manifest.transport.is_some()
            }
            Kind::Bus => {
                manifest.schema == 2
                    && manifest.api == [freergb_module_api::bus::API]
                    && manifest.bus.is_some()
            }
            Kind::Hid => {
                manifest.schema == 1 && manifest.api.contains(&freergb_module_api::MODULE_API)
            }
            Kind::Cooling => {
                manifest.schema == 3
                    && manifest.api == [freergb_module_api::cooling::API]
                    && manifest.cooling.is_some()
            }
        };
        if manifest.target != host_target() || !compatible {
            return Err(invalid(
                "installed module is not compatible with this freeRGB",
            ));
        }
        Ok(Verified {
            executable: directory.join(&manifest.executable),
            directory,
            manifest,
        })
    }

    /// Active manifests whose module.json matches the recorded hash. Cheap
    /// (no executable hashing): used for matching and status only; anything
    /// that executes goes through [`Store::verified`].
    pub fn manifests(&self) -> Vec<PackageManifest> {
        self.state()
            .modules
            .iter()
            .filter_map(|(id, i)| {
                let bytes = std::fs::read(self.version_dir(id, &i.active).join(MANIFEST)).ok()?;
                let sha = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(&bytes));
                (i.manifests.get(&i.active) == Some(&sha)).then_some(())?;
                serde_json::from_slice(&bytes).ok()
            })
            .collect()
    }

    /// All verified installed modules (full file check).
    pub fn all_verified(&self) -> Vec<Verified> {
        self.state()
            .modules
            .keys()
            .filter_map(|id| match self.verified(id) {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::warn!(module = %id, error = %e, "installed module failed verification");
                    None
                }
            })
            .collect()
    }

    pub fn mark_good(&self, id: &str) -> Result<()> {
        let mut state = self.state();
        if let Some(installed) = state.modules.get_mut(id) {
            if installed.last_good.as_deref() != Some(installed.active.as_str()) {
                installed.last_good = Some(installed.active.clone());
                self.save(&state)?;
                self.collect(id);
            }
        }
        Ok(())
    }

    /// Returns to the last version that initialized a device (or the previous one).
    pub fn rollback(&self, id: &str) -> Result<String> {
        let mut state = self.state();
        let installed = state
            .modules
            .get_mut(id)
            .ok_or_else(|| invalid("module is not installed"))?;
        let target = installed
            .last_good
            .clone()
            .filter(|v| v != &installed.active)
            .or_else(|| installed.previous.clone())
            .ok_or_else(|| invalid("no earlier version to return to"))?;
        if !installed.manifests.contains_key(&target) || !self.version_dir(id, &target).exists() {
            return Err(invalid("earlier version is no longer on disk"));
        }
        installed.previous = Some(std::mem::replace(&mut installed.active, target.clone()));
        self.save(&state)?;
        Ok(target)
    }

    pub fn summaries(&self) -> Vec<InstalledSummary> {
        let state = self.state();
        state
            .modules
            .iter()
            .map(|(id, i)| {
                let manifest = self.manifests().into_iter().find(|m| &m.id == id);
                InstalledSummary {
                    id: id.clone(),
                    version: i.active.clone(),
                    previous: i.previous.clone(),
                    last_good: i.last_good.clone(),
                    family: manifest
                        .as_ref()
                        .map(|m| m.family.clone())
                        .unwrap_or_default(),
                    license: manifest.map(|m| m.license).unwrap_or_default(),
                }
            })
            .collect()
    }
}
