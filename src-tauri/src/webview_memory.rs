//! Asks WebView2 to run the island's browser in its low-memory mode.
//!
//! Emptying the working set only hands pages to disk, the browser's own commit stays where it
//! was. `ICoreWebView2_19::SetMemoryUsageTargetLevel(Low)` is the switch the browser
//! understands as "this page is in the background, free what you do not need" — what Edge's
//! sleeping tabs do. `TrySuspend` would free far more, but it refuses to run unless the
//! controller reports `IsVisible: false`, and hiding the webview also hides the collapsed bar
//! the island is showing, so it is not usable here.
//!
//! The guess about the user is deliberately coarse: the bar stays on screen, so the only
//! evidence of "nobody is using this" is that the panel has not been opened for a long while.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::AppHandle;

/// How long the panel may stay unopened before the browser is asked to shrink.
const PARK_AFTER: Duration = Duration::from_secs(10 * 60);

/// When the island was last expanded, or when the app started.
static LAST_ACTIVE: Mutex<Option<Instant>> = Mutex::new(None);

/// Whether the browser is currently in low-memory mode.
static PARKED: Mutex<bool> = Mutex::new(false);

/// Records that the user is using the island, and lets the browser breathe again if it had been
/// asked to shrink.
pub(crate) fn note_activity(app: &AppHandle) {
    if let Ok(mut last) = LAST_ACTIVE.lock() {
        *last = Some(Instant::now());
    }
    set_level(app, false);
}

/// Returns how long the panel has been left alone, once that is long enough to matter.
fn left_alone_for() -> Option<Duration> {
    let last = (*LAST_ACTIVE.lock().ok()?)?;
    let idle = last.elapsed();
    (idle >= PARK_AFTER).then_some(idle)
}

/// Puts the browser into low-memory mode if the panel has been left alone long enough. Called
/// from the memory trimmer's loop, which already knows the island is collapsed.
pub(crate) fn maybe_park(app: &AppHandle) {
    if left_alone_for().is_none() {
        return;
    }
    let parked = match PARKED.lock() {
        Ok(parked) => *parked,
        Err(poisoned) => *poisoned.into_inner(),
    };
    if parked {
        return;
    }
    set_level(app, true);
}

/// Switches the island's webview between the low and the normal memory target.
fn set_level(app: &AppHandle, low: bool) {
    let mut parked = match PARKED.lock() {
        Ok(parked) => parked,
        Err(poisoned) => poisoned.into_inner(),
    };
    if *parked == low {
        return;
    }
    let Ok(window) = crate::island::main_window(app) else {
        return;
    };
    // The closure runs on the UI thread, which is where the webview controller may be used.
    let switched = window
        .with_webview(move |webview| {
            let controller = webview.controller();
            let _ = crate::win32::webview::set_low_memory(&controller, low);
        })
        .is_ok();
    if switched {
        *parked = low;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::{left_alone_for, LAST_ACTIVE, PARK_AFTER};

    #[test]
    fn a_panel_left_alone_long_enough_is_parked() {
        *LAST_ACTIVE.lock().unwrap() = Some(Instant::now() - PARK_AFTER);
        assert!(left_alone_for().is_some_and(|idle| idle >= PARK_AFTER));
    }

    #[test]
    fn a_panel_that_was_just_opened_is_left_alone() {
        *LAST_ACTIVE.lock().unwrap() = Some(Instant::now());
        assert!(left_alone_for().is_none());
    }

    #[test]
    fn a_brand_new_app_has_nothing_to_compare_against() {
        *LAST_ACTIVE.lock().unwrap() = None;
        assert!(left_alone_for().is_none());
    }
}
