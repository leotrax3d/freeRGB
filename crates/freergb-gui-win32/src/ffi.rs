//! All windows-sys imports in one place so the GUI code stays readable and
//! every FFI name is verified against the local windows-sys 0.61 sources.
#![allow(clippy::too_many_lines)]

pub use windows_sys::Win32::Foundation::{
    CloseHandle, COLORREF, HANDLE, HBRUSH, HCURSOR, HDC, HGLOBAL, HICON, HINSTANCE, HMODULE,
    HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
pub use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect,
    GetStockObject, GetTextExtentPoint32W, IntersectClipRect, InvalidateRect, LineTo, MoveToEx,
    RestoreDC, RoundRect, SaveDC, ScreenToClient, SelectClipRgn, SelectObject, SetBkMode,
    SetRectEmpty, SetTextColor, StretchDIBits, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER,
    HBITMAP, HFONT, HGDIOBJ, PAINTSTRUCT, BI_RGB, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS,
    DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, FW_NORMAL,
    FW_SEMIBOLD, LR_DEFAULTSIZE, NULL_BRUSH, NULL_PEN, OUT_DEFAULT_PRECIS, PS_SOLID, SRCCOPY,
    TRANSPARENT, WHITE_BRUSH,
};
pub use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, OpenClipboard, CF_UNICODETEXT,
};
pub use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
pub use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
pub use windows_sys::Win32::System::SystemInformation::GetTickCount64;
pub use windows_sys::Win32::System::SystemServices::{MK_LBUTTON, MK_RBUTTON};
pub use windows_sys::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
pub use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL,
};
pub use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, DestroyWindow, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, KillTimer, LoadCursorW, LoadImageW, MessageBoxW, PostMessageW,
    PostQuitMessage, RegisterClassW, SendMessageW, SetTimer, SetWindowLongPtrW, ShowWindow,
    TranslateMessage, MSG, WNDCLASSW, WNDPROC, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
    GWLP_USERDATA, HMENU, ICON_BIG, ICON_SMALL, IDC_ARROW, IMAGE_ICON, MB_ICONERROR, MB_OK,
    SW_SHOW, TIMERPROC, WINDOW_STYLE, WM_CHAR, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND,
    WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR,
    WM_SETICON, WM_SIZE, WM_TIMER, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};