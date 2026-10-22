//! Module package (.zip) format and safe extraction.
//!
//! A package is a flat zip: `module.json`, the executable, the license text
//! and `PROVENANCE.md`. Directories, links, absolute or parent paths, unknown
//! files and oversized entries are rejected before anything is written.
use crate::catalog::{valid_id, valid_sha, DeviceMatch, ModuleEntry};
use crate::{invalid, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;

pub const MANIFEST: &str = "module.json";
const MAX_FILES: usize = 32;
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub schema: u32,
    pub id: String,
    pub version: String,
    pub family: String,
    pub api: Vec<u32>,
    pub target: String,
    pub executable: String,
    pub license: String,
    /// Copied from the catalog so installed modules match devices offline.
    pub devices: Vec<DeviceMatch>,
    /// Every other file in the package with its SHA-256.
    pub files: BTreeMap<String, String>,
}

pub fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

impl PackageManifest {
    pub fn validate(&self) -> Result<()> {
        if self.schema != 1 || !valid_id(&self.id) || self.api.is_empty() || self.devices.is_empty()
        {
            return Err(invalid("invalid package manifest"));
        }
        if !self.files.contains_key(&self.executable) {
            return Err(invalid("package manifest does not list its executable"));
        }
        if !self.files.contains_key("PROVENANCE.md")
            || !self.files.keys().any(|f| f == "COPYING" || f == "LICENSE")
        {
            return Err(invalid("package lacks license text or PROVENANCE.md"));
        }
        if self.files.len() + 1 > MAX_FILES
            || self
                .files
                .iter()
                .any(|(name, sha)| !safe_name(name) || name == MANIFEST || !valid_sha(sha))
        {
            return Err(invalid("package manifest lists invalid files"));
        }
        Ok(())
    }

    /// The package must be exactly the module the signed catalog describes.
    pub fn check_against(&self, entry: &ModuleEntry, target: &str) -> Result<()> {
        if self.id != entry.id
            || self.version != entry.version
            || self.family != entry.family
            || self.target != target
            || self.api != entry.api
            || self.devices != entry.devices
            || self.license != entry.license
        {
            return Err(invalid("package does not match its catalog entry"));
        }
        Ok(())
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}

/// Extracts `package` into the empty directory `into` and returns the
/// manifest plus the SHA-256 of `module.json` itself.
pub fn extract(package: &Path, into: &Path) -> Result<(PackageManifest, String)> {
    let file = std::fs::File::open(package)?;
    let mut archive = zip::ZipArchive::new(file).map_err(invalid)?;
    if archive.len() > MAX_FILES {
        return Err(invalid("package has too many entries"));
    }
    // First pass: names and declared sizes only.
    let mut total = 0u64;
    let mut names = std::collections::BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index).map_err(invalid)?;
        let name = entry.name().to_owned();
        if !entry.is_file() || entry.is_symlink() || !safe_name(&name) || !names.insert(name) {
            return Err(invalid("package contains an unsafe or duplicate entry"));
        }
        total = total.saturating_add(entry.size());
    }
    if total > MAX_UNPACKED {
        return Err(invalid("package unpacks to more than the limit"));
    }
    let manifest_bytes = {
        let entry = archive
            .by_name(MANIFEST)
            .map_err(|_| invalid("package has no module.json"))?;
        let mut bytes = Vec::new();
        entry.take(256 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 256 * 1024 {
            return Err(invalid("module.json too large"));
        }
        bytes
    };
    let manifest: PackageManifest = serde_json::from_slice(&manifest_bytes).map_err(invalid)?;
    manifest.validate()?;
    let expected: std::collections::BTreeSet<_> = manifest
        .files
        .keys()
        .cloned()
        .chain([MANIFEST.to_owned()])
        .collect();
    if expected != names {
        return Err(invalid("package files differ from module.json"));
    }
    std::fs::write(into.join(MANIFEST), &manifest_bytes)?;
    for (name, sha) in &manifest.files {
        let entry = archive.by_name(name).map_err(invalid)?;
        let declared = entry.size();
        let mut reader = entry.take(declared + 1);
        let path = into.join(name);
        let mut out = std::fs::File::create(&path)?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut written = 0u64;
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            written += n as u64;
            if written > declared {
                return Err(invalid("entry exceeds its declared size"));
            }
            hash.update(&buffer[..n]);
            out.write_all(&buffer[..n])?;
        }
        out.sync_all()?;
        if &hex::encode(hash.finalize()) != sha {
            return Err(invalid(format!("{name} does not match module.json")));
        }
    }
    Ok((manifest, hex::encode(Sha256::digest(&manifest_bytes))))
}

/// Builds a package (release tool and tests). `files` are (name, bytes).
pub fn build(mut manifest: PackageManifest, files: &[(String, Vec<u8>)], out: &Path) -> Result<()> {
    manifest.files = files
        .iter()
        .map(|(n, b)| (n.clone(), hex::encode(Sha256::digest(b))))
        .collect();
    manifest.validate()?;
    let file = std::fs::File::create(out)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    zip.start_file(MANIFEST, options).map_err(invalid)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest).map_err(invalid)?)?;
    for (name, bytes) in files {
        zip.start_file(name.as_str(), options).map_err(invalid)?;
        zip.write_all(bytes)?;
    }
    zip.finish().map_err(invalid)?;
    Ok(())
}
