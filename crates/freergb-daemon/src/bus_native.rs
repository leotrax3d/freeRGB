//! Opt-in native API 2 backends. Constructors do not enumerate or open hardware.
//! The protected owner must retain one broker per physical adapter and supply
//! its adapter identity; modules never select paths, drivers or port addresses.
use crate::bus_simulation::Backend;
use freergb_module_api::bus::{Error, Transfer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Instant;

#[cfg(target_os = "linux")]
pub mod linux;
pub mod windows;

/// Implemented only inside the protected helper. A synchronous native operation
/// can block in the kernel; the bounded frontend never waits past its deadline.
/// There is deliberately no retry, recovery write, driver installation or load.
pub trait Device: Send + 'static {
    fn transfer(&mut self, request: &Transfer) -> Result<Vec<u8>, Error>;
}

struct Job {
    request: Transfer,
    deadline: Instant,
    response: mpsc::SyncSender<Result<Vec<u8>, Error>>,
}

/// One serialized worker for one physical bus, shared by all module sessions.
/// A timeout permanently quarantines this instance. An in-flight kernel call
/// cannot be undone: the protected supervisor must keep the physical adapter
/// quarantined until the worker exits; never create a replacement automatically.
pub struct NativeBackend {
    name: &'static str,
    jobs: mpsc::SyncSender<Job>,
    faulted: Arc<AtomicBool>,
}

impl NativeBackend {
    pub fn linux(device: impl Device) -> std::io::Result<Self> {
        Self::spawn("linux-i2c-dev", device)
    }

    pub fn windows(driver: impl windows::DriverInterface) -> std::io::Result<Self> {
        Self::spawn("windows-driver", windows::WindowsDevice::new(driver))
    }

    fn spawn(name: &'static str, mut device: impl Device) -> std::io::Result<Self> {
        let (jobs, receive) = mpsc::sync_channel::<Job>(1);
        let faulted = Arc::new(AtomicBool::new(false));
        let worker_fault = faulted.clone();
        std::thread::Builder::new()
            .name("smbus-native".into())
            .spawn(move || {
                while let Ok(job) = receive.recv() {
                    if worker_fault.load(Ordering::Acquire) || Instant::now() >= job.deadline {
                        let _ = job.response.send(Err(Error::Faulted));
                        break;
                    }
                    let result = device.transfer(&job.request).and_then(|bytes| {
                        let expected = if job.request.op.writes() {
                            0
                        } else {
                            job.request.len()
                        };
                        if bytes.len() == expected {
                            Ok(bytes)
                        } else {
                            Err(Error::Faulted)
                        }
                    });
                    let failed = result.is_err() || Instant::now() >= job.deadline;
                    if failed {
                        worker_fault.store(true, Ordering::Release);
                    }
                    let _ = job.response.send(result);
                    if failed {
                        break;
                    }
                }
            })?;
        Ok(Self {
            name,
            jobs,
            faulted,
        })
    }
}

impl Backend for NativeBackend {
    fn name(&self) -> &'static str {
        self.name
    }

    fn transfer(&mut self, request: &Transfer, deadline: Instant) -> Result<Vec<u8>, Error> {
        if self.faulted.load(Ordering::Acquire) {
            return Err(Error::Faulted);
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        let (response, receive) = mpsc::sync_channel(1);
        self.jobs
            .try_send(Job {
                request: request.clone(),
                deadline,
                response,
            })
            .map_err(|_| Error::Faulted)?;
        let result = receive
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Err(Error::Timeout));
        if result.is_err() || Instant::now() >= deadline {
            self.faulted.store(true, Ordering::Release);
            return Err(result.err().unwrap_or(Error::Timeout));
        }
        result
    }
}

impl Drop for NativeBackend {
    fn drop(&mut self) {
        self.faulted.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests;
