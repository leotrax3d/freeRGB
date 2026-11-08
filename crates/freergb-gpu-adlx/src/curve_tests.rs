//! Driver-free tests for the actual curve reader's FFI and error handling.
use super::*;

#[repr(C)]
struct State {
    vtbl: *const StateVtbl,
    speed: i32,
    temperature: i32,
    speed_result: Result_,
    temperature_result: Result_,
    releases: u32,
}

#[repr(C)]
struct List {
    vtbl: *const StateListVtbl,
    state: State,
    count: u32,
    unavailable: bool,
    releases: u32,
}

unsafe extern "system" fn release_state(p: P<StateVtbl>) -> i32 {
    (*(p as *mut State)).releases += 1;
    0
}
unsafe extern "system" fn get_speed(p: P<StateVtbl>, out: *mut i32) -> Result_ {
    let state = &*(p as *mut State);
    *out = state.speed;
    state.speed_result
}
unsafe extern "system" fn get_temperature(p: P<StateVtbl>, out: *mut i32) -> Result_ {
    let state = &*(p as *mut State);
    *out = state.temperature;
    state.temperature_result
}
unsafe extern "system" fn set_value(_: P<StateVtbl>, _: i32) -> Result_ {
    panic!("read-only curve discovery must never call setters")
}
unsafe extern "system" fn release_list(p: P<StateListVtbl>) -> i32 {
    (*(p as *mut List)).releases += 1;
    0
}
unsafe extern "system" fn size(p: P<StateListVtbl>) -> u32 {
    (*(p as *mut List)).count
}
unsafe extern "system" fn at(p: P<StateListVtbl>, index: u32, out: *mut P<StateVtbl>) -> Result_ {
    let list = &mut *(p as *mut List);
    if list.unavailable && index > 0 {
        *out = std::ptr::null_mut();
        return NOT_SUPPORTED;
    }
    *out = &mut list.state as *mut State as P<StateVtbl>;
    OK
}
static STATE: StateVtbl = StateVtbl {
    _acquire: 0,
    release: release_state,
    _query: 0,
    get_speed,
    set_speed: set_value,
    get_temperature,
    set_temperature: set_value,
};
static LIST: StateListVtbl = StateListVtbl {
    _acquire: 0,
    release: release_list,
    _query: 0,
    size,
    _empty: 0,
    _begin: 0,
    _end: 0,
    _at: 0,
    _clear: 0,
    _remove_back: 0,
    _add_back: 0,
    at_state: at,
};

fn fixture() -> List {
    List {
        vtbl: &LIST,
        state: State {
            vtbl: &STATE,
            speed: 25,
            temperature: 40,
            speed_result: OK,
            temperature_result: OK,
            releases: 0,
        },
        count: 2,
        unavailable: false,
        releases: 0,
    }
}

fn read(list: &mut List) -> Result<Curve, String> {
    unsafe { read_state_list(list as *mut List as P<StateListVtbl>) }
}

#[test]
fn reads_complete_curve_and_releases_each_reference() {
    let mut list = fixture();
    assert_eq!(read(&mut list).unwrap(), vec![(25, 40), (25, 40)]);
    assert_eq!(list.releases, 1);
    assert_eq!(list.state.releases, 2);
}

#[test]
fn incomplete_curves_are_errors_instead_of_captures_for_restoration() {
    for (speed_result, temperature_result, unavailable, count) in [
        (NOT_SUPPORTED, OK, false, 2),
        (OK, NOT_SUPPORTED, false, 2),
        (OK, OK, true, 2),
        (OK, OK, false, 0),
        (OK, OK, false, 17),
    ] {
        let mut list = fixture();
        list.state.speed_result = speed_result;
        list.state.temperature_result = temperature_result;
        list.unavailable = unavailable;
        list.count = count;
        assert!(read(&mut list).is_err());
        assert_eq!(list.releases, 1);
        assert_eq!(list.state.releases, u32::from(count == 2));
    }
}
