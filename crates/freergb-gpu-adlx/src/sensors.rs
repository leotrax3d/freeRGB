//! Read-only prefixes from GPUOpen ADLX SDK `IPerformanceMonitoring.h`.
use super::*;

/// One current ADLX GPU sample. Missing values include unsupported metrics,
/// driver errors and implausible values. A caller must timestamp each poll and
/// expire it; this source never reuses a previous successful reading.
#[derive(Clone, Debug, PartialEq)]
pub struct AdlxSensors {
    pub gpu_id: String,
    pub temperature_c: Option<f64>,
    pub hotspot_c: Option<f64>,
    pub fan_rpm: Option<u32>,
}

#[repr(C)]
pub(super) struct PerformanceVtbl {
    _acquire: Slot,
    pub(super) release: unsafe extern "system" fn(P<PerformanceVtbl>) -> i32,
    _query: Slot,
    _before_current_gpu: [Slot; 15],
    current_gpu:
        unsafe extern "system" fn(P<PerformanceVtbl>, P<GpuVtbl>, *mut P<MetricsVtbl>) -> Result_,
    _current_system: Slot,
    _current_fps: Slot,
    supported:
        unsafe extern "system" fn(P<PerformanceVtbl>, P<GpuVtbl>, *mut P<SupportVtbl>) -> Result_,
}

#[repr(C)]
struct SupportVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<SupportVtbl>) -> i32,
    _query: Slot,
    _usage_clock_vram: [Slot; 3],
    temperature: unsafe extern "system" fn(P<SupportVtbl>, *mut u8) -> Result_,
    hotspot: unsafe extern "system" fn(P<SupportVtbl>, *mut u8) -> Result_,
    _power: [Slot; 2],
    fan: unsafe extern "system" fn(P<SupportVtbl>, *mut u8) -> Result_,
}

#[repr(C)]
struct MetricsVtbl {
    _acquire: Slot,
    release: unsafe extern "system" fn(P<MetricsVtbl>) -> i32,
    _query: Slot,
    _timestamp_usage_clock_vram: [Slot; 4],
    temperature: unsafe extern "system" fn(P<MetricsVtbl>, *mut f64) -> Result_,
    hotspot: unsafe extern "system" fn(P<MetricsVtbl>, *mut f64) -> Result_,
    _power: [Slot; 2],
    fan: unsafe extern "system" fn(P<MetricsVtbl>, *mut i32) -> Result_,
}

unsafe fn supported(
    support: P<SupportVtbl>,
    get: unsafe extern "system" fn(P<SupportVtbl>, *mut u8) -> Result_,
) -> bool {
    let mut value = 0;
    ok(get(support, &mut value)) && value != 0
}

unsafe fn temperature(
    metrics: P<MetricsVtbl>,
    get: unsafe extern "system" fn(P<MetricsVtbl>, *mut f64) -> Result_,
) -> Option<f64> {
    let mut value = f64::NAN;
    (ok(get(metrics, &mut value)) && value.is_finite() && (-20.0..=150.0).contains(&value))
        .then_some(value)
}

