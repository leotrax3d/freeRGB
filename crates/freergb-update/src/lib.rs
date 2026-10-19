//! Downloads immutable release installers, never a Git working tree.
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const API: &str = "https://api.github.com/repos/leotrax3d/FreeRGB";
const MAX_INSTALLER: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update {
    pub version: String,
    pub asset_id: u64,
    pub sha256: String,
    pub size: u64,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    id: u64,
    name: String,
    size: u64,
    digest: Option<String>,
}

fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("freeRGB/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}
fn request(client: &Client, url: String, accept: &str) -> reqwest::blocking::RequestBuilder {
    let request = client
        .get(url)
        .header("Accept", accept)
        .header("X-GitHub-Api-Version", "2022-11-28");
    match std::env::var("FREERGB_GITHUB_TOKEN") {
        Ok(token) if !token.is_empty() => request.bearer_auth(token),
        _ => request,
    }
}
fn parse_release(text: &str, current: &str) -> Result<Option<Update>, String> {
    let release: Release = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let version = semver::Version::parse(release.tag_name.trim_start_matches('v'))
        .map_err(|e| e.to_string())?;
    let current = semver::Version::parse(current).map_err(|e| e.to_string())?;
    if release.draft || release.prerelease || !version.pre.is_empty() || version <= current {
        return Ok(None);
    }
    let name = format!("freeRGB-{version}-windows-x86_64-setup.exe");
    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name == name)
        .ok_or("release has no Windows x64 installer")?;
    if asset.size == 0 || asset.size > MAX_INSTALLER {
        return Err("installer size is invalid".into());
    }
    let digest = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .ok_or("release asset has no GitHub SHA-256 digest")?;
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid installer digest".into());
    }
    Ok(Some(Update {
        version: version.to_string(),
        asset_id: asset.id,
        sha256: digest.to_ascii_lowercase(),
        size: asset.size,
    }))
}
pub fn check() -> Result<Option<Update>, String> {
    if !cfg!(windows) {
        return Err("automatic installer updates currently support Windows only".into());
    }
    let client = client()?;
    let response = request(
        &client,
        format!("{API}/releases/latest"),
        "application/vnd.github+json",
    )
    .timeout(Duration::from_secs(15))
    .send()
    .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("No accessible release. This repository is private; set FREERGB_GITHUB_TOKEN for read access, or publish a release first.".into());
    }
    let response = response.error_for_status().map_err(|e| e.to_string())?;
    let mut text = String::new();
    response
        .take(1024 * 1024 + 1)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() > 1024 * 1024 {
        return Err("release metadata exceeds limit".into());
    }
    parse_release(&text, env!("CARGO_PKG_VERSION"))
}

pub fn download(update: &Update, directory: &Path) -> Result<PathBuf, String> {
    // Validate caller-controlled serialized metadata as well as release metadata.
    semver::Version::parse(&update.version).map_err(|e| e.to_string())?;
    if update.sha256.len() != 64
        || !update.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || update.size == 0
        || update.size > MAX_INSTALLER
    {
        return Err("invalid update metadata".into());
    }
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let destination = directory.join(format!("freeRGB-{}-setup.exe", update.version));
    let temporary = destination.with_extension("part");
    let result = (|| {
        let client = client()?;
        let response = request(
            &client,
            format!("{API}/releases/assets/{}", update.asset_id),
            "application/octet-stream",
        )
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
        let mut reader = response.take(update.size + 1);
        let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
        let mut digest = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            size += count as u64;
            if size > update.size {
                return Err("download exceeds expected size".into());
            }
            file.write_all(&buffer[..count])
                .map_err(|e| e.to_string())?;
            digest.update(&buffer[..count]);
        }
        if size != update.size
            || format!("{:x}", digest.finalize()) != update.sha256.to_ascii_lowercase()
        {
            return Err("installer checksum/size mismatch".into());
        }
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&temporary, &destination).map_err(|e| e.to_string())?;
        Ok(destination)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

pub fn verify(path: &Path, update: &Update) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() != update.size {
        return Err("installer size changed".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if format!("{:x}", hash.finalize()) != update.sha256.to_ascii_lowercase() {
        return Err("installer checksum changed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_selection_blocks_downgrades_and_unverifiable_assets() {
        let release = serde_json::json!({"tag_name":"v0.2.0","draft":false,"prerelease":false,"assets":[{"id":42,"name":"freeRGB-0.2.0-windows-x86_64-setup.exe","size":100,"digest":format!("sha256:{}", "a".repeat(64))}]});
        assert_eq!(
            parse_release(&release.to_string(), "0.1.0")
                .unwrap()
                .unwrap()
                .asset_id,
            42
        );
        assert!(parse_release(&release.to_string(), "0.3.0")
            .unwrap()
            .is_none());
        let mut bad = release.clone();
        bad["assets"][0]["digest"] = serde_json::Value::Null;
        assert!(parse_release(&bad.to_string(), "0.1.0").is_err());
        bad = release;
        bad["prerelease"] = serde_json::json!(true);
        assert!(parse_release(&bad.to_string(), "0.1.0").unwrap().is_none());
    }

    #[test]
    fn staged_installer_is_reverified_before_execution() {
        let path =
            std::env::temp_dir().join(format!("freergb-update-test-{}.exe", std::process::id()));
        let bytes = b"fixture only, never executed";
        std::fs::write(&path, bytes).unwrap();
        let update = Update {
            version: "0.2.0".into(),
            asset_id: 1,
            size: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        };
        verify(&path, &update).unwrap();
        std::fs::write(&path, vec![b'x'; bytes.len()]).unwrap();
        assert!(verify(&path, &update).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
