//! Minimal safe wrapper over AMD ADLX manual fan tuning (Windows).
//!
//! ADLX ships with the AMD Software driver as `System32\amdadlx64.dll`; it is
//! loaded at runtime from that absolute path and never bundled. Interface
//! layouts follow the public ADLX SDK C headers (MIT, GPUOpen): only the
//! vtable prefixes that are called are declared, in header order.
//!
//! Control model: ADLX exposes a fan *curve* (a list of speed/temperature
//! states), not a raw duty. A manual duty is written as a flat curve (every
//! state at the same speed). `take_control` captures the current curve and the
//! zero-RPM state first; `release` restores them. Without a capture (crash
//! recovery in a new process) `release` applies the driver's default curve
//! (`IADLXManualFanTuning1::GetDefaultFanTuningStates`) and, only if that
//! interface is missing, `ResetToFactory` for that GPU's tuning.
#![allow(unsafe_code)]
use std::ffi::{c_char, c_void, CStr};

/// One ADLX fan (one per GPU). `id` is stable across restarts
/// (`adlx:<unique id>/fan0`).
#[derive(Clone, Debug, PartialEq)]
pub struct AdlxFan {
    pub id: String,
    pub gpu_name: String,
    pub index: u32,
    pub duty_percent: Option<f32>,
    pub controllable: bool,
    pub reason: Option<String>,
    /// Lowest speed the driver accepts for manual tuning (percent).
    pub min_percent: f32,
}

type Result_ = i32;
const OK: Result_ = 0;
const ALREADY_ENABLED: Result_ = 1;
const ALREADY_INITIALIZED: Result_ = 2;
const NOT_SUPPORTED: Result_ = 12;
/// ADLX_MAKE_FULL_VER(1, 0, 0, 0): the oldest interface revision we need.
const VERSION: u64 = 1 << 48;
const MAX_GPUS: u32 = 16;

fn ok(r: Result_) -> bool {
    matches!(r, OK | ALREADY_ENABLED | ALREADY_INITIALIZED)
}

fn check(r: Result_, what: &str) -> Result<(), String> {
    if ok(r) {
        Ok(())
    } else if r == NOT_SUPPORTED {
        Err(format!("{what}: not supported by this GPU/driver"))
    } else {
        Err(format!("{what}: ADLX error {r}"))
    }
}

#[repr(C)]
struct Iface<V> {
    vtbl: *const V,
}
type P<V> = *mut Iface<V>;
type Slot = usize;
type Wide = *const u16;

#[repr(C)]
struct SystemVtbl {
    _hybrid: Slot,
    get_gpus: unsafe extern "system" fn(P<SystemVtbl>, *mut P<GpuListVtbl>) -> Result_,
    _query: Slot,
    _displays: Slot,
    _desktops: Slot,
    _gpus_changed: Slot,
    _log: Slot,
    _settings3d: Slot,
    tuning: unsafe extern "system" fn(P<SystemVtbl>, *mut P<TuningVtbl>) -> Result_,
}

#[repr(C)]
struct GpuListVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<GpuListVtbl>) -> i32,
    _query: Slot,
    size: unsafe extern "system" fn(P<GpuListVtbl>) -> u32,
    _empty: Slot,
    _begin: Slot,
    _end: Slot,
    _at: Slot,
    _clear: Slot,
    _remove_back: Slot,
    _add_back: Slot,
    at_gpu: unsafe extern "system" fn(P<GpuListVtbl>, u32, *mut P<GpuVtbl>) -> Result_,
}

#[repr(C)]
struct GpuVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<GpuVtbl>) -> i32,
    _query: Slot,
    _vendor: Slot,
    _asic: Slot,
    _type: Slot,
    _external: Slot,
    name: unsafe extern "system" fn(P<GpuVtbl>, *mut *const c_char) -> Result_,
    _driver_path: Slot,
    _pnp: Slot,
    _desktops: Slot,
    _vram: Slot,
    _vram_type: Slot,
    _bios: Slot,
    _device: Slot,
    _revision: Slot,
    _subsystem: Slot,
    _subsystem_vendor: Slot,
    unique_id: unsafe extern "system" fn(P<GpuVtbl>, *mut i32) -> Result_,
}

