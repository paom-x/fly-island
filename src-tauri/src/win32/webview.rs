//! Safe wrapper around the WebView2 calls the island makes on its own browser.

use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
};
use windows::core::Interface;

/// Asks the browser behind `controller` to run in its low-memory mode, or back to normal.
///
/// Returns whether the request reached the browser. Runtimes older than the interface have no
/// memory target level at all; the island then simply keeps its usual footprint.
pub(crate) fn set_low_memory(controller: &ICoreWebView2Controller, low: bool) -> bool {
    // SAFETY: CoreWebView2 returns the webview this controller owns, and the caller keeps the
    // controller alive for the duration of the call.
    let Ok(core) = (unsafe { controller.CoreWebView2() }) else {
        return false;
    };
    let Ok(scaled) = core.cast::<ICoreWebView2_19>() else {
        return false;
    };
    let target = if low {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
    } else {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
    };
    // SAFETY: the interface call only changes this webview's own memory target.
    unsafe { scaled.SetMemoryUsageTargetLevel(target) }.is_ok()
}
