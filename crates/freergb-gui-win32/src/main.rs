//! freeRGB Classic (Win32). Owns the window, the message loop and the chrome
//! (header, page navigation, device panel, scroll thumb); everything inside
//! the page area comes from [`crate::pages`], every daemon action goes
//! through [`crate::rpc`]. Repaints are input-driven plus a 33 ms timer while
//! something animates (preview, dropdown, caret, toast note).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod ffi;
#[cfg(windows)]
mod pages;
#[cfg(windows)]
mod preview;
// Platform-neutral so its tests run everywhere; only the Windows app uses it.
#[cfg_attr(not(windows), allow(dead_code))]
mod rpc;
#[cfg(windows)]
mod theme;
#[cfg(windows)]
mod ui;

fn main() {
    #[cfg(windows)]
    {
        app::run();
    }
    #[cfg(not(windows))]
    {
        eprintln!("freeRGB Classic GUI requires Windows.");
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod app {
    use crate::ffi::*;
    use crate::pages::{Pages, PAGES};
    use crate::rpc::{Command, Snapshot};
    use crate::theme::{self, Font, Fonts, Style};
    use crate::ui::{self, Input, Rect, Ui, Widgets};
    use serde_json::Value;
    use std::sync::mpsc::Sender;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    const NAV_WIDTH: f32 = 176.0;
    const DEVICES_WIDTH: f32 = 252.0;
    const HEADER_HEIGHT: f32 = 64.0;
    const MARGIN: f32 = 12.0;
    const ROW: f32 = 8.0;
    const NOTE_HEIGHT: f32 = 26.0;
    const TIMER_MS: u32 = 33;

    pub struct App {
        pub input: Input,
        pub widgets: Widgets,
        pub pages: Pages,
        pub style: Style,
        pub scale: f32,
        pub fonts_dpi: u32,
        pub fonts: Fonts,
        pub brushes: theme::Brushes,
        pub dc: HDC,
        pub bitmap: HBITMAP,
        pub size: (i32, i32),
        pub snapshot: Arc<Mutex<Snapshot>>,
        pub tx: Sender<Command>,
        /// Scroll thumb grab state: (grab y, scroll at grab).
        pub anchor: Option<(f32, f32)>,
    }

    fn empty_fonts() -> Fonts {
        Fonts {
            body: std::ptr::null_mut(),
            small: std::ptr::null_mut(),
            strong: std::ptr::null_mut(),
            heading: std::ptr::null_mut(),
            mono: std::ptr::null_mut(),
            line: 16.0,
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);
        wide
    }

    pub fn run() {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let instance = GetModuleHandleW(std::ptr::null());
            let class_name = wide("freeRGBClassic");
            let title = wide("freeRGB Classic");
            let icon =
                LoadImageW(instance, 1 as *const u16, IMAGE_ICON, 0, 0, LR_DEFAULTSIZE) as HICON;
            let class = WNDCLASSW {
                style: CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: icon,
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            if RegisterClassW(&class) == 0 {
                let message = wide("RegisterClassW failed.");
                MessageBoxW(
                    std::ptr::null_mut(),
                    message.as_ptr(),
                    title.as_ptr(),
                    MB_ICONERROR | MB_OK,
                );
                return;
            }
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                (WS_OVERLAPPEDWINDOW | WS_VISIBLE) as WINDOW_STYLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1000,
                680,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null_mut(),
            );
            if hwnd.is_null() {
                let message = wide("CreateWindowExW failed.");
                MessageBoxW(
                    std::ptr::null_mut(),
                    message.as_ptr(),
                    title.as_ptr(),
                    MB_ICONERROR | MB_OK,
                );
                return;
            }
            let (tx, snapshot) = crate::rpc::start(hwnd as isize);
            let app = Box::new(App {
                input: Input::default(),
                widgets: Widgets::default(),
                pages: Pages::default(),
                style: theme::load_style(),
                scale: 1.0,
                fonts_dpi: 0,
                fonts: empty_fonts(),
                brushes: theme::Brushes::default(),
                dc: std::ptr::null_mut(),
                bitmap: std::ptr::null_mut(),
                size: (0, 0),
                snapshot,
                tx,
                anchor: None,
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(app) as isize);
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
            SetTimer(hwnd, 1, TIMER_MS, None);
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    /// Client position (physical px) from a message `lParam`.
    fn point(lparam: LPARAM) -> (i32, i32) {
        (
            (lparam & 0xFFFF) as u16 as i16 as i32,
            ((lparam >> 16) & 0xFFFF) as u16 as i16 as i32,
        )
    }

    /// Layout of the scrollable page column: the view rect, the page-space y
    /// the content starts at (`headroom`) and the visible height.
    fn content_rect(pages: &Pages, ui_width: f32, ui_height: f32) -> (Rect, f32, f32) {
        let x0 = NAV_WIDTH + DEVICES_WIDTH + MARGIN;
        let width = (ui_width - x0 - MARGIN).max(120.0);
        let headroom = content_headroom(pages);
        let visible = (ui_height - headroom - MARGIN).max(40.0);
        (
            Rect::new(x0, HEADER_HEIGHT + ROW, width, visible),
            headroom,
            visible,
        )
    }

    fn note_open(pages: &Pages) -> bool {
        pages
            .note
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() < Duration::from_secs(4))
    }

    fn content_headroom(pages: &Pages) -> f32 {
        HEADER_HEIGHT
            + ROW
            + if note_open(pages) {
                NOTE_HEIGHT + ROW
            } else {
                0.0
            }
            + ROW
    }

    #[cfg(windows)]
    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
        if state.is_null() {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
        let app = &mut *state;
        match message {
            WM_PAINT => {
                paint(app, hwnd);
                0
            }
            WM_ERASEBKGND => 1,
            WM_SIZE => {
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_DPICHANGED => {
                app.fonts_dpi = 0;
                let rect = lparam as *const RECT;
                if !rect.is_null() {
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        (*rect).left,
                        (*rect).top,
                        (*rect).right - (*rect).left,
                        (*rect).bottom - (*rect).top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                0
            }
            WM_TIMER => {
                // Repaint only while something animates.
                if app.pages.page == 0
                    || note_open(&app.pages)
                    || app.widgets.popup.is_some()
                    || app.widgets.focus.is_some()
                {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
                0
            }
            WM_MOUSEMOVE => {
                let (x, y) = point(lparam);
                app.input.mouse = (x as f32 / app.scale, y as f32 / app.scale);
                app.input.valid = true;
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_LBUTTONDOWN => {
                SetCapture(hwnd);
                app.input.down = true;
                app.input.just_down = true;
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_LBUTTONDBLCLK => {
                SetCapture(hwnd);
                app.input.down = true;
                app.input.just_down = true;
                app.input.double_click = true;
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_LBUTTONUP => {
                ReleaseCapture();
                app.input.down = false;
                app.input.just_up = true;
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_RBUTTONDOWN => {
                app.input.right_down = true;
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_RBUTTONUP => {
                app.input.right_down = false;
                app.input.right_released = true;
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_MOUSEWHEEL => {
                let delta = (wparam >> 16) as i16;
                app.input.wheel += (delta as f32 / 120.0).round() as i32;
                if app.widgets.popup.is_none() {
                    let mut pt = POINT {
                        x: point(lparam).0,
                        y: point(lparam).1,
                    };
                    ScreenToClient(hwnd, &mut pt);
                    let x = pt.x as f32 / app.scale;
                    if app.input.valid {
                        let mut rect: RECT = std::mem::zeroed();
                        GetClientRect(hwnd, &mut rect);
                        let width = rect.right as f32 / app.scale;
                        let height = rect.bottom as f32 / app.scale;
                        if x >= NAV_WIDTH + DEVICES_WIDTH {
                            let (_, headroom, visible) = content_rect(&app.pages, width, height);
                            let max =
                                (app.pages.content_height + app.pages.scroll - headroom - visible)
                                    .max(0.0);
                            app.pages.scroll =
                                (app.pages.scroll - app.input.wheel as f32 * 48.0).clamp(0.0, max);
                        } else if x >= NAV_WIDTH {
                            let visible = (height - HEADER_HEIGHT - ROW - MARGIN).max(40.0);
                            let max = (app.pages.devices_height + app.pages.devices_scroll
                                - visible)
                                .max(0.0);
                            app.pages.devices_scroll = (app.pages.devices_scroll
                                - app.input.wheel as f32 * 48.0)
                                .clamp(0.0, max);
                        }
                    }
                }
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_CHAR => {
                app.input.unicode.push(wparam as u16);
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_KEYDOWN => {
                app.input.keys.push(wparam as u32);
                if wparam as u32 == 0x56 && GetKeyState(VK_CONTROL as i32) < 0 {
                    app.input.paste = true;
                }
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_KEYUP => {
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_DESTROY => {
                drop(Box::from_raw(state));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                KillTimer(hwnd, 1);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe fn client_size(hwnd: HWND) -> (i32, i32) {
        let mut rect: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rect);
        (rect.right.max(0), rect.bottom.max(0))
    }

    unsafe fn paint(app: &mut App, hwnd: HWND) {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc.is_null() {
            return;
        }
        let dpi = GetDpiForWindow(hwnd);
        let scale = dpi.max(96) as f32 / 96.0;
        if scale != app.scale {
            app.scale = scale;
            app.fonts_dpi = 0;
        }
        let (w_px, h_px) = client_size(hwnd);
        if w_px == 0 || h_px == 0 {
            EndPaint(hwnd, &ps);
            return;
        }
        if app.dc.is_null() || app.size != (w_px, h_px) {
            if !app.dc.is_null() {
                DeleteDC(app.dc);
                DeleteObject(app.bitmap as _);
            }
            app.dc = CreateCompatibleDC(hdc);
            app.size = (w_px, h_px);
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            app.bitmap = CreateDIBSection(
                hdc,
                &bitmap_info(w_px, -h_px),
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            SelectObject(app.dc, app.bitmap as HGDIOBJ);
            app.fonts = theme::create_fonts(hdc, scale);
            app.fonts_dpi = dpi;
        } else if app.fonts_dpi != dpi {
            app.fonts = theme::create_fonts(hdc, scale);
            app.fonts_dpi = dpi;
        }
        SetBkMode(app.dc, TRANSPARENT as i32);
        let palette = theme::palette(app.style);
        let (width, height) = (w_px as f32 / scale, h_px as f32 / scale);
        // Clear the backing store first: every widget paints onto the clean
        // frame, nothing of the previous frame survives (labels and sliders
        // fill only their own cells).
        let client = RECT {
            left: 0,
            top: 0,
            right: w_px,
            bottom: h_px,
        };
        FillRect(app.dc, &client, app.brushes.get(palette.window));
        let guard = app
            .snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let snapshot: &Snapshot = &guard;
        // Chrome pass: header, navigation and the scroll thumb (absolute
        // input). Device pass and content pass follow below.
        {
            let mut ui = ui::Ui::new(
                app.dc,
                scale,
                width,
                height,
                palette,
                app.fonts,
                &app.input,
                &mut app.widgets,
                &mut app.brushes,
            );
            header(&mut ui, snapshot);
            nav(&mut ui, &mut app.pages, &mut app.style);
            chrome_scrollbar(&mut ui, &mut app.pages, &mut app.anchor);
        }
        {
            let mut ui = ui::Ui::new(
                app.dc,
                scale,
                width,
                height,
                palette,
                app.fonts,
                &app.input,
                &mut app.widgets,
                &mut app.brushes,
            );
            devices_panel(&mut ui, &mut app.pages, &app.tx, snapshot);
        }
        {
            // Content pass: the mouse is translated into page space so every
            // widget below sees page coordinates. The wheel reaches the page
            // only while the dropdown or the toast note is open (the list
            // scrolls), otherwise the chrome already scrolled the column.
            let popup_open = app.widgets.popup.is_some();
            let wheel = if popup_open { app.input.wheel } else { 0 };
            let shifted = shifted_input(&app.input, app.pages.scroll, wheel);
            let mut ui = ui::Ui::new(
                app.dc,
                scale,
                width,
                height,
                palette,
                app.fonts,
                &shifted,
                &mut app.widgets,
                &mut app.brushes,
            );
            content(&mut ui, &mut app.pages, &app.tx, snapshot);
        }
        if app.input.just_up {
            app.widgets.press = None;
        }
        app.input.begin_frame();
        BitBlt(hdc, 0, 0, w_px, h_px, app.dc, 0, 0, SRCCOPY);
        EndPaint(hwnd, &ps);
    }

    fn bitmap_info(width: i32, height: i32) -> BITMAPINFO {
        BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [unsafe { std::mem::zeroed() }],
        }
    }

    fn shifted_input(input: &Input, scroll: f32, wheel: i32) -> Input {
        Input {
            mouse: (input.mouse.0, input.mouse.1 - scroll),
            valid: input.valid,
            down: input.down,
            right_down: input.right_down,
            just_down: input.just_down,
            just_up: input.just_up,
            double_click: input.double_click,
            right_released: input.right_released,
            wheel,
            unicode: input.unicode.clone(),
            keys: input.keys.clone(),
            paste: input.paste,
        }
    }

    // ---- chrome ----

    fn header(ui: &mut Ui, snapshot: &Snapshot) {
        let rect = Rect::new(0.0, 0.0, ui.width, HEADER_HEIGHT);
        if ui.palette.chrome_gradient {
            ui.gradient_v(&rect, theme::rgb(252, 253, 254), theme::rgb(225, 236, 249));
        } else {
            ui.fill(&rect, ui.palette.nav);
        }
        ui.line(
            0.0,
            HEADER_HEIGHT - 1.0,
            ui.width,
            HEADER_HEIGHT - 1.0,
            ui.palette.border,
            1,
        );
        ui.text(
            &Rect::new(MARGIN, 14.0, 260.0, 26.0),
            "freeRGB · Classic",
            Font::Heading,
            ui.palette.text,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        let connected = snapshot.connected;
        let color = if connected { theme::GOOD } else { theme::BAD };
        let status = if connected {
            "● daemon connected"
        } else {
            "● daemon not reachable"
        };
        ui.text(
            &Rect::new(ui.width - 290.0, 14.0, 278.0, 26.0),
            status,
            Font::Body,
            color,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE,
        );
        if connected {
            let fps = snapshot
                .diagnostics
                .as_ref()
                .and_then(|diag| diag.get("measured_fps"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            ui.text(
                &Rect::new(ui.width - 290.0, 38.0, 278.0, 16.0),
                &format!("{fps:.1} fps"),
                Font::Small,
                ui.palette.text_dim,
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE,
            );
        } else {
            ui.text(
                &Rect::new(ui.width - 290.0, 38.0, 278.0, 16.0),
                "Start freergbd to connect.",
                Font::Small,
                ui.palette.text_dim,
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE,
            );
        }
        // One status line: an RPC error wins over running-conflict warnings.
        if let Some(error) = &snapshot.error {
            status_line(ui, error, theme::BAD);
            return;
        }
        if let Some(conflicts) = snapshot
            .diagnostics
            .as_ref()
            .and_then(|diag| diag.get("conflicts"))
            .and_then(Value::as_array)
        {
            let names: Vec<&str> = conflicts.iter().filter_map(Value::as_str).collect();
            if !names.is_empty() {
                status_line(
                    ui,
                    &format!(
                        "{} is running – freeRGB leaves your hardware alone until it is closed.",
                        names.join(", ")
                    ),
                    theme::WARN,
                );
            }
        }
    }

    fn status_line(ui: &mut Ui, text: &str, color: COLORREF) {
        let mut text = text.to_owned();
        if text.chars().count() > 140 {
            text = text.chars().take(140).collect::<String>() + "…";
        }
        ui.text(
            &Rect::new(MARGIN + 8.0, 38.0, (ui.width - 600.0).max(60.0), 16.0),
            &text,
            Font::Small,
            color,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }

    fn nav(ui: &mut Ui, pages: &mut Pages, style: &mut Style) {
        let rail = Rect::new(
            0.0,
            HEADER_HEIGHT - 1.0,
            NAV_WIDTH,
            (ui.height - HEADER_HEIGHT).max(0.0),
        );
        ui.fill(&rail, ui.palette.nav);
        ui.line(
            NAV_WIDTH,
            HEADER_HEIGHT,
            NAV_WIDTH,
            ui.height,
            ui.palette.border,
            1,
        );
        let mut y = HEADER_HEIGHT + MARGIN;
        for (name, index) in PAGES {
            let rect = Rect::new(10.0, y, NAV_WIDTH - 20.0, 30.0);
            let id = ui::id(&["chrome", "nav", name]);
            ui.claim_press(id, &rect);
            let hovered = ui.contains(&rect);
            if pages.page == index {
                ui.box_(
                    &rect,
                    Some((ui.palette.list_selected, ui.palette.list_selected)),
                    Some(ui.palette.list_selected_border),
                );
            } else if hovered {
                ui.box_(
                    &rect,
                    Some((ui.palette.list_hover, ui.palette.list_hover)),
                    None,
                );
            }
            ui.text(
                &rect,
                name,
                Font::Body,
                ui.palette.text,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            if ui.click(id, &rect) {
                pages.page = index;
                ui.widgets.popup = None;
            }
            y += 36.0;
        }
        // Skin switch at the rail's bottom; the choice is saved next to the
        // Classic GUI's files.
        let skin = Rect::new(
            10.0,
            (ui.height - MARGIN - 30.0).max(y),
            NAV_WIDTH - 20.0,
            30.0,
        );
        let id = ui::id(&["chrome", "skin"]);
        if ui.button_at(id, &skin, &format!("Skin: {}", style.label()), false) {
            *style = style.other();
            theme::save_style(*style);
        }
    }

    /// Vertical scroll thumb of the page column (the wheel also scrolls).
    fn chrome_scrollbar(ui: &mut Ui, pages: &mut Pages, anchor: &mut Option<(f32, f32)>) {
        let (view, headroom, visible) = content_rect(pages, ui.width, ui.height);
        // `content` stores the final cursor y (= headroom - scroll + height),
        // so the page height is content_height + scroll - headroom.
        let total = (pages.content_height + pages.scroll - headroom).max(visible);
        pages.scroll = pages.scroll.clamp(0.0, (total - visible).max(0.0));
        if total <= visible {
            *anchor = None;
            return;
        }
        let track_w = 8.0;
        let thumb_h = (visible * visible / total).clamp(24.0, visible);
        let fraction = pages.scroll / (total - visible).max(1.0);
        let thumb = Rect::new(
            view.right() - track_w - 1.0,
            view.y + (visible - thumb_h) * fraction,
            track_w,
            thumb_h,
        );
        ui.fill(&thumb, ui.palette.track);
        if ui.contains(&thumb) {
            // Aero scroll thumbs turn blue under the pointer.
            ui.fill(&thumb, ui.palette.track_fill);
        }
        let id = ui::id(&["chrome", "thumb"]);
        if ui.input.just_down && ui.contains(&thumb) && ui.widgets.press.is_none() {
            ui.widgets.press = Some(id);
            *anchor = Some((ui.input.mouse.1, pages.scroll));
        }
        if ui.widgets.press == Some(id) {
            if let Some((grab, base)) = *anchor {
                let dy = ui.input.mouse.1 - grab;
                pages.scroll =
                    (base + dy * (total / visible)).clamp(0.0, (total - visible).max(0.0));
            }
        } else {
            *anchor = None;
        }
    }

    fn devices_panel(ui: &mut Ui, pages: &mut Pages, tx: &Sender<Command>, snapshot: &Snapshot) {
        let x0 = NAV_WIDTH;
        ui.fill(
            &Rect::new(
                x0 + 1.0,
                HEADER_HEIGHT,
                (DEVICES_WIDTH - 2.0).max(0.0),
                (ui.height - HEADER_HEIGHT).max(0.0),
            ),
            ui.palette.devices,
        );
        ui.line(x0, HEADER_HEIGHT, x0, ui.height, ui.palette.border, 1);
        let top = HEADER_HEIGHT + ROW;
        let visible = (ui.height - top - ROW).max(40.0);
        {
            let cards = Rect::new(x0, top, DEVICES_WIDTH, visible);
            ui.clip(&cards, |ui| {
                ui.set_bounds(x0 + 12.0, x0 + DEVICES_WIDTH - 12.0);
                ui.set_cursor(x0 + 12.0, top - pages.devices_scroll);
                ui.heading("Devices");
                if snapshot.devices.is_empty() {
                    ui.label_color(
                        if snapshot.connected {
                            "No devices."
                        } else {
                            "Start freergbd to see devices."
                        },
                        Font::Body,
                        ui.palette.text_dim,
                    );
                }
                for device in &snapshot.devices {
                    device_card(ui, pages, tx, device, snapshot.connected);
                }
                if !snapshot.plugins.is_empty() {
                    ui.strong("Plugins");
                    for plugin in &snapshot.plugins {
                        let id = plugin["id"].as_str().unwrap_or_default().to_owned();
                        let enabled = plugin["enabled"].as_bool().unwrap_or(false);
                        let allowed = plugin["allowed"].as_bool().unwrap_or(false);
                        ui.text(
                            &Rect::new(ui.bounds.0, ui.cursor.1, ui.column_width(), 16.0),
                            plugin["name"].as_str().unwrap_or(&id),
                            Font::Body,
                            ui.palette.text,
                            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                        );
                        ui.advance(16.0 + ROW);
                        if ui.button_enabled(
                            ui::id(&["chrome", "plugin", &id]),
                            if enabled {
                                "Stop plugin"
                            } else {
                                "Start plugin"
                            },
                            false,
                            snapshot.connected && allowed,
                        ) {
                            let method = if enabled {
                                "plugins.stop"
                            } else {
                                "plugins.start"
                            };
                            let _ = tx.send(Command {
                                method,
                                params: serde_json::json!({"id": id}),
                            });
                        }
                        if !allowed {
                            ui.hint("Hardware disabled for this daemon.");
                        }
                        if let Some(error) = plugin["last_error"].as_str() {
                            ui.label_color(error, Font::Small, theme::BAD);
                        }
                    }
                }
                // Page-space height of the panel content, same trick as the
                // page column: the cursor runs at `- scroll`.
                pages.devices_height = ui.cursor.1 - (HEADER_HEIGHT + ROW) + pages.devices_scroll;
            });
        }
        let total = pages.devices_height.max(visible);
        pages.devices_scroll = pages.devices_scroll.clamp(0.0, (total - visible).max(0.0));
    }

    fn device_card(
        ui: &mut Ui,
        pages: &mut Pages,
        tx: &Sender<Command>,
        device: &Value,
        connected: bool,
    ) {
        let info = device.get("info").cloned().unwrap_or(Value::Null);
        let name = info["name"].as_str().unwrap_or("?");
        let id = info["id"].as_str().unwrap_or_default().to_owned();
        let status = match device.get("status") {
            Some(Value::String(text)) => text.clone(),
            Some(Value::Object(map)) => map.keys().next().cloned().unwrap_or_default(),
            _ => "?".into(),
        };
        let error = device
            .get("last_error")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let firmware = info
            .get("firmware")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let card_h = 92.0
            + if firmware.is_some() || error.is_some() {
                16.0
            } else {
                0.0
            };
        let column = (ui.bounds.0, ui.bounds.1);
        let card = Rect::new(
            ui.bounds.0 - 4.0,
            ui.cursor.1,
            ui.column_width() + 8.0,
            card_h,
        );
        ui.box_(
            &card,
            Some((ui.palette.card, ui.palette.card)),
            Some(ui.palette.border),
        );
        let mut y = card.y + 8.0;
        ui.text(
            &Rect::new(card.x + 10.0, y, card.w - 24.0, 16.0),
            name,
            Font::Strong,
            ui.palette.text,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        y += 18.0;
        ui.text(
            &Rect::new(card.x + 10.0, y, card.w - 24.0, 14.0),
            &format!(
                "{} · {} LEDs · {} frames",
                info["vendor"].as_str().unwrap_or("?"),
                device.get("leds").and_then(Value::as_u64).unwrap_or(0),
                device.get("frames").and_then(Value::as_u64).unwrap_or(0),
            ),
            Font::Small,
            ui.palette.text_dim,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        y += 16.0;
        if let Some(firmware) = &firmware {
            ui.text(
                &Rect::new(card.x + 10.0, y, card.w - 24.0, 14.0),
                &format!("Firmware {firmware}"),
                Font::Small,
                ui.palette.text_dim,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 16.0;
        }
        ui.text(
            &Rect::new(card.x + 10.0, y, card.w - 24.0, 14.0),
            &format!("● {status}"),
            Font::Small,
            if status == "Connected" {
                theme::GOOD
            } else {
                theme::WARN
            },
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        y += 17.0;
        // Per-device brightness, mirroring the Classic device row.
        let reported = device
            .get("brightness")
            .and_then(Value::as_f64)
            .unwrap_or(1.0) as f32;
        if pages
            .device_brightness
            .get(&id)
            .is_some_and(|value| (value - reported).abs() < 0.005)
        {
            pages.device_brightness.remove(&id);
        }
        let mut value = pages
            .device_brightness
            .get(&id)
            .copied()
            .unwrap_or(reported);
        let display = format!("{:.0} %", value * 100.0);
        ui.set_bounds(card.x + 10.0, card.right() - 10.0);
        ui.set_cursor(card.x + 10.0, y);
        if ui.slider(
            ui::id(&["chrome", "device", &id]),
            &mut value,
            0.0,
            1.0,
            "Brightness",
            &display,
        ) {
            pages.device_brightness.insert(id.clone(), value);
            if connected {
                let _ = tx.send(Command {
                    method: "devices.set_brightness",
                    params: serde_json::json!({"id": id, "value": value}),
                });
            }
        }
        if let Some(error) = &error {
            ui.set_bounds(column.0, column.1);
            ui.label_color(error, Font::Small, theme::BAD);
        }
        ui.set_bounds(column.0, column.1);
        ui.set_cursor(column.0, card.bottom() + ROW);
    }

    /// The page column: toast note, scrolled page content and the dropdown
    /// popup rendered inside the same clip (page-space coordinates).
    fn content(ui: &mut Ui, pages: &mut Pages, tx: &Sender<Command>, snapshot: &Snapshot) {
        let (view, headroom, _) = content_rect(pages, ui.width, ui.height);
        if note_open(pages) {
            if let Some((text, _)) = &pages.note {
                let note = Rect::new(view.x, view.y, view.w.min(480.0), NOTE_HEIGHT);
                ui.box_(
                    &note,
                    Some((ui.palette.accent_dim, ui.palette.accent_dim)),
                    Some(ui.palette.accent),
                );
                ui.text(
                    &note,
                    text,
                    Font::Body,
                    ui.palette.text,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
            }
        }
        ui.clip(&view, |ui| {
            ui.set_bounds(view.x, view.right());
            ui.set_cursor(view.x, headroom - pages.scroll);
            crate::pages::draw(ui, pages, snapshot, tx);
            ui.draw_popup();
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn content_layout_tracks_the_headroom_constants() {
            let mut pages = Pages::default();
            pages.note = None;
            pages.scroll = 0.0;
            pages.content_height = 0.0;
            let (view, headroom, visible) = content_rect(&pages, 1000.0, 600.0);
            // Without a toast note the page starts one row below the header.
            assert!(
                (headroom - (HEADER_HEIGHT + ROW + ROW)).abs() < 0.01,
                "plain headroom"
            );
            assert!((view.y - (HEADER_HEIGHT + ROW)).abs() < 0.01, "view top");
            assert!(
                (visible - (600.0 - headroom - MARGIN)).abs() < 0.01,
                "visible"
            );
            // With a note open the content moves down; the same math drives
            // the scroll clamp in the wheel handler.
            pages.note = Some(("hello".to_owned(), std::time::Instant::now()));
            let (_, headroom_note, _) = content_rect(&pages, 1000.0, 600.0);
            assert!(
                (headroom_note - headroom - NOTE_HEIGHT - ROW).abs() < 0.01,
                "note headroom"
            );
        }
    }
}
