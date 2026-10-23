//! Windows tray icon and menu. Polls the daemon every few seconds; actions run
//! on worker threads so a UAC prompt or slow service never freezes the icon.
use crate::state::{alerts, TrayState};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

const POLL: Duration = Duration::from_secs(3);
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icons/freergb-32.png");

fn call(method: &str, params: Value) -> Option<Value> {
    freergb_client::call(method, params).ok()
}

fn read_state() -> TrayState {
    let version = call("daemon.version", Value::Null);
    if version.is_none() {
        return TrayState::default();
    }
    TrayState::from_replies(
        version,
        call("lighting.status", Value::Null),
        call("diagnostics.get", Value::Null),
        call("profiles.list", Value::Null),
        call("onboarding.get", Value::Null),
        call("cooling.background.get", Value::Null),
        call("cooling.profiles.list", Value::Null),
        call("cooling.sessions.status", Value::Null),
    )
}

fn sibling(name: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

fn spawn(name: &str, args: &[&str]) {
    if let Err(error) = Command::new(sibling(name)).args(args).spawn() {
        eprintln!("freergb-tray: cannot start {name}: {error}");
    }
}

/// RGBA from the bundled brand icon; the dimmed variant is greyscale and faded.
fn icon(dimmed: bool) -> Option<Icon> {
    let decoder = png::Decoder::new(std::io::Cursor::new(ICON_PNG));
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buffer.truncate(info.buffer_size());
    if dimmed {
        for pixel in buffer.chunks_exact_mut(4) {
            let grey = ((u16::from(pixel[0]) * 3 + u16::from(pixel[1]) * 6 + u16::from(pixel[2]))
                / 10) as u8;
            pixel[0] = grey;
            pixel[1] = grey;
            pixel[2] = grey;
            pixel[3] /= 2;
        }
    }
    Icon::from_rgba(buffer, info.width, info.height).ok()
}

enum Action {
    OpenGui,
    OpenClassic,
    StartService,
    PauseLighting(bool),
    Brightness(f64),
    Profile(String),
    PauseFans(bool),
    CoolingProfile(String),
    Quit,
}

struct Menus {
    menu: Menu,
    actions: Vec<(tray_icon::menu::MenuId, Action)>,
}

fn build_menu(state: &TrayState) -> Menus {
    let menu = Menu::new();
    let mut actions = Vec::new();
    fn add(
        menu: &Menu,
        actions: &mut Vec<(tray_icon::menu::MenuId, Action)>,
        item: &dyn tray_icon::menu::IsMenuItem,
        id: tray_icon::menu::MenuId,
        action: Action,
    ) {
        let _ = menu.append(item);
        actions.push((id, action));
    }
    let _ = menu.append(&MenuItem::new(state.status_line(), false, None));
    let _ = menu.append(&MenuItem::new(state.fan_line(), false, None));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let open = MenuItem::new("Open freeRGB", true, None);
    add(
        &menu,
        &mut actions,
        &open,
        open.id().clone(),
        Action::OpenGui,
    );
    let classic = MenuItem::new("Open freeRGB Classic", true, None);
    add(
        &menu,
        &mut actions,
        &classic,
        classic.id().clone(),
        Action::OpenClassic,
    );
    let _ = menu.append(&PredefinedMenuItem::separator());
    if !state.connected {
        let start = MenuItem::new("Start freeRGB service", true, None);
        add(
            &menu,
            &mut actions,
            &start,
            start.id().clone(),
            Action::StartService,
        );
    } else {
        let pause = CheckMenuItem::new("Pause lighting", true, state.lighting_paused, None);
        add(
            &menu,
            &mut actions,
            &pause,
            pause.id().clone(),
            Action::PauseLighting(!state.lighting_paused),
        );
        let brightness = Submenu::new("Brightness", !state.lighting_paused);
        for percent in [25, 50, 75, 100] {
            let value = f64::from(percent) / 100.0;
            let current = state.brightness.is_some_and(|b| (b - value).abs() < 0.01);
            let item = CheckMenuItem::new(format!("{percent} %"), true, current, None);
            let _ = brightness.append(&item);
            actions.push((item.id().clone(), Action::Brightness(value)));
        }
        let _ = menu.append(&brightness);
        if !state.profiles.is_empty() {
            let profiles = Submenu::new("Lighting profile", true);
            for name in &state.profiles {
                let item = MenuItem::new(name, true, None);
                let _ = profiles.append(&item);
                actions.push((item.id().clone(), Action::Profile(name.clone())));
            }
            let _ = menu.append(&profiles);
        }
        let _ = menu.append(&PredefinedMenuItem::separator());
        let fans = state.fans_available && !state.fans_locked;
        let pause_fans = CheckMenuItem::new(
            "Pause fan control (BIOS takes over)",
            fans,
            state.fans_paused,
            None,
        );
        add(
            &menu,
            &mut actions,
            &pause_fans,
            pause_fans.id().clone(),
            Action::PauseFans(!state.fans_paused),
        );
        if fans && !state.cooling_profiles.is_empty() {
            let profiles = Submenu::new("Cooling profile (asks for permission)", true);
            for name in &state.cooling_profiles {
                let active = state.active_cooling_profile.as_deref() == Some(name.as_str());
                let item = CheckMenuItem::new(name, true, active, None);
                let _ = profiles.append(&item);
                actions.push((item.id().clone(), Action::CoolingProfile(name.clone())));
            }
            let _ = menu.append(&profiles);
        }
    }
    let _ = menu.append(&PredefinedMenuItem::separator());
    let quit = MenuItem::new("Quit freeRGB", true, None);
    add(&menu, &mut actions, &quit, quit.id().clone(), Action::Quit);
    Menus { menu, actions }
}

/// Runs an action without blocking the message loop; `done` triggers a refresh.
fn perform(action: &Action, done: mpsc::Sender<()>) -> bool {
    match action {
        Action::OpenGui => spawn("freergb-desktop.exe", &[]),
        Action::OpenClassic => spawn("freergb-gui.exe", &[]),
        Action::StartService => spawn("freergb-launcher.exe", &["--daemon-only"]),
        Action::Quit => {
            // Stops the service; lighting returns to hardware mode and fans to
            // firmware/background control as on any daemon shutdown.
            let _ = call("daemon.shutdown", Value::Null);
            return false;
        }
        other => {
            let (method, params) = match other {
                Action::PauseLighting(paused) => ("lighting.pause", json!({"paused": paused})),
                Action::Brightness(value) => ("brightness.set", json!({"value": value})),
                Action::Profile(name) => ("profiles.load", json!({"name": name})),
                Action::PauseFans(paused) => ("cooling.fans.pause", json!({"paused": paused})),
                Action::CoolingProfile(name) => {
                    ("cooling.profiles.activate", json!({"name": name}))
                }
                _ => unreachable!(),
            };
            std::thread::spawn(move || {
                if let Err(error) = freergb_client::call(method, params) {
                    eprintln!("freergb-tray: {method} failed: {error}");
                }
                let _ = done.send(());
            });
        }
    }
    true
}

/// Windows toast (falls back silently if notifications are off). The app id
/// matches the Start-menu shortcut created by Setup.
fn notify(text: &str) {
    let _ = tauri_winrt_notification::Toast::new("freeRGB.Desktop")
        .title("freeRGB")
        .text1(text)
        .show();
}

/// One tray per user session (named mutex).
fn already_running() -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    let name: Vec<u16> = "Local\\freeRGB.Tray.v1\0".encode_utf16().collect();
    // SAFETY: valid NUL-terminated name; the handle is intentionally kept for
    // the process lifetime (released by Windows on exit).
    #[allow(unsafe_code)]
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        handle.is_null() || GetLastError() == ERROR_ALREADY_EXISTS
    }
}