#[repr(C)]
struct TuningVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<TuningVtbl>) -> i32,
    _query: Slot,
    _changed: Slot,
    _is_factory: Slot,
    reset_to_factory: unsafe extern "system" fn(P<TuningVtbl>, P<GpuVtbl>) -> Result_,
    _auto: Slot,
    _preset: Slot,
    _gfx: Slot,
    _vram: Slot,
    supports_fan: unsafe extern "system" fn(P<TuningVtbl>, P<GpuVtbl>, *mut u8) -> Result_,
    _power: Slot,
    _get_auto: Slot,
    _get_preset: Slot,
    _get_gfx: Slot,
    _get_vram: Slot,
    get_fan: unsafe extern "system" fn(P<TuningVtbl>, P<GpuVtbl>, *mut P<BaseVtbl>) -> Result_,
}

/// IADLXInterface: Acquire, Release, QueryInterface.
#[repr(C)]
struct BaseVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<BaseVtbl>) -> i32,
    query: unsafe extern "system" fn(P<BaseVtbl>, Wide, *mut *mut c_void) -> Result_,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct IntRange {
    min: i32,
    max: i32,
    step: i32,
}

/// IADLXManualFanTuning (and the prefix of IADLXManualFanTuning1).
#[repr(C)]
struct FanVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<FanVtbl>) -> i32,
    query: unsafe extern "system" fn(P<FanVtbl>, Wide, *mut *mut c_void) -> Result_,
    ranges: unsafe extern "system" fn(P<FanVtbl>, *mut IntRange, *mut IntRange) -> Result_,
    states: unsafe extern "system" fn(P<FanVtbl>, *mut P<StateListVtbl>) -> Result_,
    _empty_states: Slot,
    valid: unsafe extern "system" fn(P<FanVtbl>, P<StateListVtbl>, *mut i32) -> Result_,
    set_states: unsafe extern "system" fn(P<FanVtbl>, P<StateListVtbl>) -> Result_,
    supports_zero_rpm: unsafe extern "system" fn(P<FanVtbl>, *mut u8) -> Result_,
    zero_rpm: unsafe extern "system" fn(P<FanVtbl>, *mut u8) -> Result_,
    set_zero_rpm: unsafe extern "system" fn(P<FanVtbl>, u8) -> Result_,
}

/// IADLXManualFanTuning1: 23 base entries, then GetDefaultFanTuningStates.
#[repr(C)]
struct Fan1Vtbl {
    _base: [Slot; 23],
    default_states: unsafe extern "system" fn(P<Fan1Vtbl>, *mut P<StateListVtbl>) -> Result_,
}

#[repr(C)]
struct StateListVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<StateListVtbl>) -> i32,
    _query: Slot,
    size: unsafe extern "system" fn(P<StateListVtbl>) -> u32,
    _empty: Slot,
    _begin: Slot,
    _end: Slot,
    _at: Slot,
    _clear: Slot,
    _remove_back: Slot,
    _add_back: Slot,
    at_state: unsafe extern "system" fn(P<StateListVtbl>, u32, *mut P<StateVtbl>) -> Result_,
}

#[repr(C)]
struct StateVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<StateVtbl>) -> i32,
    _query: Slot,
    get_speed: unsafe extern "system" fn(P<StateVtbl>, *mut i32) -> Result_,
    set_speed: unsafe extern "system" fn(P<StateVtbl>, i32) -> Result_,
    get_temperature: unsafe extern "system" fn(P<StateVtbl>, *mut i32) -> Result_,
    set_temperature: unsafe extern "system" fn(P<StateVtbl>, i32) -> Result_,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Speed written for a requested duty: never below the driver minimum or
/// above 100 %; the caller's floor (40 %) is applied before this.
pub fn clamp_speed(percent: f32, range_min: i32, range_max: i32) -> i32 {
    let lo = range_min.clamp(0, 100);
    let hi = range_max.clamp(lo, 100);
    (percent.round() as i32).clamp(lo, hi)
}

/// Flat curve for a manual duty: the captured temperatures, every speed equal.
pub fn flat_curve(points: &[(i32, i32)], speed: i32) -> Vec<(i32, i32)> {
    points.iter().map(|&(_, t)| (speed, t)).collect()
}

/// (speed, temperature) points of a fan curve.
type Curve = Vec<(i32, i32)>;
/// Curve and zero-RPM flag captured before control.
type Captured = (Curve, Option<u8>);

struct Fan {
    id: String,
    gpu: P<GpuVtbl>,
    tuning: P<FanVtbl>,
    speed_range: IntRange,
    /// Captured (speed, temperature) states and zero-RPM flag before control.
    captured: Option<Captured>,
}

pub struct Adlx {
    _lib: libloading::Library,
    terminate: unsafe extern "C" fn() -> Result_,
    services: P<TuningVtbl>,
    gpus: Vec<P<GpuVtbl>>,
    fans: Vec<Fan>,
}

