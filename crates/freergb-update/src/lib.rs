//! Downloads immutable release installers, never a Git working tree.
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const API: &str = "https://api.github.com/repos/leotrax3d/FreeRGB";
// Provision a release public key here via a reviewed source change. Never load keys
// from the network, environment or update metadata. No production key exists yet.
const PINNED_UPDATE_KEY: Option<[u8; 32]> = None;

#[cfg(not(test))]
fn pinned_key() -> Result<[u8; 32], String> {
    PINNED_UPDATE_KEY.ok_or_else(|| "installer signing key is not provisioned".into())
}
#[cfg(test)]
fn pinned_key() -> Result<[u8; 32], String> {
    Ok(tests::signing_key().verifying_key().to_bytes())
}

fn signature_message(update: &Update) -> Result<String, String> {
    let version = semver::Version::parse(&update.version).map_err(|e| e.to_string())?;
    if version.to_string() != update.version
        || update.sha256.len() != 64
        || !update.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || update.size == 0
        || update.size > MAX_INSTALLER
    {
        return Err("invalid signed installer metadata".into());
    }
    Ok(format!(
        "freeRGB-installer-v1\nwindows-x86_64\n{}\n{}\n{}\n",
        update.version,
        update.size,
        update.sha256.to_ascii_lowercase()
    ))
}

fn verify_signature_with_key(update: &Update, key: &[u8; 32]) -> Result<(), String> {
    let message = signature_message(update)?;
    let signature = ed25519_dalek::Signature::from_slice(&update.signature)
        .map_err(|_| "missing or malformed installer signature")?;
    let key = ed25519_dalek::VerifyingKey::from_bytes(key).map_err(|e| e.to_string())?;
    key.verify_strict(message.as_bytes(), &signature)
        .map_err(|_| "installer signature verification failed".into())
}
fn verify_signature(update: &Update) -> Result<(), String> {
    verify_signature_with_key(update, &pinned_key()?)
}

const MAX_INSTALLER: u64 = 256 * 1024 * 1024;

fn temporary_installer(destination: &Path) -> Result<(PathBuf, std::fs::File), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..128 {
        let path = destination.with_extension(format!(
            "{}-{}.part",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // Never truncate another download or follow a pre-existing symlink.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("cannot reserve an installer staging file".into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update {
    pub version: String,
    pub asset_id: u64,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub signature: Vec<u8>,
    #[serde(default)]
    pub signature_asset_id: u64,
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
        .https_only(true)
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
    select_release(release, current)
}

fn release_version(release: &Release) -> Result<semver::Version, semver::Error> {
    semver::Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )
}

fn eligible(release: &Release, version: &semver::Version, current: &semver::Version) -> bool {
    !release.draft
        && version > current
        && !(release.prerelease && version.pre.is_empty())
        && (version.pre.is_empty()
            || (!current.pre.is_empty()
                && version.pre.as_str().split('.').next()
                    == current.pre.as_str().split('.').next()))
}

fn select_release(release: Release, current: &str) -> Result<Option<Update>, String> {
    let version = release_version(&release).map_err(|e| e.to_string())?;
    let current = semver::Version::parse(current).map_err(|e| e.to_string())?;
    if !eligible(&release, &version, &current) {
        return Ok(None);
    }
    let name = format!("freeRGB-{version}-windows-x86_64-setup.exe");
    let signature_name = format!("{name}.sig");
    let signature_asset_id = release
        .assets
        .iter()
        .find(|a| a.name == signature_name && a.size == 64)
        .map_or(0, |a| a.id);
    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name == name)
        .ok_or("release has no Windows x64 installer")?;
    if asset.id == 0 || asset.size == 0 || asset.size > MAX_INSTALLER {
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
        signature: Vec::new(),
        signature_asset_id,
    }))
}

