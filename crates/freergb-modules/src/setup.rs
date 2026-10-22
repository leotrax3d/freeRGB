//! Service-side device setup: candidates, selection, shared downloads,
//! installation and the per-device phases the GUI shows.
//!
//! All work runs on a daemon thread; the GUI only polls [`Manager::status`]
//! and sends commands, so closing the window never stops a setup.
use crate::catalog::{Catalog, ModuleEntry, MAX_CATALOG};
use crate::source::{Fetcher, Progress, Source, CATALOG_FILE, SIGNATURE_FILE};
use crate::store::Store;
use crate::trust::TrustedKeys;
use crate::{invalid, Result};
use freergb_module_api::{DeviceKind, InterfaceInfo};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Initialization must finish within this time after installation.
pub const READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Vendors shown as "not yet supported" even before a catalog is loaded.
const KNOWN_RGB_VENDORS: &[(u16, &str)] = &[
    (0x1532, "Razer"),
    (0x1B1C, "Corsair"),
    (0x0B05, "ASUS"),
    (0x1038, "SteelSeries"),
    (0x0951, "HyperX"),
    (0x03F0, "HyperX"),
    (0x2516, "Cooler Master"),
    (0x1E71, "NZXT"),
    (0x264A, "Thermaltake"),
    (0x3633, "Glorious"),
    (0x0CF2, "Lian Li"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Detected,
    Waiting,
    Downloading,
    Verifying,
    Installing,
    Initializing,
    Ready,
    Failed,
    NotConnected,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CardError {
    /// offline | permission | busy | invalid_package | incompatible |
    /// initialization | timeout | removed | cancelled | source
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Bytes {
    pub done: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Card {
    pub key: String,
    pub name: String,
    pub vendor: String,
    pub kind: DeviceKind,
    pub phase: Phase,
    pub module: Option<String>,
    /// Package size of the module this card needs (shared modules: same value).
    pub download_size: Option<u64>,
    /// Bytes of that module's package (shared between cards of one module).
    pub progress: Option<Bytes>,
    pub error: Option<CardError>,
    pub configured: bool,
    /// The user deselected it earlier; not offered again as new.
    pub dismissed: bool,
    /// Built into freeRGB; needs no module.
    pub builtin: bool,
    /// Module already installed and compatible; no download needed.
    pub installed: bool,
    pub connected: bool,
    pub engine_id: Option<String>,
    /// Shown in the "copy technical information" action.
    pub technical: String,
    #[serde(skip)]
    pub seen: Instant,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CatalogStatus {
    /// none | loading | ready | cached | error
    pub state: &'static str,
    pub source: String,
    pub message: Option<String>,
    pub sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Job {
    pub running: bool,
    pub cancelled: bool,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub first_run: bool,
    pub cards: Vec<Card>,
    pub catalog: CatalogStatus,
    pub job: Job,
    pub installed: Vec<crate::store::InstalledSummary>,
    pub trusted_release_key: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Persisted {
    first_run_complete: bool,
    /// Device key -> module id.
    configured: BTreeMap<String, String>,
    /// Keys the user deselected; not offered again as "new".
    dismissed: BTreeSet<String>,
    last_sequence: u64,
}

/// One HID interface seen by the daemon's scan.
#[derive(Debug, Clone)]
pub struct Seen {
    pub interface: InterfaceInfo,
    pub manufacturer: Option<String>,
    /// Interface is driven by a built-in adapter (name).
    pub builtin: Option<String>,
}

/// Runtime state of engine devices created from modules (key -> status).
#[derive(Debug, Clone)]
pub struct Runtime {
    pub engine_id: String,
    pub connected: bool,
    pub error: Option<String>,
}

pub fn device_key(i: &InterfaceInfo) -> String {
    format!(
        "usb:{:04x}:{:04x}:if{}:{:04x}:{:04x}",
        i.vid, i.pid, i.interface, i.usage_page, i.usage
    )
}

struct Inner {
    persisted: Persisted,
    cards: BTreeMap<String, Card>,
    catalog: Option<Catalog>,
    catalog_status: CatalogStatus,
    job: Job,
    /// Ready deadline per key after installation.
    awaiting: BTreeMap<String, Instant>,
}

pub struct Manager {
    store: Store,
    keys: TrustedKeys,
    source: Source,
    config: PathBuf,
    inner: Mutex<Inner>,
    cancel: AtomicBool,
    /// Set when the daemon should rescan hardware right away.
    pub rescan: AtomicBool,
    module_bytes: Mutex<BTreeMap<String, Arc<AtomicU64>>>,
}

fn lock(m: &Mutex<Inner>) -> MutexGuard<'_, Inner> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn classify(error: &std::io::Error) -> &'static str {
    use std::io::ErrorKind::*;
    match error.kind() {
        Interrupted => "cancelled",
        TimedOut => "timeout",
        PermissionDenied => "permission",
        NotFound => "source",
        InvalidData => "invalid_package",
        _ => "offline",
    }
}

impl Manager {
    pub fn new(store: Store, keys: TrustedKeys, source: Source, config: PathBuf) -> Self {
        let persisted: Persisted = std::fs::read(config.join("modules-setup.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        // Last verified catalog: offline matching and "not yet supported".
        let catalog = std::fs::read(config.join("modules-catalog.json"))
            .ok()
            .zip(std::fs::read(config.join("modules-catalog.json.sig")).ok())
            .and_then(|(c, s)| Catalog::verify(&c, &s, &keys).ok());
        let catalog_status = CatalogStatus {
            state: if catalog.is_some() { "cached" } else { "none" },
            source: source.describe(),
            message: None,
            sequence: catalog.as_ref().map(|c| c.sequence),
        };
        Self {
            store,
            keys,
            source,
            config,
            inner: Mutex::new(Inner {
                persisted,
                cards: BTreeMap::new(),
                catalog,
                catalog_status,
                job: Job::default(),
                awaiting: BTreeMap::new(),
            }),
            cancel: AtomicBool::new(false),
            rescan: AtomicBool::new(false),
            module_bytes: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    fn save(&self, persisted: &Persisted) {
        let path = self.config.join("modules-setup.json");
        let _ = std::fs::create_dir_all(&self.config);
        let temporary = path.with_extension("tmp");
        if std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(persisted).unwrap_or_default(),
        )
        .is_ok()
        {
            let _ = std::fs::rename(&temporary, &path);
        }
    }

    /// Devices whose module should run: key -> module id.
    pub fn configured(&self) -> BTreeMap<String, String> {
        lock(&self.inner).persisted.configured.clone()
    }

    /// Fetches and authenticates the catalog. Keeps the cached one on failure.
    pub fn refresh_catalog(&self) {
        {
            let mut inner = lock(&self.inner);
            inner.catalog_status.state = "loading";
            inner.catalog_status.message = None;
        }
        let result = (|| -> Result<Catalog> {
            let mut fetcher = Fetcher::new(self.source.clone());
            let bytes = fetcher.small(CATALOG_FILE, MAX_CATALOG)?;
            let signature = fetcher.small(SIGNATURE_FILE, 1024)?;
            let catalog = Catalog::verify(&bytes, &signature, &self.keys)?;
            let last = lock(&self.inner).persisted.last_sequence;
            if catalog.sequence < last {
                return Err(invalid(
                    "catalog is older than the last accepted one (rollback refused)",
                ));
            }
            let _ = std::fs::create_dir_all(&self.config);
            std::fs::write(self.config.join("modules-catalog.json"), &bytes)?;
            std::fs::write(self.config.join("modules-catalog.json.sig"), &signature)?;
            Ok(catalog)
        })();
        let mut inner = lock(&self.inner);
        match result {
            Ok(catalog) => {
                inner.persisted.last_sequence = catalog.sequence;
                inner.catalog_status = CatalogStatus {
                    state: "ready",
                    source: self.source.describe(),
                    message: None,
                    sequence: Some(catalog.sequence),
                };
                inner.catalog = Some(catalog);
                let persisted = inner.persisted.clone();
                drop(inner);
                self.save(&persisted);
            }
            Err(error) => {
                inner.catalog_status.state = if inner.catalog.is_some() {
                    "cached"
                } else {
                    "error"
                };
                inner.catalog_status.message = Some(error.to_string());
            }
        }
    }

    fn technical(seen: &Seen, catalog: Option<&Catalog>) -> String {
        let i = &seen.interface;
        format!(
            "USB vendor ID: 0x{:04X}\nUSB product ID: 0x{:04X}\nInterface: {}\nUsage page: 0x{:04X}\nUsage: 0x{:04X}\nManufacturer: {}\nProduct: {}\nfreeRGB: {} ({}), module API {}, catalog {}",
            i.vid,
            i.pid,
            i.interface,
            i.usage_page,
            i.usage,
            seen.manufacturer.as_deref().unwrap_or("-"),
            i.product.as_deref().unwrap_or("-"),
            env!("CARGO_PKG_VERSION"),
            crate::catalog::host_target(),
            freergb_module_api::MODULE_API,
            catalog.map_or("none".to_string(), |c| c.sequence.to_string()),
        )
    }

    /// Updates the cards from one hardware scan. `runtime` is keyed by
    /// device key. Unknown, irrelevant interfaces never become cards.
    pub fn observe(&self, interfaces: &[Seen], runtime: &BTreeMap<String, Runtime>) {
        let installed = self.store.manifests();
        let now = Instant::now();
        let mut inner = lock(&self.inner);
        let mut present = BTreeSet::new();
        let mut unsupported_models = BTreeSet::new();
        for seen in interfaces {
            let i = &seen.interface;
            let key = device_key(i);
            let catalog_match = inner
                .catalog
                .as_ref()
                .and_then(|c| c.match_interface(i))
                .map(|(m, d)| {
                    (
                        m.id.clone(),
                        d.name.clone(),
                        d.kind,
                        m.compatible().ok().map(|a| a.size),
                    )
                });
            let installed_match = installed.iter().find_map(|m| {
                m.devices
                    .iter()
                    .find(|d| d.matches(i))
                    .map(|d| (m.id.clone(), d.name.clone(), d.kind))
            });
            let vendor_name = inner
                .catalog
                .as_ref()
                .and_then(|c| c.is_relevant_vendor(i.vid).map(str::to_owned))
                .or_else(|| {
                    KNOWN_RGB_VENDORS
                        .iter()
                        .find(|(v, _)| *v == i.vid)
                        .map(|(_, n)| (*n).to_owned())
                });
            let (name, kind, module, size, builtin) = if let Some(adapter) = &seen.builtin {
                (adapter.clone(), DeviceKind::Other, None, None, true)
            } else if let Some((id, name, kind)) = installed_match.clone() {
                let size = catalog_match.as_ref().and_then(|m| m.3);
                (name, kind, Some(id), size, false)
            } else if let Some((id, name, kind, size)) = catalog_match.clone() {
                (name, kind, Some(id), size, false)
            } else if let Some(vendor) = &vendor_name {
                // One card per unsupported model, not per HID collection.
                if !unsupported_models.insert((i.vid, i.pid)) {
                    continue;
                }
                let product = i
                    .product
                    .clone()
                    .unwrap_or_else(|| format!("{vendor} device {:04X}", i.pid));
                (product, DeviceKind::Other, None, None, false)
            } else {
                continue;
            };
            // Unsupported cards use a model-level key.
            let key = if module.is_none() && !builtin {
                format!("usb:{:04x}:{:04x}", i.vid, i.pid)
            } else {
                key
            };
            present.insert(key.clone());
            let configured = inner.persisted.configured.contains_key(&key);
            let state = runtime.get(&key);
            let technical = Self::technical(seen, inner.catalog.as_ref());
            let awaiting = inner.awaiting.get(&key).copied();
            let dismissed = inner.persisted.dismissed.contains(&key);
            let card = inner.cards.entry(key.clone()).or_insert_with(|| Card {
                key: key.clone(),
                name: name.clone(),
                vendor: vendor_name.clone().unwrap_or_default(),
                kind,
                phase: Phase::Detected,
                module: module.clone(),
                download_size: size,
                progress: None,
                error: None,
                configured,
                dismissed: false,
                builtin,
                installed: false,
                connected: false,
                engine_id: None,
                technical: technical.clone(),
                seen: now,
            });
            card.seen = now;
            card.technical = technical;
            card.module = module.clone();
            card.download_size = size.or(card.download_size);
            card.installed = installed_match.is_some();
            card.configured = configured;
            card.dismissed = dismissed;
            card.builtin = builtin;
            if let Some(runtime) = state {
                card.engine_id = Some(runtime.engine_id.clone());
                card.connected = runtime.connected;
            }
            card.phase = match () {
                _ if builtin => Phase::Ready,
                _ if module.is_none() => Phase::Unsupported,
                _ if configured => match state {
                    Some(r) if r.connected => {
                        card.error = None;
                        Phase::Ready
                    }
                    Some(r)
                        if r.error.is_some()
                            && awaiting.map_or(true, |d| {
                                now >= d
                                    || r.error.as_deref().is_some_and(|e| {
                                        e.contains("Unsupported") || e.contains("Permission")
                                    })
                            }) =>
                    {
                        card.error = Some(CardError {
                            code: "initialization",
                            message: r.error.clone().unwrap_or_default(),
                        });
                        Phase::Failed
                    }
                    _ if awaiting.is_some_and(|d| now >= d) => {
                        card.error = Some(CardError {
                            code: "timeout",
                            message: "The device did not finish initializing in time.".into(),
                        });
                        Phase::Failed
                    }
                    _ => Phase::Initializing,
                },
                _ if matches!(
                    card.phase,
                    Phase::Waiting
                        | Phase::Downloading
                        | Phase::Verifying
                        | Phase::Installing
                        | Phase::Failed
                ) =>
                {
                    card.phase
                }
                _ => Phase::Detected,
            };
            if card.phase == Phase::Ready {
                inner.awaiting.remove(&key);
                if let Some(id) = &module {
                    let _ = self.store.mark_good(id);
                }
            }
        }
        // Absent devices: configured ones stay visible as "not connected".
        let configured = inner.persisted.configured.clone();
        inner.cards.retain(|key, card| {
            if present.contains(key) {
                return true;
            }
            if configured.contains_key(key) {
                card.phase = Phase::NotConnected;
                card.connected = false;
                card.error = None;
                return true;
            }
            // Removed while being set up: say so instead of vanishing.
            if matches!(
                card.phase,
                Phase::Waiting | Phase::Downloading | Phase::Verifying | Phase::Installing
            ) {
                card.error = Some(CardError {
                    code: "removed",
                    message: "The device was disconnected.".into(),
                });
                card.phase = Phase::Failed;
                return true;
            }
            false
        });
    }

    pub fn status(&self) -> Status {
        let inner = lock(&self.inner);
        let mut cards: Vec<Card> = inner.cards.values().cloned().collect();
        let bytes = self.module_bytes.lock().unwrap_or_else(|e| e.into_inner());
        for card in &mut cards {
            if card.phase == Phase::Downloading {
                if let (Some(module), Some(total)) = (&card.module, card.download_size) {
                    let done = bytes.get(module).map_or(0, |b| b.load(Ordering::Relaxed));
                    card.progress = Some(Bytes {
                        done: done.min(total),
                        total,
                    });
                }
            }
        }
        let mut job = inner.job.clone();
        if job.running {
            job.bytes_done = job
                .modules
                .iter()
                .filter_map(|m| bytes.get(m))
                .map(|b| b.load(Ordering::Relaxed))
                .sum::<u64>()
                .min(job.bytes_total);
        }
        cards.sort_by(|a, b| {
            (a.builtin, a.phase == Phase::Unsupported, &a.name).cmp(&(
                b.builtin,
                b.phase == Phase::Unsupported,
                &b.name,
            ))
        });
        Status {
            first_run: !inner.persisted.first_run_complete,
            cards,
            catalog: inner.catalog_status.clone(),
            job,
            installed: self.store.summaries(),
            trusted_release_key: self.keys.has_release_key(),
        }
    }

    pub fn complete_first_run(&self) {
        let mut inner = lock(&self.inner);
        inner.persisted.first_run_complete = true;
        let persisted = inner.persisted.clone();
        drop(inner);
        self.save(&persisted);
    }

    pub fn dismiss(&self, keys: &[String]) {
        let mut inner = lock(&self.inner);
        inner.persisted.dismissed.extend(keys.iter().cloned());
        let persisted = inner.persisted.clone();
        drop(inner);
        self.save(&persisted);
    }

    pub fn is_dismissed(&self, key: &str) -> bool {
        lock(&self.inner).persisted.dismissed.contains(key)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Removes a device from the configuration (its module stays installed).
    pub fn forget(&self, key: &str) {
        let mut inner = lock(&self.inner);
        inner.persisted.configured.remove(key);
        let persisted = inner.persisted.clone();
        drop(inner);
        self.save(&persisted);
    }

    /// Sets up the selected device keys. Runs to completion on the calling
    /// thread (the daemon spawns one); returns at once if a job is running.
    pub fn run(&self, keys: &[String]) {
        let plan = {
            let mut inner = lock(&self.inner);
            if inner.job.running {
                return;
            }
            self.cancel.store(false, Ordering::SeqCst);
            let catalog = inner.catalog.clone();
            let mut modules: Vec<String> = Vec::new();
            let mut cards = Vec::new();
            for key in keys {
                let Some(card) = inner.cards.get_mut(key) else {
                    continue;
                };
                let Some(module) = card.module.clone() else {
                    continue;
                };
                if card.builtin {
                    continue;
                }
                card.error = None;
                card.progress = None;
                card.phase = Phase::Waiting;
                cards.push((key.clone(), module.clone()));
                if !modules.contains(&module) {
                    modules.push(module);
                }
            }
            // Dependencies first, each module once.
            let mut ordered: Vec<String> = Vec::new();
            fn add(id: &str, catalog: Option<&Catalog>, ordered: &mut Vec<String>, depth: u32) {
                if depth > 8 || ordered.iter().any(|m| m == id) {
                    return;
                }
                if let Some(entry) = catalog.and_then(|c| c.module(id)) {
                    for dep in &entry.requires {
                        add(dep, catalog, ordered, depth + 1);
                    }
                }
                ordered.push(id.to_owned());
            }
            for m in &modules {
                add(m, catalog.as_ref(), &mut ordered, 0);
            }
            let installed: BTreeMap<String, String> = self
                .store
                .all_verified()
                .into_iter()
                .map(|v| (v.manifest.id.clone(), v.manifest.version.clone()))
                .collect();
            // (Full verification here: a damaged install is downloaded again.)
            // Reuse installed modules unless the catalog has a newer version
            // *and* the user explicitly selected the device (no silent updates:
            // updates only happen inside an explicit setup run).
            let mut work: Vec<ModuleEntry> = Vec::new();
            let mut total = 0;
            for id in &ordered {
                let entry = catalog.as_ref().and_then(|c| c.module(id)).cloned();
                match (entry, installed.get(id)) {
                    (Some(e), Some(v))
                        if crate::catalog::version_key(&e.version)
                            <= crate::catalog::version_key(v) => {}
                    (None, Some(_)) => {}
                    (Some(e), _) => {
                        total += e.compatible().map(|a| a.size).unwrap_or(0);
                        work.push(e);
                    }
                    (None, None) => {
                        for (key, m) in &cards {
                            if m == id {
                                if let Some(card) = inner.cards.get_mut(key) {
                                    card.phase = Phase::Failed;
                                    card.error = Some(CardError {
                                        code: "source",
                                        message: "The module catalog is not available. Check the network connection and try again.".into(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            inner.job = Job {
                running: true,
                cancelled: false,
                bytes_done: 0,
                bytes_total: total,
                modules: work.iter().map(|e| e.id.clone()).collect(),
            };
            (work, cards)
        };
        let (work, cards) = plan;
        {
            let mut bytes = self.module_bytes.lock().unwrap_or_else(|e| e.into_inner());
            bytes.clear();
            for e in &work {
                bytes.insert(e.id.clone(), Arc::new(AtomicU64::new(0)));
            }
        }
        let mut failed: BTreeMap<String, CardError> = BTreeMap::new();
        let mut fetcher = Fetcher::new(self.source.clone());
        for entry in &work {
            let set_phase = |phase: Phase| {
                let mut inner = lock(&self.inner);
                for (key, m) in &cards {
                    if m == &entry.id {
                        if let Some(card) = inner.cards.get_mut(key) {
                            if card.phase != Phase::Failed {
                                card.phase = phase;
                            }
                        }
                    }
                }
            };
            let outcome = (|| -> std::result::Result<(), CardError> {
                if self.cancel.load(Ordering::SeqCst) {
                    return Err(CardError {
                        code: "cancelled",
                        message: "Setup was cancelled.".into(),
                    });
                }
                if let Some(missing) = entry.requires.iter().find(|d| failed.contains_key(*d)) {
                    return Err(CardError {
                        code: "invalid_package",
                        message: format!("Required module {missing} could not be set up."),
                    });
                }
                let artifact = entry
                    .compatible()
                    .map_err(|m| CardError {
                        code: "incompatible",
                        message: m,
                    })?
                    .clone();
                set_phase(Phase::Downloading);
                let counter = self
                    .module_bytes
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(&entry.id)
                    .cloned()
                    .unwrap();
                let on_bytes = |n: u64| counter.store(n, Ordering::Relaxed);
                let progress = Progress {
                    cancel: &self.cancel,
                    on_bytes: &on_bytes,
                };
                let path = fetcher
                    .download(&artifact, &self.store.downloads(), &progress)
                    .map_err(|e| CardError {
                        code: classify(&e),
                        message: e.to_string(),
                    })?;
                set_phase(Phase::Verifying);
                Store::check_package(entry, &path).map_err(|e| CardError {
                    code: "invalid_package",
                    message: e.to_string(),
                })?;
                // Installation re-checks, unpacks and verifies every file
                // against module.json and the signed catalog entry.
                set_phase(Phase::Installing);
                self.store.install(entry, &path).map_err(|e| CardError {
                    code: "invalid_package",
                    message: e.to_string(),
                })?;
                let _ = std::fs::remove_file(&path);
                Ok(())
            })();
            if let Err(error) = outcome {
                failed.insert(entry.id.clone(), error);
            }
        }
        let mut inner = lock(&self.inner);
        let deadline = Instant::now() + READY_TIMEOUT;
        for (key, module) in &cards {
            let Some(card) = inner.cards.get_mut(key) else {
                continue;
            };
            if card.phase == Phase::Failed {
                continue;
            }
            if let Some(error) = failed.get(module) {
                card.phase = if error.code == "cancelled" {
                    Phase::Detected
                } else {
                    Phase::Failed
                };
                card.error = Some(error.clone());
                card.progress = None;
                continue;
            }
            card.phase = Phase::Initializing;
            card.progress = None;
            inner
                .persisted
                .configured
                .insert(key.clone(), module.clone());
            inner.persisted.dismissed.remove(key);
            inner.awaiting.insert(key.clone(), deadline);
        }
        inner.job.running = false;
        inner.job.cancelled = self.cancel.load(Ordering::SeqCst);
        let persisted = inner.persisted.clone();
        drop(inner);
        self.save(&persisted);
        self.rescan.store(true, Ordering::SeqCst);
    }
}
