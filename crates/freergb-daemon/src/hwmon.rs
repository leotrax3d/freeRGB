//! Linux motherboard fan control through the kernel hwmon sysfs ABI
//! (<https://docs.kernel.org/hwmon/sysfs-interface.html>).
//!
//! The kernel's SuperIO/EC drivers (nct6775, nct6683, it87, f71882fg, w83627ehf,
//! asus-ec-sensors, dell-smm, ...) already cover nearly every desktop board, so
//! freeRGB needs no chip-specific code here: any `pwmN` with `pwmN_enable` is a
//! controllable header. Reading is always passive. Writes happen only for
//! channels the user explicitly assigns, and every write path keeps these
//! invariants:
//!
//! * the original `pwmN_enable`/`pwmN` are captured before the first write and
//!   journaled to disk, so a crashed daemon restores them on its next start;
//! * release, daemon shutdown and every fault restore the captured firmware mode;
//! * fans keep a 40% floor unless the global unrestricted switch allows fan stop;
//! * missing/stale temperature input or the curve's critical temperature drive
//!   the header to 100% immediately;
//! * readback after each write; if the driver/BIOS takes the header back
//!   (`pwmN_enable` no longer manual) we re-assert once, then fault and restore;
//! * a stalled tachometer (0 RPM while commanded >= 40%) faults the channel.
use crate::mainboard::{BoardIdentity, Channel, ProviderSummary, SensorEntry};
use freergb_control::{CurveController, FanCurve};
use freergb_device_api::SensorReading;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const PROVIDER: &str = "mainboard.hwmon";
const MAX_CHIPS: usize = 64;
const MAX_ATTRS: usize = 32;
const STALE_MS: u64 = 5000;
const FAN_FLOOR: f32 = freergb_control::fan_support::SAFE_FAN_FLOOR as f32;
/// Consecutive 1 s ticks with 0 RPM at >= floor duty before a stall faults.
const STALL_TICKS: u32 = 6;
const MAX_ASSIGNMENTS: usize = 32;

/// Chips whose drivers usually only report (no `pwmN_enable`), for display.
const READ_ONLY_HINT: &[&str] = &[
    "k10temp",
    "coretemp",
    "nvme",
    "acpitz",
    "amdgpu",
    "drivetemp",
];

/// `pwmN_enable` semantics of the kernel drivers freeRGB supports. Values are
/// only captured and restored, never interpreted beyond `1` (manual); the
/// table drives display, the coverage matrix (scripts/cooling/fan-coverage.mjs
/// parses one entry per line) and the per-driver simulated tests below.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HwmonDriver {
    pub driver: &'static str,
    /// Chip family id in crates/freergb-control/tests/fixtures/chip_families.json.
    pub family: &'static str,
    /// `name` attribute prefixes reported by the driver.
    pub names: &'static [&'static str],
    /// The driver exposes writable `pwmN_enable` (manual = 1).
    pub control: bool,
    /// `pwmN_enable` value meaning "full speed, no control", if any.
    pub full_speed: Option<i64>,
    /// `pwmN_enable` values for chip-automatic modes.
    pub automatic: &'static [i64],
}

/// More specific prefixes first (nct6683 before the generic nct6775 family).
#[rustfmt::skip]
pub const DRIVERS: &[HwmonDriver] = &[
    HwmonDriver { driver: "nct6683", family: "nct668x", names: &["nct6683", "nct6686", "nct6687"], control: false, full_speed: None, automatic: &[] },
    HwmonDriver { driver: "nct6775", family: "nct67xx_separate", names: &["nct6106", "nct6116", "nct6775", "nct6776", "nct6779", "nct6791", "nct6792", "nct6793", "nct6795", "nct6796", "nct6797", "nct6798", "nct6799"], control: true, full_speed: Some(0), automatic: &[2, 3, 4, 5] },
    HwmonDriver { driver: "it87", family: "ite_it87_ext", names: &["it8"], control: true, full_speed: Some(0), automatic: &[2] },
    HwmonDriver { driver: "f71882fg", family: "fintek_f718xx", names: &["f718", "f8"], control: true, full_speed: None, automatic: &[2, 3] },
    HwmonDriver { driver: "w83627ehf", family: "winbond_w836xx", names: &["w83627", "w83667"], control: true, full_speed: None, automatic: &[2, 3, 4, 5] },
    HwmonDriver { driver: "asus-ec-sensors", family: "asus_ec", names: &["asusec"], control: false, full_speed: None, automatic: &[] },
];

/// Driver entry for a hwmon `name`, if freeRGB knows its semantics.
pub fn driver(name: &str) -> Option<&'static HwmonDriver> {
    DRIVERS
        .iter()
        .find(|d| d.names.iter().any(|prefix| name.starts_with(prefix)))
}

fn read_trim(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim();
    (text.len() <= 128 && !text.chars().any(char::is_control)).then(|| text.to_owned())
}
fn read_i64(path: &Path) -> Option<i64> {
    read_trim(path)?.parse().ok()
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Temp {
    pub index: u32,
    pub label: String,
    pub celsius: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pwm {
    pub index: u32,
    pub label: String,
    /// 0..=255 raw duty.
    pub raw: Option<u8>,
    /// Raw `pwmN_enable` (1 = manual, 2+ = chip automatic, 0 = full speed/off).
    pub enable: Option<i64>,
    /// `pwmN_mode`: 0 DC, 1 PWM.
    pub mode: Option<i64>,
    pub writable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fan {
    pub index: u32,
    pub label: String,
    pub rpm: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Chip {
    /// Stable identity: driver name plus device path (hwmonN numbers change).
    pub id: String,
    pub name: String,
    #[serde(skip)]
    pub dir: PathBuf,
    pub temps: Vec<Temp>,
    pub fans: Vec<Fan>,
    pub pwms: Vec<Pwm>,
}

fn indices(dir: &Path, prefix: &str, suffix: &str) -> Vec<u32> {
    let mut found: Vec<u32> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let rest = name.strip_prefix(prefix)?.strip_suffix(suffix)?;
            (!rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
                .then(|| rest.parse().ok())
                .flatten()
        })
        .filter(|i| (1..=MAX_ATTRS as u32).contains(i))
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}

fn writable(path: &Path) -> bool {
    // Opening for write performs no write; it only checks the sysfs permission.
    std::fs::OpenOptions::new().write(true).open(path).is_ok()
}

/// Reads one `hwmonN` directory. `root` is `/sys/class/hwmon` in production.
fn scan_chip(dir: &Path) -> Option<Chip> {
    let name = read_trim(&dir.join("name"))?;
    // `device` is a symlink to the bus device; its canonical path is stable
    // across reboots unlike the hwmonN number.
    let device = std::fs::canonicalize(dir.join("device"))
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| dir.display().to_string());
    let label = |kind: &str, i: u32, fallback: String| {
        read_trim(&dir.join(format!("{kind}{i}_label"))).unwrap_or(fallback)
    };
    let temps = indices(dir, "temp", "_input")
        .into_iter()
        .map(|i| Temp {
            index: i,
            label: label("temp", i, format!("Temperature {i}")),
            celsius: read_i64(&dir.join(format!("temp{i}_input")))
                .map(|m| m as f64 / 1000.0)
                .filter(|c| (-100.0..=200.0).contains(c)),
        })
        .collect();
    let fans = indices(dir, "fan", "_input")
        .into_iter()
        .map(|i| Fan {
            index: i,
            label: label("fan", i, format!("Fan {i}")),
            rpm: read_i64(&dir.join(format!("fan{i}_input")))
                .map(|r| r as f64)
                .filter(|r| (0.0..=100_000.0).contains(r)),
        })
        .collect();
    let pwms = indices(dir, "pwm", "")
        .into_iter()
        .map(|i| {
            let file = dir.join(format!("pwm{i}"));
            let enable_file = dir.join(format!("pwm{i}_enable"));
            Pwm {
                index: i,
                label: label("pwm", i, format!("PWM {i}")),
                raw: read_i64(&file).and_then(|v| u8::try_from(v).ok()),
                enable: read_i64(&enable_file),
                mode: read_i64(&dir.join(format!("pwm{i}_mode"))),
                writable: enable_file.exists() && writable(&file) && writable(&enable_file),
            }
        })
        .collect();
    Some(Chip {
        id: format!("{name}@{device}"),
        name,
        dir: dir.to_owned(),
        temps,
        fans,
        pwms,
    })
}

pub fn scan(root: &Path) -> Vec<Chip> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("hwmon"))
        })
        .collect();
    dirs.sort();
    let mut chips: Vec<Chip> = dirs.iter().filter_map(|d| scan_chip(d)).collect();
    chips.truncate(MAX_CHIPS);
    // Two instances of one driver on one device would collide; keep the first.
    let mut seen = std::collections::BTreeSet::new();
    chips.retain(|c| seen.insert(c.id.clone()));
    chips
}