fn parse_release_list(text: &str, current: &str) -> Result<Option<Update>, String> {
    let releases: Vec<Release> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let current_version = semver::Version::parse(current).map_err(|e| e.to_string())?;
    let newest = releases
        .into_iter()
        .filter_map(|release| {
            // Rolling module/effect releases do not use application SemVer tags.
            let version = release_version(&release).ok()?;
            eligible(&release, &version, &current_version).then_some((version, release))
        })
        .max_by(|a, b| a.0.cmp(&b.0));
    newest.map_or(Ok(None), |(_, release)| select_release(release, current))
}
pub fn check() -> Result<Option<Update>, String> {
    if !cfg!(windows) {
        return Err("automatic installer updates currently support Windows only".into());
    }
    let client = client()?;
    let current = env!("CARGO_PKG_VERSION");
    let beta = !semver::Version::parse(current)
        .map_err(|e| e.to_string())?
        .pre
        .is_empty();
    let endpoint = if beta {
        "releases?per_page=100"
    } else {
        "releases/latest"
    };
    let response = request(
        &client,
        format!("{API}/{endpoint}"),
        "application/vnd.github+json",
    )
    .timeout(Duration::from_secs(15))
    .send()
    .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("No accessible release: no application release is published, or this account lacks read access.".into());
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
    let mut update = if beta {
        parse_release_list(&text, current)?
    } else {
        parse_release(&text, current)?
    };
    if let Some(update) = &mut update {
        fetch_signature(update)?;
        verify_signature(update)?;
    }
    Ok(update)
}

fn fetch_signature(update: &mut Update) -> Result<(), String> {
    let client = client()?;
    if update.signature.is_empty() {
        if update.signature_asset_id == 0 {
            return Err("release has no detached installer signature".into());
        }
        let response = request(
            &client,
            format!("{API}/releases/assets/{}", update.signature_asset_id),
            "application/octet-stream",
        )
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
        response
            .take(65)
            .read_to_end(&mut update.signature)
            .map_err(|e| e.to_string())?;
    }
    verify_signature(update)?;
    Ok(())
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
    verify_signature(update)?;
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let destination = directory.join(format!("freeRGB-{}-setup.exe", update.version));
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
    stage_verified(response, update, &destination)
}

fn stage_verified(
    reader: impl Read,
    update: &Update,
    destination: &Path,
) -> Result<PathBuf, String> {
    verify_signature(update)?;
    stage_transfer(destination, |file| copy_verified(reader, file, update))
}