// ADLX interfaces are documented as thread-safe; every pointer is used only
// through `&mut self` and released in `Drop`.
unsafe impl Send for Adlx {}

fn library_path() -> std::path::PathBuf {
    // Absolute System32 path: never resolve the DLL via the search path.
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    std::path::PathBuf::from(root)
        .join("System32")
        .join("amdadlx64.dll")
}

impl Adlx {
    pub fn load() -> Result<Self, String> {
        if !cfg!(windows) {
            return Err("ADLX is Windows-only (Linux AMD fans use amdgpu hwmon)".into());
        }
        // SAFETY: loading the driver's ADLX and resolving the documented
        // ADLXInitialize/ADLXTerminate entry points (cdecl, ADLX.h).
        unsafe {
            let lib = libloading::Library::new(library_path()).map_err(|e| e.to_string())?;
            let init: unsafe extern "C" fn(u64, *mut P<SystemVtbl>) -> Result_ = *lib
                .get(b"ADLXInitialize\0")
                .map_err(|e| format!("ADLXInitialize: {e}"))?;
            let terminate: unsafe extern "C" fn() -> Result_ = *lib
                .get(b"ADLXTerminate\0")
                .map_err(|e| format!("ADLXTerminate: {e}"))?;
            let mut system: P<SystemVtbl> = std::ptr::null_mut();
            check(init(VERSION, &mut system), "ADLXInitialize")?;
            if system.is_null() {
                terminate();
                return Err("ADLXInitialize returned no system".into());
            }
            let mut services: P<TuningVtbl> = std::ptr::null_mut();
            let r = ((*(*system).vtbl).tuning)(system, &mut services);
            if !ok(r) || services.is_null() {
                terminate();
                return Err(check(r, "GPU tuning services")
                    .err()
                    .unwrap_or_else(|| "no GPU tuning services".into()));
            }
            let mut list: P<GpuListVtbl> = std::ptr::null_mut();
            let mut gpus = Vec::new();
            if ok(((*(*system).vtbl).get_gpus)(system, &mut list)) && !list.is_null() {
                let n = ((*(*list).vtbl).size)(list).min(MAX_GPUS);
                for i in 0..n {
                    let mut gpu: P<GpuVtbl> = std::ptr::null_mut();
                    if ok(((*(*list).vtbl).at_gpu)(list, i, &mut gpu)) && !gpu.is_null() {
                        gpus.push(gpu);
                    }
                }
                ((*(*list).vtbl).release)(list);
            }
            Ok(Self {
                _lib: lib,
                terminate,
                services,
                gpus,
                fans: Vec::new(),
            })
        }
    }

    pub fn discover(&mut self) -> Vec<AdlxFan> {
        self.drop_fans();
        let mut out = Vec::new();
        for &gpu in &self.gpus {
            // SAFETY: interface pointers acquired in `load`; out-pointers.
            unsafe {
                let mut name: *const c_char = std::ptr::null();
                let gpu_name = if ok(((*(*gpu).vtbl).name)(gpu, &mut name)) && !name.is_null() {
                    CStr::from_ptr(name).to_string_lossy().into_owned()
                } else {
                    "AMD GPU".into()
                };
                let mut unique = 0;
                if !ok(((*(*gpu).vtbl).unique_id)(gpu, &mut unique)) {
                    continue;
                }
                let id = format!("adlx:{unique}/fan0");
                let mut supported = 0u8;
                let sr =
                    ((*(*self.services).vtbl).supports_fan)(self.services, gpu, &mut supported);
                if !ok(sr) || supported == 0 {
                    out.push(AdlxFan {
                        id,
                        gpu_name,
                        index: 0,
                        duty_percent: None,
                        controllable: false,
                        reason: Some(
                            "Manual fan tuning is not supported by this GPU/driver".into(),
                        ),
                        min_percent: 100.0,
                    });
                    continue;
                }
                let mut base: P<BaseVtbl> = std::ptr::null_mut();
                if !ok(((*(*self.services).vtbl).get_fan)(
                    self.services,
                    gpu,
                    &mut base,
                )) || base.is_null()
                {
                    continue;
                }
                let iid = wide("IADLXManualFanTuning");
                let mut tuning: *mut c_void = std::ptr::null_mut();
                let qr = ((*(*base).vtbl).query)(base, iid.as_ptr(), &mut tuning);
                ((*(*base).vtbl).release)(base);
                if !ok(qr) || tuning.is_null() {
                    continue;
                }
                let tuning = tuning as P<FanVtbl>;
                let (mut speed, mut temp) = (IntRange::default(), IntRange::default());
                let _ = ((*(*tuning).vtbl).ranges)(tuning, &mut speed, &mut temp);
                let current = read_states(tuning).ok();
                out.push(AdlxFan {
                    id: id.clone(),
                    gpu_name,
                    index: 0,
                    duty_percent: current
                        .as_ref()
                        .and_then(|s| s.iter().map(|p| p.0).max())
                        .map(|s| s as f32),
                    controllable: current.is_some(),
                    reason: current
                        .is_none()
                        .then(|| "Fan tuning states unreadable".into()),
                    min_percent: speed.min.clamp(0, 100) as f32,
                });
                self.fans.push(Fan {
                    id,
                    gpu,
                    tuning,
                    speed_range: speed,
                    captured: None,
                });
            }
        }
        out
    }

