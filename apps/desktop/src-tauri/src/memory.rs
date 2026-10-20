//! Lets WebView2 release memory while the window is minimized.
//!
//! WebView2's `MemoryUsageTargetLevel::Low` trims the browser processes'
//! working sets; Tauri does not expose it, so this sets it through the
//! WebView2 COM interface. Restoring the window switches back to normal.

/// Applies the memory target for `minimized`. Errors are ignored: older
/// WebView2 runtimes without `ICoreWebView2_19` simply keep normal behaviour.
#[cfg(windows)]
pub fn set_minimized(window: &tauri::WebviewWindow, minimized: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows::core::Interface;

    let level = if minimized {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
    } else {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
    };
    let _ = window.with_webview(move |webview| {
        // SAFETY: `with_webview` runs this on the WebView2 UI thread with a live
        // controller, which is the threading model these COM calls require; the
        // interface pointers are reference-counted and dropped before returning.
        #[allow(unsafe_code)]
        unsafe {
            if let Ok(core) = webview.controller().CoreWebView2() {
                if let Ok(core) = core.cast::<ICoreWebView2_19>() {
                    let _ = core.SetMemoryUsageTargetLevel(level);
                }
            }
        }
    });
}

#[cfg(not(windows))]
pub fn set_minimized(_window: &tauri::WebviewWindow, _minimized: bool) {}
