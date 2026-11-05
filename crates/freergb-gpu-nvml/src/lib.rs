//! Minimal safe wrapper over the NVIDIA Management Library fan functions.
//!
//! NVML ships with the NVIDIA driver; it is loaded at runtime from the system
//! directory and never bundled. Setting fan speed needs administrator/root
//! rights. `release` restores the driver's automatic fan policy.
#![allow(unsafe_code)]
use std::ffi::{c_char, c_uint, c_void};

/// One NVML fan. `id` is stable across restarts (`nvml:<GPU UUID>/fan<N>`).
#[derive(Clone, Debug, PartialEq)]
pub struct NvmlFan {
    pub id: String,
    pub gpu_name: String,
    pub index: u32,
    pub duty_percent: Option<f32>,
    pub controllable: bool,
    pub reason: Option<String>,
    pub min_percent: f32,
}

type Device = *mut c_void;
type Ret = c_uint;
const SUCCESS: Ret = 0;
const NO_PERMISSION: Ret = 4;
const NOT_SUPPORTED: Ret = 3;
const MAX_DEVICES: c_uint = 16;
const MAX_FANS: c_uint = 8;

struct Api {
    _lib: libloading::Library,
    shutdown: unsafe extern "C" fn() -> Ret,
    count: unsafe extern "C" fn(*mut c_uint) -> Ret,
    handle: unsafe extern "C" fn(c_uint, *mut Device) -> Ret,
    uuid: unsafe extern "C" fn(Device, *mut c_char, c_uint) -> Ret,
    name: unsafe extern "C" fn(Device, *mut c_char, c_uint) -> Ret,
    num_fans: unsafe extern "C" fn(Device, *mut c_uint) -> Ret,
    speed: unsafe extern "C" fn(Device, c_uint, *mut c_uint) -> Ret,
    min_max: unsafe extern "C" fn(Device, *mut c_uint, *mut c_uint) -> Ret,
    set_speed: unsafe extern "C" fn(Device, c_uint, c_uint) -> Ret,
    set_default: unsafe extern "C" fn(Device, c_uint) -> Ret,
}

pub struct Nvml {
    api: Api,
    /// fan id -> (device handle, fan index); handles live until shutdown.
    fans: Vec<(String, Device, c_uint)>,
}

// NVML is documented thread-safe; handles are opaque driver pointers used
// only through `&mut self`.
unsafe impl Send for Nvml {}

fn library_path() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        // Absolute System32 path: never resolve the DLL via the search path.
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        std::path::PathBuf::from(root)
            .join("System32")
            .join("nvml.dll")
    }
    #[cfg(not(windows))]
    {
        "libnvidia-ml.so.1".into()
    }
}