/// Dispatches pending window messages (the tray icon's hidden window lives on
/// this thread). Returns false on WM_QUIT.
fn pump() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, WM_QUIT,
    };
    // SAFETY: standard Win32 message loop on the owning thread with a zeroed MSG.
    #[allow(unsafe_code)]
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if message.message == WM_QUIT {
                return false;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    true
}

pub fn run() {
    if already_running() {
        return;
    }
    let mut state = read_state();
    let mut menus = build_menu(&state);
    let Ok(tray): Result<TrayIcon, _> = TrayIconBuilder::new()
        .with_menu(Box::new(menus.menu.clone()))
        .with_tooltip(state.tooltip())
        .with_icon(
            icon(state.dimmed())
                .unwrap_or_else(|| Icon::from_rgba(vec![0; 4], 1, 1).expect("1x1 icon")),
        )
        .build()
    else {
        return;
    };
    let (done, refresh) = mpsc::channel();
    let mut next_poll = Instant::now() + POLL;
    loop {
        if !pump() {
            return;
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                spawn("freergb-desktop.exe", &[]);
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some((_, action)) = menus.actions.iter().find(|(id, _)| *id == event.id) {
                if !perform(action, done.clone()) {
                    return;
                }
                next_poll = Instant::now() + Duration::from_millis(400);
            }
        }
        if refresh.try_recv().is_ok() {
            next_poll = Instant::now();
        }
        if Instant::now() >= next_poll {
            let fresh = read_state();
            for text in alerts(&state, &fresh) {
                notify(&text);
            }
            if fresh != state {
                state = fresh;
                menus = build_menu(&state);
                tray.set_menu(Some(Box::new(menus.menu.clone())));
                let _ = tray.set_tooltip(Some(state.tooltip()));
                let _ = tray.set_icon(icon(state.dimmed()));
            }
            next_poll = Instant::now() + POLL;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}