/// All acquired interfaces are released, including partially returned ones on
/// error. A missing support bit prevents calling its corresponding getter.
pub(super) unsafe fn read(
    service: P<PerformanceVtbl>,
    gpu: P<GpuVtbl>,
    unique: i32,
) -> AdlxSensors {
    let mut out = AdlxSensors {
        gpu_id: format!("adlx:{unique}"),
        temperature_c: None,
        hotspot_c: None,
        fan_rpm: None,
    };
    if service.is_null() {
        return out;
    }
    let mut support: P<SupportVtbl> = std::ptr::null_mut();
    let status = ((*(*service).vtbl).supported)(service, gpu, &mut support);
    if support.is_null() {
        return out;
    }
    if !ok(status) {
        ((*(*support).vtbl).release)(support);
        return out;
    }
    let mut metrics: P<MetricsVtbl> = std::ptr::null_mut();
    let status = ((*(*service).vtbl).current_gpu)(service, gpu, &mut metrics);
    if !metrics.is_null() {
        if ok(status) {
            let s = &*(*support).vtbl;
            let m = &*(*metrics).vtbl;
            if supported(support, s.temperature) {
                out.temperature_c = temperature(metrics, m.temperature);
            }
            if supported(support, s.hotspot) {
                out.hotspot_c = temperature(metrics, m.hotspot);
            }
            if supported(support, s.fan) {
                let mut value = -1;
                if ok((m.fan)(metrics, &mut value)) && (0..=100_000).contains(&value) {
                    out.fan_rpm = Some(value as u32);
                }
            }
        }
        ((*(*metrics).vtbl).release)(metrics);
    }
    ((*(*support).vtbl).release)(support);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    struct Fake<V, T> {
        vtbl: *const V,
        data: T,
        releases: u32,
    }

    struct Values {
        temperature: f64,
        hotspot: f64,
        rpm: i32,
        status: Result_,
        getter_calls: u32,
    }

    struct Service {
        support: Fake<SupportVtbl, bool>,
        metrics: Fake<MetricsVtbl, Values>,
        status: Result_,
        support_status: Result_,
        no_support: bool,
        no_metrics: bool,
    }

    unsafe extern "system" fn release_support(p: P<SupportVtbl>) -> i32 {
        (*(p as *mut Fake<SupportVtbl, bool>)).releases += 1;
        0
    }
    unsafe extern "system" fn release_metrics(p: P<MetricsVtbl>) -> i32 {
        (*(p as *mut Fake<MetricsVtbl, Values>)).releases += 1;
        0
    }
    unsafe extern "system" fn support(p: P<SupportVtbl>, out: *mut u8) -> Result_ {
        *out = (*(p as *mut Fake<SupportVtbl, bool>)).data.into();
        OK
    }
    unsafe extern "system" fn temp(p: P<MetricsVtbl>, out: *mut f64) -> Result_ {
        let data = &mut (*(p as *mut Fake<MetricsVtbl, Values>)).data;
        data.getter_calls += 1;
        *out = data.temperature;
        data.status
    }
    unsafe extern "system" fn hotspot(p: P<MetricsVtbl>, out: *mut f64) -> Result_ {
        let data = &mut (*(p as *mut Fake<MetricsVtbl, Values>)).data;
        data.getter_calls += 1;
        *out = data.hotspot;
        OK
    }
    unsafe extern "system" fn rpm(p: P<MetricsVtbl>, out: *mut i32) -> Result_ {
        let data = &mut (*(p as *mut Fake<MetricsVtbl, Values>)).data;
        data.getter_calls += 1;
        *out = data.rpm;
        OK
    }
    unsafe extern "system" fn get_support(
        p: P<PerformanceVtbl>,
        _: P<GpuVtbl>,
        out: *mut P<SupportVtbl>,
    ) -> Result_ {
        let data = &mut (*(p as *mut Fake<PerformanceVtbl, Service>)).data;
        *out = if data.no_support {
            std::ptr::null_mut()
        } else {
            &mut data.support as *mut _ as P<SupportVtbl>
        };
        data.support_status
    }
    unsafe extern "system" fn get_metrics(
        p: P<PerformanceVtbl>,
        _: P<GpuVtbl>,
        out: *mut P<MetricsVtbl>,
    ) -> Result_ {
        let data = &mut (*(p as *mut Fake<PerformanceVtbl, Service>)).data;
        *out = if data.no_metrics {
            std::ptr::null_mut()
        } else {
            &mut data.metrics as *mut _ as P<MetricsVtbl>
        };
        data.status
    }
    unsafe extern "system" fn release_service(_: P<PerformanceVtbl>) -> i32 {
        0
    }
    static SUPPORT: SupportVtbl = SupportVtbl {
        _acquire: 0,
        release: release_support,
        _query: 0,
        _usage_clock_vram: [0; 3],
        temperature: support,
        hotspot: support,
        _power: [0; 2],
        fan: support,
    };
    static METRICS: MetricsVtbl = MetricsVtbl {
        _acquire: 0,
        release: release_metrics,
        _query: 0,
        _timestamp_usage_clock_vram: [0; 4],
        temperature: temp,
        hotspot,
        _power: [0; 2],
        fan: rpm,
    };
    static SERVICE: PerformanceVtbl = PerformanceVtbl {
        _acquire: 0,
        release: release_service,
        _query: 0,
        _before_current_gpu: [0; 15],
        current_gpu: get_metrics,
        _current_system: 0,
        _current_fps: 0,
        supported: get_support,
    };

    fn fixture() -> Fake<PerformanceVtbl, Service> {
        Fake {
            vtbl: &SERVICE,
            data: Service {
                support: Fake {
                    vtbl: &SUPPORT,
                    data: true,
                    releases: 0,
                },
                metrics: Fake {
                    vtbl: &METRICS,
                    data: Values {
                        temperature: 54.5,
                        hotspot: 67.0,
                        rpm: 0,
                        status: OK,
                        getter_calls: 0,
                    },
                    releases: 0,
                },
                status: OK,
                support_status: OK,
                no_support: false,
                no_metrics: false,
            },
            releases: 0,
        }
    }

    fn poll(fake: &mut Fake<PerformanceVtbl, Service>) -> AdlxSensors {
        // Fake interfaces never load ADLX or dereference the GPU argument.
        unsafe {
            read(
                fake as *mut _ as P<PerformanceVtbl>,
                std::ptr::null_mut(),
                768,
            )
        }
    }

    #[test]
    fn reads_current_metrics_and_releases_interfaces() {
        let mut fake = fixture();
        assert_eq!(
            poll(&mut fake),
            AdlxSensors {
                gpu_id: "adlx:768".into(),
                temperature_c: Some(54.5),
                hotspot_c: Some(67.0),
                fan_rpm: Some(0),
            }
        );
        assert_eq!(fake.data.support.releases, 1);
        assert_eq!(fake.data.metrics.releases, 1);
        fake.data.metrics.data.rpm = 1800;
        assert_eq!(poll(&mut fake).fan_rpm, Some(1800));
    }

    #[test]
    fn failures_and_unsupported_metrics_do_not_reuse_values() {
        let mut fake = fixture();
        assert!(poll(&mut fake).temperature_c.is_some());
        fake.data.metrics.data.status = NOT_SUPPORTED;
        assert_eq!(poll(&mut fake).temperature_c, None);
        assert_eq!(poll(&mut fake).hotspot_c, Some(67.0));
        fake.data.support.data = false;
        let calls = fake.data.metrics.data.getter_calls;
        assert_eq!(poll(&mut fake).fan_rpm, None);
        assert_eq!(fake.data.metrics.data.getter_calls, calls);
        fake.data.support.data = true;
        fake.data.status = NOT_SUPPORTED;
        let missing = poll(&mut fake);
        assert_eq!(missing.temperature_c, None);
        assert_eq!(missing.hotspot_c, None);
        assert_eq!(missing.fan_rpm, None);
        assert_eq!(fake.data.support.releases, 5);
        assert_eq!(fake.data.metrics.releases, 5);
    }

    #[test]
    fn null_outputs_and_failed_support_queries_release_only_acquired_interfaces() {
        let mut fake = fixture();
        fake.data.no_support = true;
        assert_eq!(poll(&mut fake).fan_rpm, None);
        assert_eq!(fake.data.support.releases, 0);
        assert_eq!(fake.data.metrics.releases, 0);
        fake.data.no_support = false;
        fake.data.support_status = NOT_SUPPORTED;
        assert_eq!(poll(&mut fake).temperature_c, None);
        assert_eq!(fake.data.support.releases, 1);
        assert_eq!(fake.data.metrics.releases, 0);
        fake.data.support_status = OK;
        fake.data.no_metrics = true;
        assert_eq!(poll(&mut fake).hotspot_c, None);
        assert_eq!(fake.data.support.releases, 2);
        assert_eq!(fake.data.metrics.releases, 0);
        assert_eq!(fake.data.metrics.data.getter_calls, 0);
    }

    #[test]
    fn invalid_values_are_missing_and_no_service_is_safe() {
        let mut fake = fixture();
        for value in [f64::NAN, f64::INFINITY, -21.0, 151.0] {
            fake.data.metrics.data.temperature = value;
            fake.data.metrics.data.hotspot = value;
            fake.data.metrics.data.rpm = -1;
            let sample = poll(&mut fake);
            assert_eq!(sample.temperature_c, None);
            assert_eq!(sample.hotspot_c, None);
            assert_eq!(sample.fan_rpm, None);
        }
        fake.data.metrics.data.rpm = 100_001;
        assert_eq!(poll(&mut fake).fan_rpm, None);
        let missing = unsafe { read(std::ptr::null_mut(), std::ptr::null_mut(), 9) };
        assert_eq!(missing.gpu_id, "adlx:9");
        assert_eq!(missing.temperature_c, None);
    }

    #[test]
    fn vtable_offsets_follow_the_public_sdk() {
        assert_eq!(
            std::mem::offset_of!(SystemVtbl, performance),
            9 * size_of::<Slot>()
        );
        assert_eq!(
            std::mem::offset_of!(PerformanceVtbl, current_gpu),
            18 * size_of::<Slot>()
        );
        assert_eq!(
            std::mem::offset_of!(PerformanceVtbl, supported),
            21 * size_of::<Slot>()
        );
        assert_eq!(
            std::mem::offset_of!(MetricsVtbl, fan),
            11 * size_of::<Slot>()
        );
        assert_eq!(
            std::mem::offset_of!(SupportVtbl, fan),
            10 * size_of::<Slot>()
        );
    }
}