fn text(buffer: &[c_char]) -> String {
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn check(ret: Ret, what: &str) -> Result<(), String> {
    match ret {
        SUCCESS => Ok(()),
        NO_PERMISSION => Err(format!("{what}: NVML needs administrator/root rights")),
        NOT_SUPPORTED => Err(format!("{what}: not supported by this GPU/driver")),
        other => Err(format!("{what}: NVML error {other}")),
    }
}

impl Nvml {
    pub fn load() -> Result<Self, String> {
        // SAFETY: loading the driver's NVML and resolving documented C
        // symbols with their nvml.h signatures.
        unsafe {
            let lib = libloading::Library::new(library_path()).map_err(|e| e.to_string())?;
            macro_rules! sym {
                ($name:literal) => {
                    *lib.get(concat!($name, "\0").as_bytes())
                        .map_err(|e| format!("{}: {e}", $name))?
                };
            }
            let init: unsafe extern "C" fn() -> Ret = sym!("nvmlInit_v2");
            let api = Api {
                shutdown: sym!("nvmlShutdown"),
                count: sym!("nvmlDeviceGetCount_v2"),
                handle: sym!("nvmlDeviceGetHandleByIndex_v2"),
                uuid: sym!("nvmlDeviceGetUUID"),
                name: sym!("nvmlDeviceGetName"),
                num_fans: sym!("nvmlDeviceGetNumFans"),
                speed: sym!("nvmlDeviceGetFanSpeed_v2"),
                min_max: sym!("nvmlDeviceGetMinMaxFanSpeed"),
                set_speed: sym!("nvmlDeviceSetFanSpeed_v2"),
                set_default: sym!("nvmlDeviceSetDefaultFanSpeed_v2"),
                _lib: lib,
            };
            check(init(), "nvmlInit")?;
            Ok(Self {
                api,
                fans: Vec::new(),
            })
        }
    }

    fn lookup(&self, id: &str) -> Result<(Device, c_uint), String> {
        self.fans
            .iter()
            .find(|(f, _, _)| f == id)
            .map(|(_, d, i)| (*d, *i))
            .ok_or_else(|| "unknown NVIDIA fan".into())
    }
}

impl Drop for Nvml {
    fn drop(&mut self) {
        // SAFETY: matching nvmlInit_v2 from `load`.
        unsafe { (self.api.shutdown)() };
    }
}

impl Nvml {
    pub fn discover(&mut self) -> Vec<NvmlFan> {
        let mut out = Vec::new();
        self.fans.clear();
        let api = &self.api;
        // SAFETY: out-pointers and buffers sized as the NVML API requires.
        unsafe {
            let mut count = 0;
            if (api.count)(&mut count) != SUCCESS {
                return out;
            }
            for d in 0..count.min(MAX_DEVICES) {
                let mut dev: Device = std::ptr::null_mut();
                if (api.handle)(d, &mut dev) != SUCCESS {
                    continue;
                }
                let mut uuid = [0 as c_char; 96];
                let mut name = [0 as c_char; 96];
                if (api.uuid)(dev, uuid.as_mut_ptr(), 96) != SUCCESS {
                    continue;
                }
                let _ = (api.name)(dev, name.as_mut_ptr(), 96);
                let (mut min, mut max) = (0, 100);
                let _ = (api.min_max)(dev, &mut min, &mut max);
                let mut fans = 0;
                let fans_ret = (api.num_fans)(dev, &mut fans);
                for f in 0..fans.min(MAX_FANS) {
                    let mut speed = 0;
                    let read = (api.speed)(dev, f, &mut speed);
                    let id = format!("nvml:{}/fan{f}", text(&uuid));
                    self.fans.push((id.clone(), dev, f));
                    out.push(NvmlFan {
                        id,
                        gpu_name: text(&name),
                        index: f,
                        duty_percent: (read == SUCCESS).then_some(speed as f32),
                        controllable: fans_ret == SUCCESS,
                        reason: None,
                        min_percent: min.min(100) as f32,
                    });
                }
            }
        }
        out
    }

    pub fn take_control(&mut self, id: &str) -> Result<(), String> {
        // NVML switches to manual on the first SetFanSpeed; probe the
        // current speed so permission problems surface before the journal
        // records a fan that was never touched.
        let (dev, fan) = self.lookup(id)?;
        let mut speed = 0;
        // SAFETY: valid handle from discover; out-pointer.
        check(
            unsafe { (self.api.speed)(dev, fan, &mut speed) },
            "read fan",
        )?;
        check(
            unsafe { (self.api.set_speed)(dev, fan, speed.clamp(40, 100)) },
            "take control",
        )
    }

    pub fn set_duty(&mut self, id: &str, percent: f32) -> Result<(), String> {
        let (dev, fan) = self.lookup(id)?;
        let speed = percent.round().clamp(0.0, 100.0) as c_uint;
        // SAFETY: valid handle from discover.
        check(
            unsafe { (self.api.set_speed)(dev, fan, speed) },
            "set fan speed",
        )
    }

    pub fn release(&mut self, id: &str) -> Result<(), String> {
        let (dev, fan) = self.lookup(id)?;
        // SAFETY: valid handle from discover.
        check(
            unsafe { (self.api.set_default)(dev, fan) },
            "restore driver auto",
        )
    }
}