    fn fan(&mut self, id: &str) -> Result<&mut Fan, String> {
        self.fans
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or_else(|| "unknown AMD fan".into())
    }

    pub fn take_control(&mut self, id: &str) -> Result<(), String> {
        let fan = self.fan(id)?;
        // SAFETY: tuning pointer acquired in `discover`.
        unsafe {
            let states = read_states(fan.tuning)?;
            let mut supported = 0u8;
            let mut zero = None;
            if ok(((*(*fan.tuning).vtbl).supports_zero_rpm)(
                fan.tuning,
                &mut supported,
            )) && supported != 0
            {
                let mut set = 0u8;
                if ok(((*(*fan.tuning).vtbl).zero_rpm)(fan.tuning, &mut set)) {
                    zero = Some(set);
                }
            }
            if fan.captured.is_none() {
                fan.captured = Some((states, zero));
            }
            // A held duty must not be overridden by the driver's fan stop.
            if zero == Some(1) {
                check(
                    ((*(*fan.tuning).vtbl).set_zero_rpm)(fan.tuning, 0),
                    "disable zero RPM",
                )?;
            }
        }
        Ok(())
    }

    pub fn set_duty(&mut self, id: &str, percent: f32) -> Result<(), String> {
        let fan = self.fan(id)?;
        let speed = clamp_speed(percent, fan.speed_range.min, fan.speed_range.max);
        // SAFETY: tuning pointer acquired in `discover`.
        unsafe { write_states(fan.tuning, |points| flat_curve(points, speed)) }
    }

    pub fn release(&mut self, id: &str) -> Result<(), String> {
        let services = self.services;
        let fan = self.fan(id)?;
        // SAFETY: interface pointers acquired in `load`/`discover`.
        unsafe {
            if let Some((states, zero)) = fan.captured.take() {
                let restored = write_states(fan.tuning, |_| states.clone());
                if let Some(zero) = zero {
                    let _ = ((*(*fan.tuning).vtbl).set_zero_rpm)(fan.tuning, zero);
                }
                return restored;
            }
            // No capture (crash recovery): the driver's default curve.
            let iid = wide("IADLXManualFanTuning1");
            let mut v1: *mut c_void = std::ptr::null_mut();
            if ok(((*(*fan.tuning).vtbl).query)(
                fan.tuning,
                iid.as_ptr(),
                &mut v1,
            )) && !v1.is_null()
            {
                let v1 = v1 as P<Fan1Vtbl>;
                let mut defaults: P<StateListVtbl> = std::ptr::null_mut();
                let r = ((*(*v1).vtbl).default_states)(v1, &mut defaults);
                let result = if ok(r) && !defaults.is_null() {
                    let applied = check(
                        ((*(*fan.tuning).vtbl).set_states)(fan.tuning, defaults),
                        "restore default fan curve",
                    );
                    ((*(*defaults).vtbl).release)(defaults);
                    applied
                } else {
                    check(r, "read default fan curve")
                };
                // The Fan1 interface shares IADLXInterface's Release slot.
                let base = v1 as P<BaseVtbl>;
                ((*(*base).vtbl).release)(base);
                return result;
            }
            check(
                ((*(*services).vtbl).reset_to_factory)(services, fan.gpu),
                "reset GPU tuning to factory",
            )
        }
    }

    fn drop_fans(&mut self) {
        for fan in self.fans.drain(..) {
            // SAFETY: acquired in `discover`, released exactly once.
            unsafe { ((*(*fan.tuning).vtbl).release)(fan.tuning) };
        }
    }
}