fn stage_transfer(
    destination: &Path,
    transfer: impl FnOnce(&mut std::fs::File) -> Result<(), String>,
) -> Result<PathBuf, String> {
    let (temporary, file) = temporary_installer(destination)?;
    let result = (|| {
        let mut file = file;
        transfer(&mut file)?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&temporary, destination).map_err(|e| e.to_string())?;
        Ok(destination.to_path_buf())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

// The network path and deterministic I/O fault tests share this verification.
fn copy_verified(reader: impl Read, file: &mut impl Write, update: &Update) -> Result<(), String> {
    let mut reader = reader.take(update.size + 1);
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
    Ok(())
}

pub fn verify(path: &Path, update: &Update) -> Result<(), String> {
    verified_file(path, update).map(|_| ())
}

fn verified_file(path: &Path, update: &Update) -> Result<std::fs::File, String> {
    verify_signature(update)?;
    if update.size == 0
        || update.size > MAX_INSTALLER
        || update.sha256.len() != 64
        || !update.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("invalid installer verification metadata".into());
    }
    #[cfg(windows)]
    let mut file = {
        use std::os::windows::fs::OpenOptionsExt;
        // Retain this handle across ShellExecute: deny writes/deletion between
        // verification and the elevation request (FILE_SHARE_READ only).
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(path)
            .map_err(|e| e.to_string())?
    };
    #[cfg(not(windows))]
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() != update.size {
        return Err("installer size changed".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut reader = (&mut file).take(update.size + 1);
    let mut read = 0u64;
    loop {
        let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        read += n as u64;
    }
    if read != update.size || format!("{:x}", hash.finalize()) != update.sha256.to_ascii_lowercase()
    {
        return Err("installer checksum changed".into());
    }
    Ok(file)
}

/// Start only a freshly verified installer, using the Windows shell's explicit
/// elevation verb. CreateProcess/spawn alone fails with ERROR_ELEVATION_REQUIRED
/// for Inno Setup's administrator manifest and cannot display the UAC prompt.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn install_verified(path: &Path, update: &Update) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    // Shell APIs expect DOS/UNC paths, not Rust's usual verbatim \\?\ prefix.
    let path = dunce::canonicalize(path).map_err(|e| e.to_string())?;
    let _verified = verified_file(&path, update)?;
    let executable: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    if executable[..executable.len() - 1].contains(&0) {
        return Err("invalid installer path".into());
    }
    let verb: Vec<u16> = "runas".encode_utf16().chain([0]).collect();
    // SAFETY: both strings are NUL-terminated, live for the synchronous call,
    // and contain no interior NUL. Optional pointers and parent window are null.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            executable.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        return Err(format!(
            "Installer elevation failed or was cancelled: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn install_verified(_: &Path, _: &Update) -> Result<(), String> {
    Err("automatic installer updates currently support Windows only".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;
    pub(super) fn signing_key() -> ed25519_dalek::SigningKey {
        static KEY: std::sync::OnceLock<ed25519_dalek::SigningKey> = std::sync::OnceLock::new();
        KEY.get_or_init(|| {
            // Ephemeral generated test key only; never a production trust anchor.
            let seed = format!("{:?}-{}", std::time::SystemTime::now(), std::process::id());
            ed25519_dalek::SigningKey::from_bytes(&Sha256::digest(seed.as_bytes()).into())
        })
        .clone()
    }
    pub(super) fn signed(mut update: Update) -> Update {
        update.signature = signing_key()
            .sign(signature_message(&update).unwrap().as_bytes())
            .to_bytes()
            .to_vec();
        update
    }
    #[test]
    fn signatures_bind_code_and_version_and_reject_online_forgery() {
        assert!(PINNED_UPDATE_KEY.is_none());
        let mut update = super::recovery_tests::metadata(b"fixture");
        verify_signature(&update).unwrap();
        update.version = "9.0.0".into();
        assert!(verify_signature(&update).is_err());
        update = super::recovery_tests::metadata(b"attacker code");
        update.signature.clear();
        assert!(verify_signature(&update).is_err());
        update = super::recovery_tests::metadata(b"fixture");
        let other = ed25519_dalek::SigningKey::from_bytes(
            &Sha256::digest(b"different generated test key").into(),
        );
        assert!(verify_signature_with_key(&update, &other.verifying_key().to_bytes()).is_err());
        update.signature[0] ^= 1;
        assert!(verify_signature(&update).is_err());
    }

    fn release(version: &str) -> serde_json::Value {
        serde_json::json!({"tag_name":format!("v{version}"),"draft":false,"prerelease":version.contains('-'),
            "assets":[{"id":42,"name":format!("freeRGB-{version}-windows-x86_64-setup.exe"),"size":100,
                "digest":format!("sha256:{}", "a".repeat(64))}]})
    }

    #[test]
    fn forged_hash_and_size_cannot_authorize_payload_or_touch_destination() {
        let root =
            std::env::temp_dir().join(format!("freergb-signature-forgery-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("installer.exe");
        let bytes = b"attacker payload";
        let mut forged = super::recovery_tests::metadata(b"authorized fixture");
        forged.sha256 = format!("{:x}", Sha256::digest(bytes));
        forged.size = bytes.len() as u64;
        std::fs::write(&path, bytes).unwrap();
        assert!(verify(&path, &forged).is_err());
        assert!(stage_verified(bytes.as_slice(), &forged, &path).is_err());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        let mut json = serde_json::to_value(&forged).unwrap();
        json.as_object_mut().unwrap().remove("signature");
        let unsigned: Update = serde_json::from_value(json).unwrap();
        assert!(verify(&path, &unsigned).is_err());
        assert!(download(&unsigned, &root.join("never-created")).is_err());
        assert!(!root.join("never-created").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn beta_updates_stay_in_channel_allow_stable_promotion_and_use_semver_order() {
        let mut draft = release("0.1.0-beta.20");
        draft["draft"] = true.into();
        let mut rolling = release("0.1.0");
        rolling["tag_name"] = "modules".into();
        let list = serde_json::json!([
            release("0.1.0-beta.3"),
            release("0.1.0-rc.1"),
            draft,
            rolling,
            release("0.1.0-beta.10")
        ]);
        assert_eq!(
            parse_release_list(&list.to_string(), "0.1.0-beta.2")
                .unwrap()
                .unwrap()
                .version,
            "0.1.0-beta.10"
        );
        assert!(parse_release_list(&list.to_string(), "0.1.0")
            .unwrap()
            .is_none());
        let promoted = serde_json::json!([release("0.1.0-beta.99"), release("0.1.0")]);
        assert_eq!(
            parse_release_list(&promoted.to_string(), "0.1.0-beta.2")
                .unwrap()
                .unwrap()
                .version,
            "0.1.0"
        );
        assert!(
            parse_release(&release("0.1.0-alpha.9").to_string(), "0.1.0-beta.2")
                .unwrap()
                .is_none()
        );
        assert!(
            parse_release(&release("0.1.0-beta.1").to_string(), "0.1.0-beta.2")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn newer_unverifiable_release_is_not_silently_replaced_with_an_older_candidate() {
        let mut broken = release("0.1.0-beta.4");
        broken["assets"][0]["digest"] = serde_json::Value::Null;
        assert!(parse_release_list(
            &serde_json::json!([release("0.1.0-beta.3"), broken]).to_string(),
            "0.1.0-beta.2"
        )
        .is_err());
        let mut invalid_tag = release("0.2.0");
        invalid_tag["tag_name"] = "vv0.2.0".into();
        assert!(parse_release(&invalid_tag.to_string(), "0.1.0").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn verified_handle_prevents_windows_write_or_delete_until_launch_handoff() {
        let directory =
            std::env::temp_dir().join(format!("freergb update ü {}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("installer.exe");
        let bytes = b"fixture only, never executed";
        std::fs::write(&path, bytes).unwrap();
        assert!(!dunce::canonicalize(&path)
            .unwrap()
            .as_os_str()
            .to_string_lossy()
            .starts_with(r"\\?\"));
        let update = Update {
            version: "0.2.0".into(),
            asset_id: 1,
            size: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            signature: Vec::new(),
            signature_asset_id: 0,
        };
        let update = signed(update);
        let file = verified_file(&path, &update).unwrap();
        assert!(std::fs::write(&path, b"replaced").is_err());
        assert!(std::fs::remove_file(&path).is_err());
        drop(file);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn simultaneous_downloads_have_exclusive_staging_files() {
        let root = std::env::temp_dir().join(format!("freergb-staging-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let destination = root.join("installer.exe");
        let old_partial = destination.with_extension("part");
        std::fs::write(&old_partial, b"another download").unwrap();
        let (first_path, mut first) = temporary_installer(&destination).unwrap();
        let (second_path, mut second) = temporary_installer(&destination).unwrap();
        assert_ne!(first_path, second_path);
        first.write_all(b"first").unwrap();
        second.write_all(b"second").unwrap();
        drop((first, second));
        assert_eq!(std::fs::read(first_path).unwrap(), b"first");
        assert_eq!(std::fs::read(second_path).unwrap(), b"second");
        assert_eq!(std::fs::read(old_partial).unwrap(), b"another download");
        std::fs::remove_dir_all(root).unwrap();
    }
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
            signature: Vec::new(),
            signature_asset_id: 0,
        };
        let update = signed(update);
        verify(&path, &update).unwrap();
        std::fs::write(&path, vec![b'x'; bytes.len()]).unwrap();
        assert!(verify(&path, &update).is_err());
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod recovery_tests;
