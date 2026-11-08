//! Official-only catalog, bounded GitHub downloads and immutable local packages.
//! Network jobs never hold the daemon's render mutex. TLS authenticates the
//! official repository; size/SHA-256 bind each module to its catalog entry.
use crate::runtime::{Fault, MAX_WASM};
use freergb_modules::{
    catalog::Artifact,
    source::{Fetcher, Progress, Source},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

pub const OFFICIAL: &str = "github:leotrax3d/freeRGB@effects";
const COMMUNITY_OFF: &str = "Online community features are turned off. Enable them in the Marketplace privacy settings to rate effects or send reports.";
#[cfg(test)]
mod cancellation_tests {
    use super::*;
    #[test]
    fn cancellation_preserves_installed_index_and_retry_clears_flag() {
        let root = std::env::temp_dir().join(format!(
            "freergb-cancel-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let manager = Manager::new(root.clone()).unwrap();
        let entry = manager.view().catalog[0].clone();
        manager
            .begin("install", Some(entry.id.clone()), entry.artifact.size)
            .unwrap();
        manager.cancel_install().unwrap();
        assert!(manager.install_entry(entry.clone()).is_err());
        assert!(manager.view().installed.is_empty());
        assert!(!root.join("installed.json").exists());
        manager.finish(Err("Installation cancelled".into()));
        assert_eq!(manager.view().failure.unwrap().code, "FX_INSTALL_CANCELLED");
        manager
            .begin("install", Some(entry.id.clone()), entry.artifact.size)
            .unwrap();
        assert!(!manager.install_entry(entry.clone()).unwrap());
        assert!(
            manager.cancel_install().is_err(),
            "commit is not cancellable"
        );
        manager.finish(Ok(()));
        assert_eq!(manager.view().installed[0].id, entry.id);
        drop(manager);
        std::fs::remove_dir_all(root).unwrap();
    }
}
type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub key: String,
    pub label: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub step: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publisher {
    pub id: String,
    pub name: String,
}
impl Default for Publisher {
    fn default() -> Self {
        Self {
            id: "leotrax3d".into(),
            name: "freeRGB Team".into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[serde(default)]
    pub publisher: Publisher,
    pub id: String,
    pub version: String,
    pub api: u32,
    pub name: String,
    pub description: String,
    pub category: String,
    /// Two RGB hex colours for a static preview, never executable UI content.
    pub preview: [String; 2],
    pub parameters: Vec<Parameter>,
    /// API v1 supports no sensitive inputs; unknown permissions fail closed.
    pub permissions: Vec<String>,
    pub artifact: Artifact,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema: u32,
    pub sequence: u64,
    pub effects: Vec<Entry>,
}
fn safe(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && !s.contains("..")
}
impl Entry {
    pub fn validate(&self) -> Result<()> {
        if !safe(&self.publisher.id)
            || self.publisher.name.is_empty()
            || self.publisher.name.len() > 80
            || !safe(&self.id)
            || !safe(&self.version)
            || self.api != 1
            || !self.permissions.is_empty()
            || self.name.is_empty()
            || self.name.len() > 80
            || self.description.len() > 1000
            || self.category.len() > 64
            || self.parameters.len() > 16
            || self.artifact.target != "wasm32-unknown-unknown"
            || !safe(&self.artifact.url)
            || !self.artifact.url.ends_with(".wasm")
            || self.artifact.size == 0
            || self.artifact.size > MAX_WASM as u64
            || self.artifact.sha256.len() != 64
            || !self
                .artifact
                .sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || self.preview.iter().any(|s| {
                s.len() != 7
                    || !s.starts_with('#')
                    || !s[1..].bytes().all(|b| b.is_ascii_hexdigit())
            })
        {
            return Err("Unsupported or invalid effect package metadata".into());
        }
        let mut keys = BTreeSet::new();
        for p in &self.parameters {
            if !safe(&p.key)
                || !keys.insert(&p.key)
                || p.label.is_empty()
                || p.label.len() > 80
                || ![p.min, p.max, p.default, p.step]
                    .iter()
                    .all(|v| v.is_finite())
                || p.min >= p.max
                || p.min.abs() > 1e9
                || p.max.abs() > 1e9
                || p.step <= 0.0
                || p.step > p.max - p.min
                || !(p.min..=p.max).contains(&p.default)
            {
                return Err("Invalid effect parameter schema".into());
            }
        }
        Ok(())
    }
    pub fn values(&self, parameters: &BTreeMap<String, f64>) -> Result<Vec<f64>> {
        if parameters
            .keys()
            .any(|k| !self.parameters.iter().any(|p| &p.key == k))
        {
            return Err("Unknown effect parameter".into());
        }
        self.parameters
            .iter()
            .map(|p| {
                let value = parameters.get(&p.key).copied().unwrap_or(p.default);
                if !value.is_finite() || !(p.min..=p.max).contains(&value) {
                    Err(format!("{} must be in {}..={}", p.label, p.min, p.max))
                } else {
                    Ok(value)
                }
            })
            .collect()
    }
}
impl Catalog {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 256 * 1024 {
            return Err("Catalog too large".into());
        }
        let c: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if c.schema != 1 || c.effects.len() > 128 {
            return Err("Unsupported catalog schema or size".into());
        }
        let mut ids = BTreeSet::new();
        for e in &c.effects {
            e.validate()?;
            if !ids.insert(&e.id) {
                return Err("Duplicate effect id".into());
            }
        }
        Ok(c)
    }
}
#[derive(Clone, Serialize)]
pub struct Job {
    pub action: String,
    pub id: Option<String>,
    pub stage: String,
    pub bytes: u64,
    pub total: u64,
}
#[derive(Clone, Serialize)]
pub struct Failure {
    pub code: String,
    pub effect_id: Option<String>,
    pub version: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct View {
    pub source: String,
    pub catalog: Vec<Entry>,
    pub installed: Vec<Entry>,
    pub disabled: BTreeSet<String>,
    pub job: Option<Job>,
    pub error: Option<String>,
    pub failure: Option<Failure>,
    pub ratings: crate::ratings::Ratings,
    pub community: crate::ratings::CommunitySettings,
}
struct State {
    catalog: Catalog,
    proof: Option<crate::trust::Proof>,
    disabled: BTreeSet<String>,
    installed: BTreeMap<String, Entry>,
    job: Option<Job>,
    error: Option<String>,
    failure: Option<Failure>,
    ratings: crate::ratings::Ratings,
    ratings_attempt: Option<std::time::Instant>,
}
pub struct Manager {
    root: PathBuf,
    source: Source,
    bundled: bool,
    ratings_client: Option<Arc<crate::ratings::Client>>,
    community: Mutex<crate::ratings::CommunitySettings>,
    state: Mutex<State>,
    cancel: AtomicBool,
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Stored effect file exceeds limit".into());
    }
    Ok(bytes)
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    use std::io::Write;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|e| e.to_string())
}
fn verified(entry: &Entry, bytes: &[u8]) -> Result<()> {
    if bytes.len() as u64 != entry.artifact.size
        || hex::encode(Sha256::digest(bytes)) != entry.artifact.sha256
    {
        return Err("Effect size or SHA-256 verification failed".into());
    }
    Ok(())
}
fn bundled(id: &str) -> Option<&'static [u8]> {
    match id {
        "aurora" => Some(include_bytes!(
            "../../../effects/packages/aurora-1.0.0.wasm"
        )),
        "comet" => Some(include_bytes!("../../../effects/packages/comet-1.0.0.wasm")),
        "prism" => Some(include_bytes!("../../../effects/packages/prism-1.0.0.wasm")),
        _ => None,
    }
}
impl Manager {
    pub fn new(root: PathBuf) -> Result<Arc<Self>> {
        Self::open(
            root,
            Source::parse(OFFICIAL).map_err(|e| e.to_string())?,
            true,
        )
    }
    /// Explicit startup-only development override, never exposed over IPC.
    pub fn development(root: PathBuf, source: PathBuf) -> Result<Arc<Self>> {
        Self::open(root, Source::Directory(source), false)
    }
    fn open(root: PathBuf, source: Source, bundled: bool) -> Result<Arc<Self>> {
        std::fs::create_dir_all(root.join("packages")).map_err(|e| e.to_string())?;
        let proof_path = root.join("trusted-catalog.json");
        let proof: Option<crate::trust::Proof> = if bundled && proof_path.exists() {
            Some(
                serde_json::from_slice(&read(&proof_path, 512 * 1024)?)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let catalog = if let Some(proof) = &proof {
            proof.verify()?
        } else if bundled {
            Catalog::parse(include_bytes!("../../../effects/catalog.json"))?
        } else if root.join("catalog.json").exists() {
            Catalog::parse(&read(&root.join("catalog.json"), 256 * 1024)?)?
        } else {
            Catalog {
                schema: 1,
                sequence: 0,
                effects: Vec::new(),
            }
        };
        let disabled = if root.join("disabled.json").exists() {
            serde_json::from_slice(&read(&root.join("disabled.json"), 16 * 1024)?)
                .map_err(|e| e.to_string())?
        } else {
            BTreeSet::new()
        };
        let index = root.join("installed.json");
        let installed: BTreeMap<String, Entry> = if index.exists() {
            serde_json::from_slice(&read(&index, 512 * 1024)?).map_err(|e| e.to_string())?
        } else {
            BTreeMap::new()
        };
        for (id, e) in &installed {
            e.validate()?;
            if id != &e.id {
                return Err("Invalid installed effect index".into());
            }
        }
        // Local effect development must never send ratings or download receipts.
        let (ratings_client, ratings_error) = match if matches!(source, Source::Directory(_)) {
            Ok(None)
        } else {
            crate::ratings::Client::configured(root.join("ratings-session.json"))
        } {
            Ok(client) => (client.map(Arc::new), None),
            Err(error) => (None, Some(error)),
        };
        // A missing or unreadable file means "off": never fall back to online.
        let community: crate::ratings::CommunitySettings = root
            .join("community.json")
            .exists()
            .then(|| read(&root.join("community.json"), 4 * 1024).ok())
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        if let Some(client) = &ratings_client {
            if !community.counting() {
                client.discard_queued_downloads();
            }
        }
        let ratings = crate::ratings::Ratings {
            configured: ratings_client.is_some() && community.online,
            error: ratings_error,
            ..Default::default()
        };
        Ok(Arc::new(Self {
            cancel: AtomicBool::new(false),
            ratings_client,
            community: Mutex::new(community),
            root,
            source,
            bundled,
            state: Mutex::new(State {
                catalog,
                proof,
                disabled,
                installed,
                job: None,
                error: None,
                failure: None,
                ratings,
                ratings_attempt: None,
            }),
        }))
    }
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn community_settings(&self) -> crate::ratings::CommunitySettings {
        *self.community.lock().unwrap_or_else(|e| e.into_inner())
    }
    /// Changes and persists the community switches. Turning counting (or the
    /// whole service) off also deletes receipts that were queued but not sent.
    /// Counting cannot be enabled while the service is off.
    pub fn set_community(
        &self,
        online: Option<bool>,
        count_downloads: Option<bool>,
    ) -> Result<crate::ratings::CommunitySettings> {
        let mut guard = self.community.lock().unwrap_or_else(|e| e.into_inner());
        let mut next = *guard;
        if let Some(online) = online {
            next.online = online;
        }
        if let Some(count) = count_downloads {
            next.count_downloads = count;
        }
        if !next.online {
            next.count_downloads = false;
        }
        if count_downloads == Some(true) && !next.online {
            return Err("Turn on online community features before download counting".into());
        }
        atomic(&self.root.join("community.json"), &json(&next)?)?;
        *guard = next;
        drop(guard);
        if !next.counting() {
            if let Some(client) = &self.ratings_client {
                client.discard_queued_downloads();
            }
        }
        let mut s = self.lock();
        if !next.online {
            s.ratings = crate::ratings::Ratings {
                configured: false,
                ..Default::default()
            };
        } else {
            s.ratings.configured = self.ratings_client.is_some();
            s.ratings_attempt = None;
        }
        Ok(next)
    }
    /// The community client, only while the user has switched the service on.
    fn community_client(&self) -> Option<Arc<crate::ratings::Client>> {
        if !self.community_settings().online {
            return None;
        }
        self.ratings_client.clone()
    }
    pub fn view(&self) -> View {
        let community = self.community_settings();
        let s = self.lock();
        View {
            source: self.source.describe(),
            catalog: s.catalog.effects.clone(),
            installed: s.installed.values().cloned().collect(),
            disabled: s.disabled.clone(),
            job: s.job.clone(),
            error: s.error.clone(),
            failure: s.failure.clone(),
            ratings: crate::ratings::Ratings {
                configured: s.ratings.configured && community.online,
                ..s.ratings.clone()
            },
            community,
        }
    }
    fn effect_ids(&self) -> Vec<String> {
        let s = self.lock();
        s.catalog
            .effects
            .iter()
            .chain(s.installed.values())
            .map(|e| e.id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn submit_report(
        &self,
        kind: &str,
        payload: &serde_json::Value,
        request_id: &str,
    ) -> Result<serde_json::Value> {
        self.community_client()
            .ok_or(COMMUNITY_OFF)?
            .submit(kind, payload, request_id)
    }
    pub fn inbox(&self, action: &str, params: &serde_json::Value) -> Result<serde_json::Value> {
        self.community_client()
            .ok_or(COMMUNITY_OFF)?
            .inbox(action, params)
    }
    pub fn refresh_ratings(self: &Arc<Self>) -> Result<()> {
        let Some(client) = self.community_client() else {
            return Ok(());
        };
        if !self.community_settings().counting() {
            client.discard_queued_downloads();
        }
        {
            let mut s = self.lock();
            if s.ratings.loading || s.ratings.saving.is_some() {
                return Ok(());
            }
            if s.ratings_attempt
                .is_some_and(|t| t.elapsed().as_secs() < 60)
            {
                return Ok(());
            }
            s.ratings_attempt = Some(std::time::Instant::now());
            s.ratings.loading = true;
            s.ratings.error = None;
        }
        let manager = self.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("effect-ratings".into())
            .spawn(move || {
                let result = client.fetch(&manager.effect_ids());
                let mut s = manager.lock();
                match result {
                    Ok(ratings) => s.ratings = ratings,
                    Err(error) => {
                        s.ratings.loading = false;
                        s.ratings.error = Some(error);
                    }
                }
            })
        {
            let mut s = self.lock();
            s.ratings.loading = false;
            s.ratings.error = Some(error.to_string());
            return Err(error.to_string());
        }
        Ok(())
    }
    pub fn rate(self: &Arc<Self>, id: &str, stars: Option<u8>) -> Result<()> {
        let client = self.community_client().ok_or(COMMUNITY_OFF)?;
        if !self.community_settings().counting() {
            client.discard_queued_downloads();
        }
        {
            let mut s = self.lock();
            if !s
                .catalog
                .effects
                .iter()
                .chain(s.installed.values())
                .any(|e| e.id == id)
            {
                return Err("Unknown effect".into());
            }
            if s.ratings.loading || s.ratings.saving.is_some() {
                return Err("A rating request is already running".into());
            }
            s.ratings.saving = Some(id.into());
            s.ratings.error = None;
        }
        let manager = self.clone();
        let id = id.to_owned();
        if let Err(error) = std::thread::Builder::new()
            .name("effect-rating-save".into())
            .spawn(move || {
                let saved = client.rate(&id, stars);
                let was_saved = saved.is_ok();
                let result = saved.and_then(|()| client.fetch(&manager.effect_ids()));
                let mut s = manager.lock();
                match result {
                    Ok(ratings) => s.ratings = ratings,
                    Err(error) => {
                        if was_saved {
                            if let Some(stars) = stars {
                                s.ratings.mine.insert(id.clone(), stars);
                            } else {
                                s.ratings.mine.remove(&id);
                            }
                            s.ratings.error = Some(format!(
                                "Your rating was saved, but scores could not be refreshed: {error}"
                            ));
                        } else {
                            s.ratings.error = Some(error);
                        }
                        s.ratings.saving = None;
                    }
                }
            })
        {
            let mut s = self.lock();
            s.ratings.saving = None;
            s.ratings.error = Some(error.to_string());
            return Err(error.to_string());
        }
        Ok(())
    }
    pub fn busy(&self) -> bool {
        self.lock().job.is_some()
    }
    pub fn cancel_install(&self) -> Result<()> {
        let state = self.lock();
        match &state.job {
            Some(job) if job.action == "install" && job.stage != "committing" => {
                self.cancel.store(true, Ordering::SeqCst);
                Ok(())
            }
            _ => Err("No cancellable installation is running".into()),
        }
    }
    fn begin(&self, action: &str, id: Option<String>, total: u64) -> Result<()> {
        let mut s = self.lock();
        if s.job.is_some() {
            return Err("Another marketplace operation is running".into());
        }
        s.error = None;
        self.cancel.store(false, Ordering::SeqCst);
        s.failure = None;
        s.job = Some(Job {
            action: action.into(),
            id,
            stage: "downloading".into(),
            bytes: 0,
            total,
        });
        Ok(())
    }
    fn finish(&self, result: Result<()>) {
        let mut s = self.lock();
        s.error = result.err();
        s.failure = s.error.as_ref().map(|_| {
            let id = s.job.as_ref().and_then(|j| j.id.clone());
            let version = id.as_ref().and_then(|id| {
                s.catalog
                    .effects
                    .iter()
                    .find(|e| &e.id == id)
                    .map(|e| e.version.clone())
            });
            Failure {
                code: if s.error.as_deref() == Some("Installation cancelled") {
                    "FX_INSTALL_CANCELLED"
                } else if s.job.as_ref().is_some_and(|j| j.action == "install") {
                    "FX_INSTALL_FAILED"
                } else {
                    "FX_CATALOG_FAILED"
                }
                .into(),
                effect_id: id,
                version,
            }
        });
        s.job = None;
    }
    pub fn refresh(self: &Arc<Self>) -> Result<()> {
        self.begin("refresh", None, 0)?;
        let manager = self.clone();
        if let Err(e) = std::thread::Builder::new()
            .name("effect-catalog".into())
            .spawn(move || {
                let result = (|| {
                    let bytes = Fetcher::new(manager.source.clone())
                        .small("catalog.json", 256 * 1024)
                        .map_err(|e| e.to_string())?;
                    let proof = if manager.bundled {
                        let signature = Fetcher::new(manager.source.clone())
                            .small("catalog.sig", 256)
                            .map_err(|e| e.to_string())?;
                        crate::trust::verify(&bytes, &signature)?;
                        Some(crate::trust::Proof {
                            catalog: String::from_utf8(bytes.clone()).map_err(|e| e.to_string())?,
                            signature: String::from_utf8(signature).map_err(|e| e.to_string())?,
                        })
                    } else {
                        None
                    };
                    let catalog = Catalog::parse(&bytes)?;
                    let mut s = manager.lock();
                    if catalog.sequence < s.catalog.sequence
                        || (catalog.sequence == s.catalog.sequence && catalog != s.catalog)
                    {
                        return Err("Catalog rollback or conflicting sequence rejected".into());
                    }
                    if let Some(proof) = &proof {
                        atomic(&manager.root.join("trusted-catalog.json"), &json(proof)?)?;
                    } else {
                        atomic(&manager.root.join("catalog.json"), &bytes)?;
                    }
                    s.proof = proof;
                    s.catalog = catalog;
                    Ok(())
                })();
                manager.finish(result);
            })
        {
            self.finish(Err(e.to_string()));
            return Err(e.to_string());
        }
        Ok(())
    }
    pub fn install(self: &Arc<Self>, id: &str) -> Result<()> {
        let entry = self
            .lock()
            .catalog
            .effects
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or("Effect is not in the official catalog")?;
        self.begin("install", Some(id.into()), entry.artifact.size)?;
        let manager = self.clone();
        if let Err(e) = std::thread::Builder::new()
            .name("effect-install".into())
            .spawn(move || {
                let receipt = (entry.id.clone(), entry.version.clone());
                let mut result = manager.install_entry(entry);
                if result.is_err() && manager.cancel.load(Ordering::SeqCst) {
                    result = Err("Installation cancelled".into());
                }
                let downloaded = matches!(result, Ok(true));
                manager.finish(result.map(|_| ()));
                if downloaded {
                    let client = manager
                        .community_settings()
                        .counting()
                        .then(|| manager.ratings_client.clone())
                        .flatten();
                    if let Some(client) = &client {
                        if let Err(error) = client.download(&receipt.0, &receipt.1) {
                            manager.lock().ratings.error = Some(format!(
                                "Installed successfully; download count could not be sent: {error}"
                            ));
                        }
                    }
                }
            })
        {
            self.finish(Err(e.to_string()));
            return Err(e.to_string());
        }
        Ok(())
    }
    fn install_entry(&self, entry: Entry) -> Result<bool> {
        let bundled_entry = self.bundled
            && Catalog::parse(include_bytes!("../../../effects/catalog.json"))?
                .effects
                .contains(&entry)
            && bundled(&entry.id).is_some();
        let progress = |bytes| {
            if let Some(job) = &mut self.lock().job {
                job.bytes = bytes;
            }
        };
        let bytes = if self.bundled {
            let embedded = Catalog::parse(include_bytes!("../../../effects/catalog.json"))?;
            match embedded
                .effects
                .iter()
                .find(|e| **e == entry)
                .and_then(|_| bundled(&entry.id))
            {
                Some(bytes) => {
                    progress(bytes.len() as u64);
                    bytes.to_vec()
                }
                None => self.download(&entry, &progress)?,
            }
        } else {
            self.download(&entry, &progress)?
        };
        verified(&entry, &bytes)?;
        if self.cancel.load(Ordering::SeqCst) {
            return Err("Installation cancelled".into());
        }
        if let Some(job) = &mut self.lock().job {
            job.stage = "validating".into();
        }
        let host = crate::process::HostCommand::sibling().map_err(|e| e.to_string())?;
        crate::process::ProcessEffect::load(&host, &bytes, &entry.values(&BTreeMap::new())?)?;
        {
            let mut state = self.lock();
            if self.cancel.load(Ordering::SeqCst) {
                return Err("Installation cancelled".into());
            }
            if let Some(job) = &mut state.job {
                job.stage = "committing".into();
            }
        }
        let dir = self.root.join("packages").join(&entry.id);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let meta = dir.join(format!("{}.json", entry.version));
        if meta.exists() {
            let old: Entry =
                serde_json::from_slice(&read(&meta, 64 * 1024)?).map_err(|e| e.to_string())?;
            if old != entry {
                return Err("Published effect versions are immutable; use a new version".into());
            }
        }
        if self.bundled
            && !Catalog::parse(include_bytes!("../../../effects/catalog.json"))?
                .effects
                .contains(&entry)
        {
            let proof = self
                .lock()
                .proof
                .clone()
                .ok_or("Missing signed effect catalog")?;
            if !proof.verify()?.effects.contains(&entry) {
                return Err("Package is not in signed catalog".into());
            }
            atomic(
                &dir.join(format!("{}.proof.json", entry.version)),
                &json(&proof)?,
            )?;
        }
        atomic(&dir.join(format!("{}.wasm", entry.version)), &bytes)?;
        atomic(&meta, &json(&entry)?)?;
        let mut s = self.lock();
        let mut installed = s.installed.clone();
        installed.insert(entry.id.clone(), entry);
        if installed.len() > 128 {
            return Err("At most 128 effects may be installed".into());
        }
        let index = json(&installed)?;
        if index.len() > 512 * 1024 {
            return Err("Installed effect metadata exceeds the store limit".into());
        }
        atomic(&self.root.join("installed.json"), &index)?;
        s.installed = installed;
        Ok(!bundled_entry)
    }
    fn download(&self, entry: &Entry, progress: &dyn Fn(u64)) -> Result<Vec<u8>> {
        let path = Fetcher::new(self.source.clone())
            .download(
                &entry.artifact,
                &self.root.join("downloads"),
                &Progress {
                    cancel: &self.cancel,
                    on_bytes: progress,
                },
            )
            .map_err(|e| e.to_string())?;
        read(&path, MAX_WASM)
    }
    pub fn enabled(&self, id: &str) -> bool {
        let s = self.lock();
        s.installed.contains_key(id) && !s.disabled.contains(id)
    }
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let mut s = self.lock();
        if s.job.is_some() {
            return Err("Another marketplace operation is running".into());
        }
        if !s.installed.contains_key(id) {
            return Err("Effect is not installed".into());
        }
        let mut disabled = s.disabled.clone();
        disabled.retain(|id| s.installed.contains_key(id));
        if enabled {
            disabled.remove(id);
        } else {
            disabled.insert(id.into());
        }
        atomic(&self.root.join("disabled.json"), &json(&disabled)?)?;
        s.disabled = disabled;
        Ok(())
    }
    pub fn load(
        self: &Arc<Self>,
        id: &str,
        version: &str,
        parameters: &BTreeMap<String, f64>,
        fault: Fault,
    ) -> Result<crate::lifecycle::ManagedEffect> {
        crate::lifecycle::ManagedEffect::load(self.clone(), id, version, parameters, fault)
    }
    pub(crate) fn package(
        &self,
        id: &str,
        version: &str,
        parameters: &BTreeMap<String, f64>,
    ) -> Result<(Vec<u8>, Vec<f64>)> {
        if !safe(id) || !safe(version) {
            return Err("Invalid effect reference".into());
        }
        if !self.enabled(id) {
            return Err("Effect is disabled or not installed".into());
        }
        let dir = self.root.join("packages").join(id);
        let entry: Entry =
            serde_json::from_slice(&read(&dir.join(format!("{version}.json")), 64 * 1024)?)
                .map_err(|e| e.to_string())?;
        entry.validate()?;
        if entry.id != id || entry.version != version {
            return Err("Stored package identity mismatch".into());
        }
        if self.bundled
            && !Catalog::parse(include_bytes!("../../../effects/catalog.json"))?
                .effects
                .contains(&entry)
        {
            let proof: crate::trust::Proof = serde_json::from_slice(&read(
                &dir.join(format!("{version}.proof.json")),
                512 * 1024,
            )?)
            .map_err(|e| e.to_string())?;
            if !proof.verify()?.effects.contains(&entry) {
                return Err("Stored package is not signed".into());
            }
        }
        let bytes = read(&dir.join(format!("{version}.wasm")), MAX_WASM)?;
        verified(&entry, &bytes)?;
        Ok((bytes, entry.values(parameters)?))
    }
    pub(crate) fn reload_error(&self, error: String) {
        self.lock().error = Some(error);
    }
    pub fn uninstall(&self, id: &str) -> Result<()> {
        let mut s = self.lock();
        if s.job.is_some() {
            return Err("Another marketplace operation is running".into());
        }
        if !s.installed.contains_key(id) {
            return Err("Effect is not installed".into());
        }
        let mut installed = s.installed.clone();
        installed.remove(id);
        atomic(&self.root.join("installed.json"), &json(&installed)?)?;
        s.installed = installed;
        // Retain immutable versions so existing saved profiles can report a
        // missing installation cleanly; reinstall restores them without downloads.
        Ok(())
    }
}
#[cfg(test)]
mod community_tests {
    use super::*;
    use crate::ratings::Client;
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    fn directory(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "freergb-community-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    /// Local release directory with the official catalog and artifacts.
    fn release() -> PathBuf {
        let release = directory("release");
        let bytes = include_bytes!("../../../effects/catalog.json");
        std::fs::write(release.join("catalog.json"), bytes).unwrap();
        for entry in Catalog::parse(bytes).unwrap().effects {
            std::fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../effects/packages")
                    .join(&entry.artifact.url),
                release.join(&entry.artifact.url),
            )
            .unwrap();
        }
        release
    }
    /// A manager whose community client points at a local listener that
    /// records every connection attempt.
    fn with_client(root: &Path, release: &Path) -> (Arc<Manager>, TcpListener) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let mut manager = Manager::development(root.to_path_buf(), release.to_path_buf()).unwrap();
        let client = Client::new(
            url,
            "test-project".into(),
            root.join("ratings-session.json"),
            true,
        )
        .unwrap();
        Arc::get_mut(&mut manager).unwrap().ratings_client = Some(Arc::new(client));
        (manager, listener)
    }
    fn wait_idle(manager: &Arc<Manager>) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while manager.busy() {
            assert!(Instant::now() < deadline, "job timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn install(manager: &Arc<Manager>) {
        manager.refresh().unwrap();
        wait_idle(manager);
        manager.install("aurora").unwrap();
        wait_idle(manager);
        assert_eq!(manager.view().installed[0].id, "aurora");
    }
    fn connected(listener: &TcpListener, wait: Duration) -> bool {
        let deadline = Instant::now() + wait;
        loop {
            if listener.accept().is_ok() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn community_files(root: &Path) -> Vec<String> {
        std::fs::read_dir(root)
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.starts_with("ratings-session"))
            .collect()
    }

    #[test]
    fn defaults_are_off_and_install_creates_no_identity_or_download_record() {
        let (root, release) = (directory("off"), release());
        let (manager, listener) = with_client(&root, &release);
        assert_eq!(
            manager.community_settings(),
            crate::ratings::CommunitySettings::default()
        );
        assert!(!manager.community_settings().online);
        assert!(!manager.community_settings().count_downloads);
        assert!(!manager.view().ratings.configured);
        install(&manager);
        manager.refresh_ratings().unwrap();
        assert!(manager.rate("aurora", Some(5)).is_err());
        assert!(manager
            .submit_report("feedback", &serde_json::json!({}), "request-1")
            .is_err());
        assert!(manager.inbox("inbox", &serde_json::json!({})).is_err());
        assert!(
            !connected(&listener, Duration::from_millis(500)),
            "offline mode must not contact the community service"
        );
        assert!(community_files(&root).is_empty(), "no identity or receipt");
        assert!(manager.view().ratings.by_effect.is_empty());
    }

    #[test]
    fn online_without_counting_never_sends_or_queues_a_download_receipt() {
        let (root, release) = (directory("nocount"), release());
        let (manager, listener) = with_client(&root, &release);
        // Counting cannot be enabled while the service is off.
        assert!(manager.set_community(None, Some(true)).is_err());
        manager.set_community(Some(true), None).unwrap();
        assert!(!manager.community_settings().count_downloads);
        install(&manager);
        assert!(
            !connected(&listener, Duration::from_millis(500)),
            "an install alone must not contact the service when counting is off"
        );
        assert!(community_files(&root).is_empty(), "no identity or receipt");
        // Positive control: online features do reach the service when enabled.
        manager.rate("aurora", Some(4)).unwrap();
        assert!(connected(&listener, Duration::from_secs(10)));
    }

    #[test]
    fn counting_is_opt_in_persisted_and_revocable() {
        let (root, release) = (directory("count"), release());
        let (manager, listener) = with_client(&root, &release);
        manager.set_community(Some(true), Some(true)).unwrap();
        install(&manager);
        assert!(
            connected(&listener, Duration::from_secs(10)),
            "opted-in install sends a receipt"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let queue = root.join("ratings-session.downloads");
        while !queue.exists() {
            assert!(Instant::now() < deadline, "receipt was not queued");
            std::thread::sleep(Duration::from_millis(10));
        }
        // Revoking counting deletes unsent receipts and survives a restart.
        manager.set_community(None, Some(false)).unwrap();
        assert!(!queue.exists());
        drop(manager);
        let (reopened, _listener) = with_client(&root, &release);
        assert!(reopened.community_settings().online);
        assert!(!reopened.community_settings().count_downloads);
        reopened.set_community(Some(false), None).unwrap();
        assert!(!reopened.community_settings().online);
        assert!(!reopened.view().ratings.configured);
    }
    #[test]
    fn corrupt_settings_file_means_off() {
        let (root, release) = (directory("corrupt"), release());
        std::fs::write(root.join("community.json"), b"{not json").unwrap();
        let (manager, _listener) = with_client(&root, &release);
        assert!(!manager.community_settings().online);
    }
}