/// Reads (speed, temperature) of every fan tuning state.
unsafe fn read_states(tuning: P<FanVtbl>) -> Result<Vec<(i32, i32)>, String> {
    let mut list: P<StateListVtbl> = std::ptr::null_mut();
    check(
        ((*(*tuning).vtbl).states)(tuning, &mut list),
        "read fan curve",
    )?;
    if list.is_null() {
        return Err("read fan curve: empty".into());
    }
    let mut out = Vec::new();
    let n = ((*(*list).vtbl).size)(list).min(16);
    for i in 0..n {
        let mut state: P<StateVtbl> = std::ptr::null_mut();
        if ok(((*(*list).vtbl).at_state)(list, i, &mut state)) && !state.is_null() {
            let (mut speed, mut temp) = (0, 0);
            let _ = ((*(*state).vtbl).get_speed)(state, &mut speed);
            let _ = ((*(*state).vtbl).get_temperature)(state, &mut temp);
            out.push((speed, temp));
            ((*(*state).vtbl).release)(state);
        }
    }
    ((*(*list).vtbl).release)(list);
    if out.is_empty() {
        return Err("read fan curve: no states".into());
    }
    Ok(out)
}

/// Writes a new curve computed from the current one, validated by the driver.
unsafe fn write_states(
    tuning: P<FanVtbl>,
    next: impl FnOnce(&[(i32, i32)]) -> Vec<(i32, i32)>,
) -> Result<(), String> {
    let current = read_states(tuning)?;
    let wanted = next(&current);
    if wanted.len() != current.len() {
        return Err("fan curve size changed; not written".into());
    }
    let mut list: P<StateListVtbl> = std::ptr::null_mut();
    check(
        ((*(*tuning).vtbl).states)(tuning, &mut list),
        "read fan curve",
    )?;
    if list.is_null() {
        return Err("read fan curve: empty".into());
    }
    let mut result = Ok(());
    for (i, &(speed, temp)) in wanted.iter().enumerate() {
        let mut state: P<StateVtbl> = std::ptr::null_mut();
        if !ok(((*(*list).vtbl).at_state)(list, i as u32, &mut state)) || state.is_null() {
            result = Err("fan curve state unavailable".into());
            break;
        }
        let a = ((*(*state).vtbl).set_speed)(state, speed);
        let b = ((*(*state).vtbl).set_temperature)(state, temp);
        ((*(*state).vtbl).release)(state);
        if let Err(e) = check(a, "set fan speed").and(check(b, "set fan temperature")) {
            result = Err(e);
            break;
        }
    }
    if result.is_ok() {
        let mut bad = -1;
        result = check(
            ((*(*tuning).vtbl).valid)(tuning, list, &mut bad),
            "validate fan curve",
        )
        .and_then(|()| {
            check(
                ((*(*tuning).vtbl).set_states)(tuning, list),
                "apply fan curve",
            )
        });
    }
    ((*(*list).vtbl).release)(list);
    result
}

impl Drop for Adlx {
    fn drop(&mut self) {
        self.drop_fans();
        // SAFETY: acquired in `load`, released once; matching ADLXInitialize.
        unsafe {
            for gpu in self.gpus.drain(..) {
                ((*(*gpu).vtbl).release)(gpu);
            }
            ((*(*self.services).vtbl).release)(self.services);
            (self.terminate)();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duty_respects_the_driver_range() {
        assert_eq!(clamp_speed(55.4, 20, 100), 55);
        assert_eq!(clamp_speed(10.0, 23, 100), 23);
        assert_eq!(clamp_speed(150.0, 0, 100), 100);
        assert_eq!(clamp_speed(70.0, 30, 60), 60);
        assert_eq!(
            clamp_speed(50.0, 120, 90),
            100,
            "bogus ranges stay within 0..100"
        );
    }

    #[test]
    fn flat_curve_keeps_temperatures_and_sets_one_speed() {
        let points = [(25, 40), (40, 55), (60, 70), (80, 85), (100, 95)];
        let flat = flat_curve(&points, 65);
        assert_eq!(flat, vec![(65, 40), (65, 55), (65, 70), (65, 85), (65, 95)]);
    }

    #[test]
    fn version_and_interface_names_match_the_sdk() {
        assert_eq!(VERSION >> 48, 1);
        assert_eq!(wide("IADLXManualFanTuning").last(), Some(&0));
        assert_eq!(
            std::mem::size_of::<Fan1Vtbl>(),
            24 * std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::size_of::<FanVtbl>(),
            11 * std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::size_of::<TuningVtbl>(),
            17 * std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::size_of::<GpuVtbl>(),
            19 * std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn missing_runtime_is_a_clean_error() {
        // CI has no AMD driver: loading must fail without panicking.
        if !library_path().exists() {
            assert!(Adlx::load().is_err());
        }
    }
}
