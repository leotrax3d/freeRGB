//! Drive temperatures (NVMe/SATA) as read-only curve sources, kept apart
//! from the motherboard fan reader.
//!
//! LHM storage enumeration inside the protected reader blocked the first fan
//! snapshot on the owner board (docs/COOLING_ACCEPTANCE.md, 2026-11-05), so
//! drives are read here instead: unprivileged storage-query IOCTLs
//! (`freergb-storage-temp`) on their own thread, every poll under a hard
//! timeout. A hung storage driver only marks these readings stale; it never
//! touches the motherboard provider or any fan path, and at most one query
//! thread is ever outstanding.
use crate::mainboard::SensorEntry;
use freergb_device_api::SensorReading;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use freergb_storage_temp::DriveTemperature;

const PROVIDER: &str = "storage";
/// Readings older than this are reported without a value.
const STALE: Duration = Duration::from_secs(15);

/// Source of drive temperatures (real IOCTLs or a test double).
pub type Reader = Arc<dyn Fn() -> Vec<DriveTemperature> + Send + Sync>;

#[derive(Default)]
struct Cache {
    drives: Vec<DriveTemperature>,
    at: Option<Instant>,
    /// A query is still running past its timeout (driver hang).
    hung: bool,
}

pub struct StorageTemps {
    cache: Arc<Mutex<Cache>>,
    stop: Arc<(Mutex<bool>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}

impl Default for StorageTemps {
    fn default() -> Self {
        Self {
            cache: Arc::default(),
            stop: Arc::new((Mutex::new(false), Condvar::new())),
            worker: None,
        }
    }
}

impl StorageTemps {
    /// Production reader (Windows IOCTLs; empty elsewhere: Linux drives come
    /// from hwmon `nvme`/`drivetemp`).
    pub fn start_default(&mut self) {
        self.start(
            Arc::new(freergb_storage_temp::read_all),
            Duration::from_secs(5),
            Duration::from_secs(3),
        );
    }

    pub fn start(&mut self, reader: Reader, interval: Duration, timeout: Duration) {
        if self.worker.is_some() {
            return;
        }
        let cache = self.cache.clone();
        let stop = self.stop.clone();
        self.worker = std::thread::Builder::new()
            .name("storage-temperatures".into())
            .spawn(move || {
                // One outstanding query at most: a hung one is waited for, never duplicated.
                let mut pending: Option<mpsc::Receiver<Vec<DriveTemperature>>> = None;
                loop {
                    let receiver = pending.take().unwrap_or_else(|| {
                        let (tx, rx) = mpsc::channel();
                        let reader = reader.clone();
                        let _ = std::thread::Builder::new()
                            .name("storage-query".into())
                            .spawn(move || {
                                let _ = tx.send(reader());
                            });
                        rx
                    });
                    match receiver.recv_timeout(timeout) {
                        Ok(drives) => {
                            let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
                            cache.drives = drives;
                            cache.at = Some(Instant::now());
                            cache.hung = false;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            cache.lock().unwrap_or_else(|e| e.into_inner()).hung = true;
                            pending = Some(receiver);
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {}
                    }
                    let guard = stop.0.lock().unwrap_or_else(|e| e.into_inner());
                    let (guard, _) = stop
                        .1
                        .wait_timeout_while(guard, interval, |stop| !*stop)
                        .unwrap_or_else(|e| e.into_inner());
                    if *guard {
                        return;
                    }
                }
            })
            .ok();
    }

    /// True while the last query has not returned within its timeout.
    pub fn hung(&self) -> bool {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).hung
    }

    pub fn sensors(&self) -> Vec<SensorEntry> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let Some(at) = cache.at else {
            return Vec::new();
        };
        let age = at.elapsed();
        cache
            .drives
            .iter()
            .map(|drive| SensorEntry {
                provider: PROVIDER,
                device_id: format!("storage:PhysicalDrive{}", drive.index),
                device_name: drive.model.clone(),
                sensor: SensorReading {
                    id: format!("/storage/{}/temperature/0", drive.index),
                    name: format!("{} temperature", drive.model),
                    unit: "°C".into(),
                    value: (age <= STALE).then_some(drive.celsius),
                    age_ms: age.as_millis().min(u128::from(u64::MAX)) as u64,
                },
            })
            .collect()
    }
}

impl Drop for StorageTemps {
    fn drop(&mut self) {
        *self.stop.0.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.stop.1.notify_all();
        // A hung query thread is detached on purpose; the poller exits promptly.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(celsius: f64) -> DriveTemperature {
        DriveTemperature {
            index: 2,
            model: "Samsung SSD 990 PRO 1TB".into(),
            celsius,
        }
    }

    fn wait(mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !done() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn drive_temperatures_become_sensors() {
        let mut temps = StorageTemps::default();
        temps.start(
            Arc::new(|| vec![drive(45.0)]),
            Duration::from_millis(20),
            Duration::from_millis(200),
        );
        wait(|| !temps.sensors().is_empty());
        let sensors = temps.sensors();
        assert_eq!(sensors.len(), 1);
        assert_eq!(sensors[0].provider, "storage");
        assert_eq!(sensors[0].sensor.id, "/storage/2/temperature/0");
        assert_eq!(sensors[0].sensor.value, Some(45.0));
    }

    #[test]
    fn a_hanging_storage_driver_never_blocks_and_spawns_no_second_query() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mut temps = StorageTemps::default();
        let started = Instant::now();
        temps.start(
            Arc::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(Duration::from_secs(30));
                vec![]
            }),
            Duration::from_millis(10),
            Duration::from_millis(50),
        );
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "start must not wait"
        );
        wait(|| temps.hung());
        assert!(temps.hung());
        // Readers of the cache (the fan/sensor path) answer immediately.
        let read = Instant::now();
        assert!(temps.sensors().is_empty());
        assert!(read.elapsed() < Duration::from_millis(20));
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "one outstanding query at most"
        );
        let shutdown = Instant::now();
        drop(temps);
        assert!(
            shutdown.elapsed() < Duration::from_secs(1),
            "stop never waits for a hung driver"
        );
    }
}
