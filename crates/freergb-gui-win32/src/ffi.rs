//! All windows-sys imports in one place so the GUI code stays readable and
//! every FFI name is verified against the local windows-sys 0.61 sources.
#![allow(clippy::too_many_lines)]

pub use windows_sys::Win32::Foundation::{
    COLORREF, HGLOBAL, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
pub use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect,
    GetStockObject, GetTextExtentPoint32W, IntersectClipRect, InvalidateRect, LineTo, MoveToEx,
    RestoreDC, RoundRect, SaveDC, ScreenToClient, SelectObject, SetBkMode, SetTextColor,
    StretchDIBits, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, HBITMAP, HBRUSH, HFONT, HDC,
    HGDIOBJ, PAINTSTRUCT, BI_RGB, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, DIB_RGB_COLORS, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX,
    DT_RIGHT, DT_SINGLELINE, DT_VCENTER, FW_NORMAL, FW_SEMIBOLD, NULL_BRUSH, OUT_DEFAULT_PRECIS,
    PS_SOLID, SRCCOPY, TRANSPARENT,
};
pub use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
/// `CF_UNICODETEXT` (13). windows-sys 0.61 only ships this constant behind the
/// `Win32_System_Ole` feature, so the stable value is spelled out here.
pub const CF_UNICODETEXT: u32 = 13;
pub use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
pub use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
pub use windows_sys::Win32::System::SystemInformation::GetTickCount64;
pub use windows_sys::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
pub use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL,
};
pub use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowLongPtrW, HICON, KillTimer, LoadCursorW, LoadImageW, MessageBoxW, PostMessageW,
    PostQuitMessage, RegisterClassW, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    TranslateMessage, MSG, WNDCLASSW, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
    GWLP_USERDATA, IDC_ARROW, IMAGE_ICON, LR_DEFAULTSIZE, MB_ICONERROR, MB_OK, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_SHOW, WINDOW_STYLE, WM_CHAR, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SIZE, WM_TIMER,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};