pub fn board(dmi: &Path) -> Option<BoardIdentity> {
    Some(BoardIdentity {
        manufacturer: read_trim(&dmi.join("board_vendor"))?,
        product: read_trim(&dmi.join("board_name"))?,
        bios_version: read_trim(&dmi.join("bios_version")).unwrap_or_default(),
    })
}

/// `"<chip id>/pwmN"`.
pub fn channel_id(chip: &str, pwm: u32) -> String {
    format!("{chip}/pwm{pwm}")
}
/// `"<chip id>/tempN"`.
pub fn temp_id(chip: &str, temp: u32) -> String {
    format!("{chip}/temp{temp}")
}
fn split_id<'a>(id: &'a str, kind: &str) -> Option<(&'a str, u32)> {
    let (chip, attr) = id.rsplit_once('/')?;
    Some((chip, attr.strip_prefix(kind)?.parse().ok()?))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    /// Fixed duty in percent.
    Manual { percent: f32 },
    /// Curve on a hwmon temperature (`"<chip id>/tempN"`).
    Curve { curve: FanCurve, sensor_id: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub channel_id: String,
    #[serde(flatten)]
    pub mode: Mode,
    /// Pump headers are never reduced below 100% unless unrestricted.
    #[serde(default)]
    pub pump: bool,
    /// Allow fan stop (0%). Honored only while unrestricted control is enabled.
    #[serde(default)]
    pub allow_stop: bool,
}

impl Assignment {
    fn validate(&self) -> Result<(), String> {
        split_id(&self.channel_id, "pwm").ok_or("invalid hwmon channel id")?;
        match &self.mode {
            Mode::Manual { percent } if !(0.0..=100.0).contains(percent) => {
                Err("percent must be 0..100".into())
            }
            Mode::Curve { curve, sensor_id } => {
                curve.validate()?;
                split_id(sensor_id, "temp").ok_or("sensor_id must be a hwmon temperature")?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// Firmware state captured before the first write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {
    pub pwm_path: PathBuf,
    pub enable: i64,
    pub raw: u8,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ChannelStatus {
    pub channel_id: String,
    pub duty_percent: Option<f32>,
    pub rpm: Option<f64>,
    pub floor_percent: f32,
    pub state: &'static str,
    pub message: Option<String>,
}

/// Persisted user configuration (`hwmon-cooling.json`), bound to the board.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub board: Option<BoardIdentity>,
    #[serde(default)]
    pub assignments: Vec<Assignment>,
}

struct Active {
    assignment: Assignment,
    original: Original,
    controller: CurveController,
    stall: u32,
    reasserted: bool,
    /// Manual mode honours its duty only while a fresh temperature and a live
    /// loop vouch for it (`freergb_control::manual_interlock`).
    interlock: freergb_control::manual_interlock::ManualInterlock,
}

fn mono_ms() -> u64 {
    static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

struct Shared {
    chips: Vec<Chip>,
    scanned: Option<Instant>,
    active: BTreeMap<String, Active>,
    status: BTreeMap<String, ChannelStatus>,
    unrestricted: bool,
    paused: bool,
    error: Option<String>,
    /// Headers whose return to firmware is not proven; they stay journaled.
    unresolved: Vec<Original>,
}

pub struct HwmonProvider {
    root: PathBuf,
    dmi: PathBuf,
    state_dir: Option<PathBuf>,
    board: Option<BoardIdentity>,
    shared: Arc<Mutex<Shared>>,
    stop: Arc<(Mutex<bool>, Condvar)>,
    worker: Option<JoinHandle<()>>,
    started: bool,
}

impl Default for HwmonProvider {
    fn default() -> Self {
        Self::new("/sys/class/hwmon".into(), "/sys/class/dmi/id".into(), None)
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn write_attr(path: &Path, value: i64) -> Result<(), String> {
    std::fs::write(path, format!("{value}\n")).map_err(|e| format!("{}: {e}", path.display()))
}
fn enable_path(pwm: &Path) -> PathBuf {
    let mut name = pwm.as_os_str().to_owned();
    name.push("_enable");
    PathBuf::from(name)
}
fn restore(original: &Original) -> Result<(), String> {
    // Raw value first: some drivers only accept pwmN while in manual mode,
    // and a chip returning to automatic takes over the duty anyway.
    let _ = write_attr(&original.pwm_path, i64::from(original.raw));
    write_attr(&enable_path(&original.pwm_path), original.enable)
}
fn percent_to_raw(percent: f32) -> u8 {
    (percent.clamp(0.0, 100.0) * 255.0 / 100.0).round() as u8
}

impl HwmonProvider {
    pub fn new(root: PathBuf, dmi: PathBuf, state_dir: Option<PathBuf>) -> Self {
        Self {
            root,
            dmi,
            state_dir,
            board: None,
            shared: Arc::new(Mutex::new(Shared {
                chips: Vec::new(),
                scanned: None,
                active: BTreeMap::new(),
                status: BTreeMap::new(),
                unrestricted: false,
                paused: false,
                error: None,
                unresolved: Vec::new(),
            })),
            stop: Arc::new((Mutex::new(false), Condvar::new())),
            worker: None,
            started: false,
        }
    }

    fn journal_path(&self) -> Option<PathBuf> {
        self.state_dir
            .as_ref()
            .map(|d| d.join("hwmon-control-journal.json"))
    }
    fn config_path(&self) -> Option<PathBuf> {
        self.state_dir
            .as_ref()
            .map(|d| d.join("hwmon-cooling.json"))
    }

    /// Restores channels a crashed daemon left in manual mode, scans, resumes
    /// saved assignments for the same board and starts the 1 s control loop.
    pub fn start(&mut self, hardware_allowed: bool) -> Result<(), String> {
        if self.started {
            return Err("hwmon provider already started".into());
        }
        if !hardware_allowed {
            return Err("--no-hardware blocks the hwmon provider".into());
        }
        if !self.root.is_dir() {
            return Err(format!("{} is not available", self.root.display()));
        }
        if let Some(journal) = self.journal_path() {
            // An unreadable journal or an I/O fault blocks control entirely.
            let unresolved = recover_journal(&journal)?;
            lock(&self.shared).unresolved = unresolved;
        }
        self.started = true;
        self.board = board(&self.dmi);
        lock(&self.shared).chips = scan(&self.root);
        lock(&self.shared).scanned = Some(Instant::now());
        if let Some(config) = self.load_config() {
            if config.board == self.board {
                for assignment in config.assignments {
                    if let Err(error) = self.assign(assignment.clone()) {
                        tracing::warn!(channel = %assignment.channel_id, %error, "saved hwmon assignment not resumed");
                    }
                }
            }
        }
        let (shared, stop, root, journal) = (
            self.shared.clone(),
            self.stop.clone(),
            self.root.clone(),
            self.journal_path(),
        );
        self.worker = Some(
            std::thread::Builder::new()
                .name("hwmon-cooling".into())
                .spawn(move || loop {
                    {
                        let guard = lock(&stop.0);
                        let (guard, _) = stop
                            .1
                            .wait_timeout_while(guard, Duration::from_secs(1), |s| !*s)
                            .unwrap_or_else(|e| e.into_inner());
                        if *guard {
                            return;
                        }
                    }
                    let chips = scan(&root);
                    let mut shared = lock(&shared);
                    shared.chips = chips;
                    shared.scanned = Some(Instant::now());
                    if tick(&mut shared) {
                        let _ = write_journal(journal.as_deref(), &shared);
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn set_unrestricted(&self, enabled: bool) {
        lock(&self.shared).unrestricted = enabled;
    }

    fn load_config(&self) -> Option<Config> {
        let bytes = std::fs::read(self.config_path()?).ok()?;
        if bytes.len() > 256 * 1024 {
            return None;
        }
        serde_json::from_slice::<Config>(&bytes)
            .ok()
            .filter(|c| c.version == 1 && c.assignments.len() <= MAX_ASSIGNMENTS)
    }

    fn save_config(&self) -> Result<(), String> {
        let Some(path) = self.config_path() else {
            return Ok(());
        };
        let config = Config {
            version: 1,
            board: self.board.clone(),
            assignments: lock(&self.shared)
                .active
                .values()
                .map(|a| a.assignment.clone())
                .collect(),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(&config).unwrap_or_default(),
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temporary, path).map_err(|e| e.to_string())
    }

    /// Takes (or re-targets) one header. The first assignment captures and
    /// journals the firmware state before switching to manual mode.
    pub fn assign(&self, assignment: Assignment) -> Result<(), String> {
        assignment.validate()?;
        let (chip_id, index) = split_id(&assignment.channel_id, "pwm").unwrap();
        let mut shared = lock(&self.shared);
        if shared.paused {
            return Err("Fan control is paused".into());
        }
        if let Some(active) = shared.active.get_mut(&assignment.channel_id) {
            active.assignment = assignment;
            drop(shared);
            return self.save_config();
        }
        if shared.active.len() >= MAX_ASSIGNMENTS {
            return Err("too many controlled hwmon channels".into());
        }
        let chip = shared
            .chips
            .iter()
            .find(|c| c.id == chip_id)
            .ok_or("hwmon chip not found (rescan or check the kernel driver)")?;
        let pwm = chip
            .pwms
            .iter()
            .find(|p| p.index == index)
            .ok_or("hwmon channel not found")?;
        if !pwm.writable {
            return Err("No write permission for this header; install packaging/linux/60-freergb.rules or run the daemon as root".into());
        }
        if pwm.enable == Some(1) {
            // Manual mode before we touched it: fancontrol, CoolerControl or
            // another tool owns this header. Never take it over silently.
            return Err("Header is already in manual mode (another fan tool controls it)".into());
        }
        let original = Original {
            pwm_path: chip.dir.join(format!("pwm{index}")),
            enable: pwm.enable.ok_or("pwm_enable unreadable")?,
            raw: pwm.raw.ok_or("pwm value unreadable")?,
        };
        let id = assignment.channel_id.clone();
        if shared
            .unresolved
            .iter()
            .any(|u| u.pwm_path == original.pwm_path)
        {
            // Retry the proven return first; refuse while it still fails.
            let pending = std::mem::take(&mut shared.unresolved);
            shared.unresolved = pending
                .into_iter()
                .filter(|u| restore(u).is_err())
                .collect();
            let _ = write_journal(self.journal_path().as_deref(), &shared);
            if shared
                .unresolved
                .iter()
                .any(|u| u.pwm_path == original.pwm_path)
            {
                return Err("Previous return to firmware is unresolved; retry later".into());
            }
        }
        shared.active.insert(
            id.clone(),
            Active {
                assignment,
                original: original.clone(),
                controller: CurveController::default(),
                stall: 0,
                reasserted: false,
                interlock: Default::default(),
            },
        );
        // Journal durably before the first write: a crash between the two then
        // restores a header that was never changed, which is harmless. No
        // journal, no write.
        if let Err(error) = write_journal(self.journal_path().as_deref(), &shared) {
            shared.active.remove(&id);
            return Err(error);
        }
        let result = write_attr(&enable_path(&original.pwm_path), 1)
            .and_then(|()| write_attr(&original.pwm_path, 255));
        if let Err(error) = result {
            shared.active.remove(&id);
            if restore(&original).is_err() {
                shared.unresolved.push(original.clone());
            }
            let _ = write_journal(self.journal_path().as_deref(), &shared);
            return Err(error);
        }
        mark_manual(&mut shared, &id);
        tick(&mut shared);
        drop(shared);
        self.save_config()
    }

    /// Returns one header (or all with `None`) to the captured firmware mode.
    pub fn release(&self, channel_id: Option<&str>) -> Result<usize, String> {
        let mut shared = lock(&self.shared);
        let ids: Vec<String> = match channel_id {
            Some(id) if shared.active.contains_key(id) => vec![id.to_owned()],
            Some(_) => Vec::new(),
            None => shared.active.keys().cloned().collect(),
        };
        let mut errors = Vec::new();
        for id in &ids {
            if let Some(active) = shared.active.remove(id) {
                if let Err(e) = restore(&active.original) {
                    shared.unresolved.push(active.original.clone());
                    errors.push(e);
                }
                shared.status.remove(id);
            }
        }
        if let Err(e) = write_journal(self.journal_path().as_deref(), &shared) {
            errors.push(e);
        }
        drop(shared);
        self.save_config()?;
        if errors.is_empty() {
            Ok(ids.len())
        } else {
            Err(errors.join("; "))
        }
    }

    /// Pause returns every header to firmware without forgetting assignments.
    pub fn set_paused(&self, paused: bool) -> Result<bool, String> {
        let mut shared = lock(&self.shared);
        if paused == shared.paused {
            return Ok(paused);
        }
        shared.paused = paused;
        let mut errors = Vec::new();
        let ids: Vec<String> = shared.active.keys().cloned().collect();
        for id in ids {
            let active = shared.active.get_mut(&id).unwrap();
            let result = if paused {
                restore(&active.original)
            } else {
                active.controller = CurveController::default();
                active.reasserted = false;
                active.stall = 0;
                write_attr(&enable_path(&active.original.pwm_path), 1)
            };
            match result {
                Ok(()) if !paused => mark_manual(&mut shared, &id),
                Ok(()) => {}
                Err(e) => errors.push(e),
            }
        }
        if !paused {
            tick(&mut shared);
        }
        if errors.is_empty() {
            Ok(paused)
        } else {
            Err(errors.join("; "))
        }
    }

    pub fn summary(&self) -> ProviderSummary {
        let shared = lock(&self.shared);
        let age_ms = shared.scanned.map(|at| at.elapsed().as_millis() as u64);
        let controllable = shared
            .chips
            .iter()
            .flat_map(|c| &c.pwms)
            .filter(|p| p.enable.is_some())
            .count();
        let (status, message) = if !self.started {
            (
                "disabled",
                "Linux hwmon provider is not running.".to_owned(),
            )
        } else if let Some(error) = &shared.error {
            ("faulted", error.clone())
        } else if shared.chips.is_empty() {
            (
                "no_supported_hardware",
                "No hwmon chips found. Load the SuperIO driver (e.g. nct6775, it87, nct6683)."
                    .to_owned(),
            )
        } else if controllable == 0 {
            ("ready", format!("{} hwmon chips, no controllable fan headers (load the SuperIO driver, e.g. nct6775 or it87).", shared.chips.len()))
        } else {
            (
                "ready",
                format!(
                    "{} hwmon chips, {controllable} fan headers.",
                    shared.chips.len()
                ),
            )
        };
        ProviderSummary {
            id: PROVIDER,
            name: "Linux hwmon motherboard",
            status: status.into(),
            message,
            read_only: controllable == 0,
            control_enabled: self.started && controllable > 0,
            rgb_supported: false,
            age_ms,
            board: self.board.clone(),
        }
    }

    pub fn sensors(&self) -> Vec<SensorEntry> {
        let shared = lock(&self.shared);
        let age = shared
            .scanned
            .map_or(u64::MAX, |at| at.elapsed().as_millis() as u64);
        let fresh = |v: Option<f64>| v.filter(|_| age <= STALE_MS);
        let mut out = Vec::new();
        for chip in &shared.chips {
            let entry = |id: String, name: &str, unit: &str, value: Option<f64>| SensorEntry {
                provider: PROVIDER,
                device_id: format!("{PROVIDER}:{}", chip.id),
                device_name: chip.name.clone(),
                sensor: SensorReading {
                    id,
                    name: name.to_owned(),
                    unit: unit.into(),
                    value: fresh(value),
                    age_ms: age,
                },
            };
            for t in &chip.temps {
                out.push(entry(temp_id(&chip.id, t.index), &t.label, "°C", t.celsius));
            }
            for f in &chip.fans {
                out.push(entry(
                    format!("{}/fan{}", chip.id, f.index),
                    &f.label,
                    "RPM",
                    f.rpm,
                ));
            }
            for p in &chip.pwms {
                let percent = p.raw.map(|r| (f64::from(r) * 100.0 / 255.0).round());
                out.push(entry(channel_id(&chip.id, p.index), &p.label, "%", percent));
            }
        }
        out
    }

    /// Fan and control channels in the shared `cooling.channels.list` shape.
    pub fn channels(&self) -> Vec<Channel> {
        let shared = lock(&self.shared);
        let age = shared
            .scanned
            .map_or(u64::MAX, |at| at.elapsed().as_millis() as u64);
        let mut out = Vec::new();
        for chip in &shared.chips {
            let base =
                |id: String, name: &str, kind: &str, unit: &str, value: Option<f64>| Channel {
                    provider: PROVIDER,
                    id: id.clone(),
                    device_id: format!("{PROVIDER}:{}", chip.id),
                    name: name.to_owned(),
                    kind: kind.into(),
                    reading: SensorReading {
                        id: id.clone(),
                        name: name.to_owned(),
                        unit: unit.into(),
                        value: value.filter(|_| age <= STALE_MS),
                        age_ms: age,
                    },
                    control_available: false,
                    writable: false,
                    physical_mapping: "unconfirmed",
                    electrical_mode: "unknown",
                    sensor_id: id,
                    hardware_name: chip.name.clone(),
                    support: None,
                };
            for f in &chip.fans {
                out.push(base(
                    format!("{}/fan{}", chip.id, f.index),
                    &f.label,
                    "fan",
                    "RPM",
                    f.rpm,
                ));
            }
            for p in &chip.pwms {
                let mut channel = base(
                    channel_id(&chip.id, p.index),
                    &p.label,
                    "control",
                    "%",
                    p.raw.map(|r| (f64::from(r) * 100.0 / 255.0).round()),
                );
                channel.control_available = p.enable.is_some();
                channel.writable = p.writable;
                channel.electrical_mode = match p.mode {
                    Some(0) => "dc",
                    Some(1) => "pwm",
                    _ => "unknown",
                };
                out.push(channel);
            }
        }
        out
    }

    /// `cooling.hwmon.get`: chips, assignments and live status.
    pub fn view(&self) -> Value {
        let shared = lock(&self.shared);
        json!({
            "started": self.started,
            "board": self.board,
            "paused": shared.paused,
            "unrestricted": shared.unrestricted,
            "floor_percent": FAN_FLOOR,
            "chips": shared.chips.iter().map(|c| json!({
                "id": c.id,
                "name": c.name,
                "read_only_hint": READ_ONLY_HINT.contains(&c.name.as_str()),
                "driver": driver(&c.name),
                "temps": c.temps,
                "fans": c.fans,
                "pwms": c.pwms.iter().map(|p| json!({
                    "channel_id": channel_id(&c.id, p.index),
                    "label": p.label,
                    "percent": p.raw.map(|r| (f64::from(r) * 100.0 / 255.0).round()),
                    "enable": p.enable,
                    "mode": p.mode,
                    "writable": p.writable,
                    "paired_fan": c.fans.iter().find(|f| f.index == p.index).map(|f| f.index),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "assignments": shared.active.values().map(|a| &a.assignment).collect::<Vec<_>>(),
            "status": shared.status.values().collect::<Vec<_>>(),
        })
    }
}

/// Updates the cached scan after our own switch to manual mode, so the next
/// tick does not mistake the stale cache for a firmware takeback.
fn mark_manual(shared: &mut Shared, id: &str) {
    if let Some((chip, index)) = split_id(id, "pwm") {
        if let Some(pwm) = shared
            .chips
            .iter_mut()
            .filter(|c| c.id == chip)
            .flat_map(|c| c.pwms.iter_mut())
            .find(|p| p.index == index)
        {
            pwm.enable = Some(1);
        }
    }
}

fn find_temp(chips: &[Chip], sensor_id: &str) -> Option<f32> {
    let (chip, index) = split_id(sensor_id, "temp")?;
    chips
        .iter()
        .find(|c| c.id == chip)?
        .temps
        .iter()
        .find(|t| t.index == index)?
        .celsius
        .map(|c| c as f32)
}

/// Inputs for one channel, read before the mutable control step.
struct Inputs {
    present: bool,
    enable: Option<i64>,
    rpm: Option<f64>,
    temperature: Option<f32>,
}

fn fault(shared: &mut Shared, id: &str, message: String) {
    if let Some(active) = shared.active.remove(id) {
        let restored = restore(&active.original);
        if restored.is_err() {
            shared.unresolved.push(active.original.clone());
        }
        tracing::warn!(channel = id, %message, ?restored, "hwmon channel returned to firmware");
        let message = match restored {
            Ok(()) => format!("{message}; returned to firmware control"),
            Err(e) => format!("{message}; firmware restore failed: {e}"),
        };
        shared.status.insert(
            id.to_owned(),
            ChannelStatus {
                channel_id: id.to_owned(),
                state: "faulted",
                message: Some(message),
                ..Default::default()
            },
        );
    }
}

/// One control step for every active channel. Returns whether the active set
/// changed (journal must be rewritten).
fn tick(shared: &mut Shared) -> bool {
    if shared.paused {
        return false;
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let fresh = shared
        .scanned
        .is_some_and(|at| at.elapsed() <= Duration::from_millis(3000));
    let unrestricted = shared.unrestricted;
    let mono = mono_ms();
    let hottest = fresh
        .then(|| {
            shared
                .chips
                .iter()
                .flat_map(|c| &c.temps)
                .filter_map(|t| t.celsius)
                .map(|c| c as f32)
                .fold(None, |m: Option<f32>, c| Some(m.map_or(c, |m| m.max(c))))
        })
        .flatten();
    let inputs: Vec<(String, Inputs)> = shared
        .active
        .iter()
        .map(|(id, active)| {
            let (chip_id, index) = split_id(id, "pwm").unwrap();
            let chip = shared.chips.iter().find(|c| c.id == chip_id);
            let pwm = chip.and_then(|c| c.pwms.iter().find(|p| p.index == index));
            let temperature = match &active.assignment.mode {
                Mode::Curve { sensor_id, .. } if fresh => find_temp(&shared.chips, sensor_id),
                _ => None,
            };
            (
                id.clone(),
                Inputs {
                    present: pwm.is_some(),
                    enable: pwm.and_then(|p| p.enable),
                    rpm: chip
                        .and_then(|c| c.fans.iter().find(|f| f.index == index))
                        .and_then(|f| f.rpm),
                    temperature,
                },
            )
        })
        .collect();
    let mut faulted = Vec::new();
    for (id, input) in inputs {
        let active = shared.active.get_mut(&id).unwrap();
        if !input.present {
            faulted.push((id, "hwmon header disappeared".to_owned()));
            continue;
        }
        let assignment = &active.assignment;
        let floor = if assignment.pump && !unrestricted {
            100.0
        } else if unrestricted && assignment.allow_stop {
            0.0
        } else {
            FAN_FLOOR
        };
        let mut interlocked = None;
        let requested = match &assignment.mode {
            Mode::Manual { percent } => {
                // Backend-independent safety: a missing, stale or critical
                // temperature (or a stalled loop) forces 100%.
                active.interlock.observe(mono, hottest);
                let decision = active.interlock.evaluate(*percent, mono);
                interlocked = decision.interlock;
                decision.duty_percent
            }
            Mode::Curve { curve, .. } => {
                active
                    .controller
                    .update_at(curve, input.temperature, 0, now_ms)
            }
        };
        let duty = requested.clamp(floor, 100.0);
        let raw = percent_to_raw(duty);
        // The driver or BIOS took the header back from manual mode.
        if input.enable != Some(1) {
            if active.reasserted {
                faulted.push((id, "firmware took the header back from manual mode".into()));
                continue;
            }
            active.reasserted = true;
            if let Err(e) = write_attr(&enable_path(&active.original.pwm_path), 1) {
                faulted.push((id, e));
                continue;
            }
        }
        // Written every tick: some firmware silently overwrites the duty.
        {
            if let Err(e) = write_attr(&active.original.pwm_path, i64::from(raw)) {
                faulted.push((id, e));
                continue;
            }
        }
        let mut message = None;
        // Readback: some chips quantize, so only a large gap counts.
        match std::fs::read_to_string(&active.original.pwm_path)
            .ok()
            .and_then(|s| s.trim().parse::<i64>().ok())
        {
            Some(read) if (read - i64::from(raw)).abs() <= 16 => {}
            Some(read) => message = Some(format!("readback {read} differs from written {raw}")),
            None => {
                faulted.push((id, "pwm readback failed".into()));
                continue;
            }
        }
        // Tachometer stall: only meaningful while a paired tach exists.
        if duty >= FAN_FLOOR
            && input.rpm == Some(0.0)
            && !(unrestricted && active.assignment.allow_stop)
        {
            active.stall += 1;
            if active.stall >= STALL_TICKS {
                faulted.push((
                    id,
                    "fan reports 0 RPM while driven (stalled or not connected)".into(),
                ));
                continue;
            }
        } else {
            active.stall = 0;
        }
        let state = if let Some(reason) = interlocked {
            message.get_or_insert_with(|| format!("manual interlock ({reason:?}): 100%"));
            "fallback_full"
        } else if input.temperature.is_none()
            && matches!(active.assignment.mode, Mode::Curve { .. })
        {
            message.get_or_insert_with(|| "temperature input unavailable: 100%".into());
            "fallback_full"
        } else {
            "running"
        };
        shared.status.insert(
            id.clone(),
            ChannelStatus {
                channel_id: id,
                duty_percent: Some(duty),
                rpm: input.rpm,
                floor_percent: floor,
                state,
                message,
            },
        );
    }
    let changed = !faulted.is_empty();
    for (id, message) in faulted {
        fault(shared, &id, message);
    }
    changed
}

/// Persists active + unresolved headers; an empty set removes the file. The
/// error is returned so callers refuse hardware writes without a durable journal.
fn write_journal(path: Option<&Path>, shared: &Shared) -> Result<(), String> {
    let originals: Vec<&Original> = shared
        .active
        .values()
        .map(|a| &a.original)
        .chain(shared.unresolved.iter())
        .collect();
    let Some(path) = path else {
        return if originals.is_empty() {
            Ok(())
        } else {
            Err("no state directory: durable hwmon recovery journal unavailable".into())
        };
    };
    let result = if originals.is_empty() {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    } else {
        serde_json::to_vec(&originals)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                crate::gpu_cooling::durable_write(path, &bytes).map_err(|e| e.to_string())
            })
    };
    result.map_err(|error| {
        tracing::error!(%error, "hwmon control journal not durable");
        format!("hwmon recovery journal not durable: {error}")
    })
}

/// Restores headers a previous (crashed) daemon left in manual mode. Headers
/// that cannot be restored stay journaled and are returned as unresolved. An
/// unreadable journal is kept as `.corrupt` and blocks the provider.
fn recover_journal(path: &Path) -> Result<Vec<Original>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("hwmon recovery journal unreadable: {e}")),
    };
    let originals = match serde_json::from_slice::<Vec<Original>>(&bytes) {
        Ok(originals) if originals.len() <= 4 * MAX_ASSIGNMENTS => originals,
        _ => {
            let mut kept = path.to_path_buf();
            kept.set_extension("json.corrupt");
            let _ = std::fs::rename(path, &kept);
            return Err(format!(
                "hwmon recovery journal unreadable (kept as {})",
                kept.display()
            ));
        }
    };
    let mut unresolved = Vec::new();
    for original in originals {
        // Only sysfs pwm attributes are ever written from the journal.
        let valid = original.pwm_path.starts_with("/sys/")
            || cfg!(test)
            || std::env::var_os("FREERGB_HWMON_ROOT").is_some();
        if !valid {
            unresolved.push(original);
            continue;
        }
        if let Err(error) = restore(&original) {
            tracing::warn!(%error, "hwmon crash recovery failed; entry stays journaled");
            unresolved.push(original);
        }
    }
    if unresolved.is_empty() {
        tracing::warn!("restored hwmon headers after unclean shutdown");
        std::fs::remove_file(path)
            .map_err(|e| format!("hwmon recovery journal not cleared: {e}"))?;
    } else {
        let refs: Vec<&Original> = unresolved.iter().collect();
        let bytes = serde_json::to_vec(&refs).map_err(|e| e.to_string())?;
        crate::gpu_cooling::durable_write(path, &bytes)
            .map_err(|e| format!("hwmon recovery journal not durable: {e}"))?;
    }
    Ok(unresolved)
}

impl Drop for HwmonProvider {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl HwmonProvider {
    /// Stops the loop and returns every header to firmware; saved assignments
    /// are kept so they resume with the next daemon start.
    pub fn shutdown(&mut self) {
        *lock(&self.stop.0) = true;
        self.stop.1.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let mut shared = lock(&self.shared);
        let originals: Vec<Original> = std::mem::take(&mut shared.active)
            .into_values()
            .map(|a| a.original)
            .collect();
        for original in &originals {
            if let Err(error) = restore(original) {
                shared.unresolved.push(original.clone());
                tracing::error!(%error, "hwmon header not restored on shutdown");
            }
        }
        // Unresolved headers stay in the journal for the next start.
        let _ = write_journal(self.journal_path().as_deref(), &shared);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
    }
    impl Fixture {
        fn new(tag: &str) -> Self {
            Self::with_chip(
                tag,
                &[
                    ("name", "nct6798"),
                    ("temp1_input", "45000"),
                    ("temp1_label", "SYSTIN"),
                    ("temp2_input", "61000"),
                    ("fan1_input", "900"),
                    ("fan2_input", "0"),
                    ("pwm1", "128"),
                    ("pwm1_enable", "5"),
                    ("pwm1_mode", "1"),
                    ("pwm2", "77"),
                    ("pwm2_enable", "2"),
                    ("pwm3", "255"),
                    ("pwm3_enable", "1"),
                ],
            )
        }
        fn with_chip(tag: &str, files: &[(&str, &str)]) -> Self {
            let root =
                std::env::temp_dir().join(format!("freergb-hwmon-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            let chip = root.join("class/hwmon0");
            std::fs::create_dir_all(&chip).unwrap();
            for (name, value) in files {
                std::fs::write(chip.join(name), format!("{value}\n")).unwrap();
            }
            let dmi = root.join("dmi");
            std::fs::create_dir_all(&dmi).unwrap();
            for (name, value) in [
                ("board_vendor", "ASUSTeK"),
                ("board_name", "TUF B650"),
                ("bios_version", "1.2"),
            ] {
                std::fs::write(dmi.join(name), value).unwrap();
            }
            Self { root }
        }
        fn provider(&self) -> HwmonProvider {
            HwmonProvider::new(
                self.root.join("class"),
                self.root.join("dmi"),
                Some(self.root.join("state")),
            )
        }
        fn read(&self, name: &str) -> String {
            std::fs::read_to_string(self.root.join("class/hwmon0").join(name))
                .unwrap()
                .trim()
                .to_owned()
        }
        fn set(&self, name: &str, value: &str) {
            std::fs::write(self.root.join("class/hwmon0").join(name), value).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn chip_id(p: &HwmonProvider) -> String {
        lock(&p.shared).chips[0].id.clone()
    }

    #[test]
    fn scans_temperatures_fans_and_headers_with_board_identity() {
        let f = Fixture::new("scan");
        let mut p = f.provider();
        p.start(true).unwrap();
        let view = p.view();
        assert_eq!(view["board"]["product"], "TUF B650");
        let chip = &view["chips"][0];
        assert_eq!(chip["name"], "nct6798");
        assert_eq!(chip["temps"][0]["label"], "SYSTIN");
        assert_eq!(chip["temps"][0]["celsius"], 45.0);
        assert_eq!(chip["pwms"].as_array().unwrap().len(), 3);
        assert_eq!(chip["pwms"][0]["percent"], 50.0);
        let channels = p.channels();
        let control = channels.iter().find(|c| c.id.ends_with("/pwm1")).unwrap();
        assert!(control.control_available && control.writable);
        assert_eq!(control.electrical_mode, "pwm");
        assert_eq!(p.summary().status, "ready");
        assert!(p.summary().control_enabled);
        assert!(p
            .sensors()
            .iter()
            .any(|s| s.sensor.unit == "°C" && s.sensor.value == Some(61.0)));
    }

    #[test]
    fn manual_assignment_enforces_floor_and_release_restores_firmware() {
        let f = Fixture::new("manual");
        let mut p = f.provider();
        p.start(true).unwrap();
        let id = channel_id(&chip_id(&p), 1);
        p.assign(Assignment {
            channel_id: id.clone(),
            mode: Mode::Manual { percent: 10.0 },
            pump: false,
            allow_stop: true,
        })
        .unwrap();
        assert_eq!(f.read("pwm1_enable"), "1");
        // 10% requested, fan stop not unlocked: 40% floor = raw 102.
        assert_eq!(f.read("pwm1"), "102");
        assert!(f.root.join("state/hwmon-control-journal.json").exists());
        p.set_unrestricted(true);
        tick(&mut lock(&p.shared));
        assert_eq!(f.read("pwm1"), "26");
        assert_eq!(p.release(Some(&id)).unwrap(), 1);
        assert_eq!(
            (f.read("pwm1_enable").as_str(), f.read("pwm1").as_str()),
            ("5", "128")
        );
        assert!(!f.root.join("state/hwmon-control-journal.json").exists());
    }

    #[test]
    fn refuses_headers_owned_by_another_tool_and_pumps_stay_full() {
        let f = Fixture::new("owned");
        let mut p = f.provider();
        p.start(true).unwrap();
        let chip = chip_id(&p);
        let err = p
            .assign(Assignment {
                channel_id: channel_id(&chip, 3),
                mode: Mode::Manual { percent: 50.0 },
                pump: false,
                allow_stop: false,
            })
            .unwrap_err();
        assert!(err.contains("another fan tool"));
        p.assign(Assignment {
            channel_id: channel_id(&chip, 1),
            mode: Mode::Manual { percent: 50.0 },
            pump: true,
            allow_stop: false,
        })
        .unwrap();
        assert_eq!(f.read("pwm1"), "255");
    }

    #[test]
    fn curve_follows_temperature_and_missing_sensor_goes_full() {
        let f = Fixture::new("curve");
        let mut p = f.provider();
        p.start(true).unwrap();
        let chip = chip_id(&p);
        let curve = FanCurve::default(); // 25->30, 50->55, 75->100
        p.assign(Assignment {
            channel_id: channel_id(&chip, 1),
            mode: Mode::Curve {
                curve: curve.clone(),
                sensor_id: temp_id(&chip, 1),
            },
            pump: false,
            allow_stop: false,
        })
        .unwrap();
        // 45 C -> 50% -> raw 128.
        assert_eq!(f.read("pwm1"), "128");
        p.assign(Assignment {
            channel_id: channel_id(&chip, 1),
            mode: Mode::Curve {
                curve,
                sensor_id: temp_id(&chip, 9),
            },
            pump: false,
            allow_stop: false,
        })
        .unwrap();
        tick(&mut lock(&p.shared));
        assert_eq!(f.read("pwm1"), "255");
        assert_eq!(p.view()["status"][0]["state"], "fallback_full");
    }

    #[test]
    fn firmware_takeback_and_stall_fault_return_to_firmware() {
        let f = Fixture::new("fault");
        let mut p = f.provider();
        p.start(true).unwrap();
        let chip = chip_id(&p);
        p.assign(Assignment {
            channel_id: channel_id(&chip, 2),
            mode: Mode::Manual { percent: 60.0 },
            pump: false,
            allow_stop: false,
        })
        .unwrap();
        for _ in 0..STALL_TICKS {
            let chips = scan(&p.root);
            let mut shared = lock(&p.shared);
            shared.chips = chips;
            tick(&mut shared);
        }
        assert_eq!(p.view()["status"][0]["state"], "faulted");
        assert_eq!(
            (f.read("pwm2_enable").as_str(), f.read("pwm2").as_str()),
            ("2", "77")
        );

        p.assign(Assignment {
            channel_id: channel_id(&chip, 1),
            mode: Mode::Manual { percent: 60.0 },
            pump: false,
            allow_stop: false,
        })
        .unwrap();
        for _ in 0..2 {
            f.set("pwm1_enable", "5");
            let chips = scan(&p.root);
            let mut shared = lock(&p.shared);
            shared.chips = chips;
            tick(&mut shared);
        }
        assert!(lock(&p.shared).active.is_empty());
        assert_eq!(f.read("pwm1"), "128");
    }

    fn manual_70(p: &HwmonProvider) -> Assignment {
        Assignment {
            channel_id: channel_id(&chip_id(p), 1),
            mode: Mode::Manual { percent: 70.0 },
            pump: false,
            allow_stop: false,
        }
    }

    #[test]
    fn manual_without_a_fresh_temperature_goes_full() {
        let f = Fixture::new("interlock");
        let mut p = f.provider();
        p.start(true).unwrap();
        let a = manual_70(&p);
        p.assign(a).unwrap();
        assert_eq!(f.read("pwm1"), "179");
        // Every temperature input disappears (driver unload / sensor loss).
        let dir = f.root.join("class/hwmon0");
        for name in ["temp1_input", "temp2_input"] {
            std::fs::remove_file(dir.join(name)).unwrap();
        }
        for _ in 0..12 {
            std::thread::sleep(Duration::from_millis(500));
            if f.read("pwm1") == "255" {
                return;
            }
        }
        panic!(
            "manual duty never reached the safe state: pwm1={} enable={} status={:?}",
            f.read("pwm1"),
            f.read("pwm1_enable"),
            lock(&p.shared).status
        );
    }

    #[test]
    fn unavailable_journal_blocks_first_write() {
        let f = Fixture::new("nojournal");
        let mut p = f.provider();
        p.start(true).unwrap();
        // The state path is a file: the journal cannot be made durable.
        std::fs::remove_dir_all(f.root.join("state")).ok();
        std::fs::write(f.root.join("state"), b"x").unwrap();
        let a = manual_70(&p);
        assert!(p.assign(a).is_err());
        assert_eq!(f.read("pwm1_enable"), "5", "no write without a journal");
        assert_eq!(f.read("pwm1"), "128");
        assert!(lock(&p.shared).active.is_empty());
        // No state directory at all behaves the same.
        let mut p = HwmonProvider::new(f.root.join("class"), f.root.join("dmi"), None);
        p.start(true).unwrap();
        let a = manual_70(&p);
        assert!(p.assign(a).is_err());
        assert_eq!(f.read("pwm1_enable"), "5");
    }

    #[test]
    fn failed_restore_stays_journaled_until_proven() {
        let f = Fixture::new("unresolved");
        let mut p = f.provider();
        p.start(true).unwrap();
        let a = manual_70(&p);
        let id = a.channel_id.clone();
        p.assign(a.clone()).unwrap();
        // Make restore fail: the pwm_enable attribute becomes a directory.
        let enable = f.root.join("class/hwmon0/pwm1_enable");
        let saved = std::fs::read(&enable).unwrap();
        std::fs::remove_file(&enable).unwrap();
        std::fs::create_dir(&enable).unwrap();
        assert!(p.release(Some(&id)).is_err());
        let journal = f.root.join("state/hwmon-control-journal.json");
        assert!(journal.exists(), "unresolved header must stay journaled");
        assert!(
            p.assign(a.clone()).is_err(),
            "unresolved header is not re-acquired"
        );
        std::mem::forget(p); // crash before any proof
        let mut p = f.provider();
        p.start(true).unwrap();
        assert!(
            journal.exists(),
            "restart keeps the entry while restore fails"
        );
        std::mem::forget(p);
        std::fs::remove_dir(&enable).unwrap();
        std::fs::write(&enable, saved).unwrap();
        let mut p = f.provider();
        p.start(true).unwrap();
        assert!(!journal.exists(), "proven return clears the journal");
        assert_eq!(f.read("pwm1_enable"), "5");
    }

    #[test]
    fn unreadable_journal_blocks_provider_and_is_kept() {
        let f = Fixture::new("corrupt");
        std::fs::create_dir_all(f.root.join("state")).unwrap();
        std::fs::write(f.root.join("state/hwmon-control-journal.json"), b"{nope").unwrap();
        let mut p = f.provider();
        assert!(p.start(true).is_err());
        assert!(f
            .root
            .join("state/hwmon-control-journal.json.corrupt")
            .exists());
    }

    #[test]
    fn drop_restores_and_saved_assignments_resume_after_crash_recovery() {
        let f = Fixture::new("resume");
        {
            let mut p = f.provider();
            p.start(true).unwrap();
            let chip = chip_id(&p);
            p.assign(Assignment {
                channel_id: channel_id(&chip, 1),
                mode: Mode::Manual { percent: 70.0 },
                pump: false,
                allow_stop: false,
            })
            .unwrap();
            assert_eq!(f.read("pwm1"), "179");
        }
        assert_eq!(f.read("pwm1_enable"), "5");
        // Simulated crash: manual mode left behind with a journal.
        f.set("pwm1_enable", "1");
        f.set("pwm1", "200");
        let journal = vec![Original {
            pwm_path: f.root.join("class/hwmon0/pwm1"),
            enable: 5,
            raw: 128,
        }];
        std::fs::write(
            f.root.join("state/hwmon-control-journal.json"),
            serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        let mut p = f.provider();
        p.start(true).unwrap();
        // Recovered to firmware, then the saved assignment resumed at 70%.
        assert_eq!(f.read("pwm1"), "179");
        assert_eq!(p.view()["assignments"][0]["percent"], 70.0);
        p.set_paused(true).unwrap();
        assert_eq!(f.read("pwm1_enable"), "5");
        p.set_paused(false).unwrap();
        assert_eq!(f.read("pwm1_enable"), "1");
    }

    /// Simulated sysfs per supported driver: manual takeover, restore of every
    /// firmware `pwmN_enable` value (incl. full speed), takeback fault and
    /// read-only drivers. Writes go to temporary files only.
    #[test]
    fn every_driver_restores_its_own_enable_semantics() {
        for (n, d) in DRIVERS.iter().enumerate() {
            let name = if d.names[0].len() < 5 {
                format!("{}686", d.names[0])
            } else {
                d.names[0].to_owned()
            };
            assert_eq!(driver(&name), Some(d), "{name}");
            let mut originals: Vec<i64> = d.automatic.to_vec();
            originals.extend(d.full_speed);
            let mut files = vec![
                ("name".to_owned(), name.clone()),
                ("temp1_input".to_owned(), "40000".to_owned()),
            ];
            for (i, enable) in originals.iter().enumerate() {
                let i = i + 1;
                files.push((format!("fan{i}_input"), "1200".into()));
                files.push((format!("pwm{i}"), "153".into()));
                files.push((format!("pwm{i}_enable"), enable.to_string()));
            }
            if d.driver == "nct6683" {
                // nct6683 reports duty but exposes no pwm_enable.
                files.push(("fan1_input".into(), "1200".into()));
                files.push(("pwm1".into(), "153".into()));
            }
            let refs: Vec<(&str, &str)> = files
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();
            let f = Fixture::with_chip(&format!("drv{n}"), &refs);
            let mut p = f.provider();
            p.start(true).unwrap();
            assert_eq!(p.view()["chips"][0]["driver"]["family"], d.family);
            let chip = chip_id(&p);
            let controls = p
                .channels()
                .into_iter()
                .filter(|c| c.control_available)
                .count();
            if !d.control {
                assert_eq!(controls, 0, "{}", d.driver);
                assert!(!p.summary().control_enabled, "{}", d.driver);
                if d.driver == "nct6683" {
                    assert!(p
                        .assign(Assignment {
                            channel_id: channel_id(&chip, 1),
                            mode: Mode::Manual { percent: 50.0 },
                            pump: false,
                            allow_stop: false,
                        })
                        .is_err());
                }
                continue;
            }
            assert_eq!(controls, originals.len(), "{}", d.driver);
            for (i, original) in originals.iter().enumerate() {
                let index = i as u32 + 1;
                let id = channel_id(&chip, index);
                p.assign(Assignment {
                    channel_id: id.clone(),
                    mode: Mode::Manual { percent: 50.0 },
                    pump: false,
                    allow_stop: false,
                })
                .unwrap();
                assert_eq!(f.read(&format!("pwm{index}_enable")), "1", "{}", d.driver);
                assert_eq!(f.read(&format!("pwm{index}")), "128");
                p.release(Some(&id)).unwrap();
                assert_eq!(
                    (
                        f.read(&format!("pwm{index}_enable")),
                        f.read(&format!("pwm{index}"))
                    ),
                    (original.to_string(), "153".to_owned()),
                    "{} enable {original}",
                    d.driver
                );
            }
            // The BIOS/driver takes the header back into an automatic mode.
            let chips = scan(&p.root);
            lock(&p.shared).chips = chips;
            p.assign(Assignment {
                channel_id: channel_id(&chip, 1),
                mode: Mode::Manual { percent: 70.0 },
                pump: false,
                allow_stop: false,
            })
            .unwrap();
            for _ in 0..2 {
                f.set("pwm1_enable", &originals[0].to_string());
                let chips = scan(&p.root);
                let mut shared = lock(&p.shared);
                shared.chips = chips;
                tick(&mut shared);
            }
            assert!(lock(&p.shared).active.is_empty(), "{}", d.driver);
            assert_eq!(f.read("pwm1"), "153", "{}", d.driver);
        }
    }

    #[test]
    fn no_hardware_blocks_start() {
        let mut p = HwmonProvider::default();
        assert!(p.start(false).unwrap_err().contains("--no-hardware"));
        assert_eq!(p.summary().status, "disabled");
    }
}
