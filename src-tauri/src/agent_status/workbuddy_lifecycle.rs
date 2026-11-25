//! When WorkBuddy's usage is refreshed in the background, and when its hidden browser goes away.

use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tokio::{sync::Notify, time};

use super::workbuddy_connection::{
    Connection, WINDOW_LABEL, connection, maintenance_at, refresh_and_publish,
};

/// How long after launch the first background refresh may run, so it never competes with startup.
const STARTUP_DELAY: Duration = Duration::from_secs(30);
/// How long the hidden browser may idle after a check before it is closed again. Its own WebView2
/// environment costs hundreds of megabytes, and only a check every few minutes needs it.
pub(super) const IDLE_RELEASE: Duration = Duration::from_secs(30);
/// A webview's browser processes are not gone the moment `destroy` returns; rebuilding straight away
/// can fail on a locked profile directory, so the next build waits this long after a release.
pub(super) const REBUILD_COOLDOWN: Duration = Duration::from_secs(5);
/// Woken up whenever something that decides the next refresh time changed.
pub(super) static SCHEDULE_CHANGED: Notify = Notify::const_new();

/// Keeps the cached usage fresh and closes the hidden browser while nothing needs it.
pub(crate) fn init(app: &AppHandle) {
    let app = app.clone();
    let startup_at = Instant::now() + STARTUP_DELAY;
    tauri::async_runtime::spawn(async move {
        loop {
            let next = {
                let mut connection = connection().lock().await;
                // Close first: a release frees the profile for the refresh that may follow it.
                if connection
                    .destroy_at
                    .is_some_and(|at| at <= Instant::now())
                {
                    release_window(&app, &mut connection);
                }
                if maintenance_at(&app, &connection, startup_at)
                    .is_some_and(|at| at <= Instant::now())
                {
                    refresh_and_publish(&app, &mut connection, true).await;
                }
                earliest(
                    maintenance_at(&app, &connection, startup_at),
                    connection.destroy_at,
                )
            };
            if let Some(at) = next {
                let _ = time::timeout(
                    at.saturating_duration_since(Instant::now()),
                    SCHEDULE_CHANGED.notified(),
                )
                .await;
            } else {
                SCHEDULE_CHANGED.notified().await;
            }
        }
    });
}

/// Closes the hidden browser once nothing needs it, so its WebView2 processes stop costing memory.
/// Only ever called with the connection locked, which keeps a sign-in or a check from running under us.
fn release_window(app: &AppHandle, connection: &mut Connection) {
    connection.destroy_at = None;
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        return;
    };
    if connection.signing_in || connection.finish_login || window.is_visible().unwrap_or(true) {
        // Still in use or in front of the user; look again after another idle period.
        connection.destroy_at = Some(Instant::now() + IDLE_RELEASE);
        return;
    }
    if window.destroy().is_ok() {
        connection.window_released(Instant::now());
    }
}

/// Waits out the release cooldown before the caller builds a new window. User actions come through
/// here so a click just after an idle release does not hit the closing browser's profile.
pub(super) async fn wait_for_rebuild(connection: &Connection) {
    let Some(released) = connection.released_ago() else {
        return;
    };
    if let Some(remaining) = REBUILD_COOLDOWN.checked_sub(released) {
        time::sleep(remaining).await;
    }
}

/// The nearer of the next check and the next release; either alone when only one is planned.
fn earliest(refresh: Option<Instant>, release: Option<Instant>) -> Option<Instant> {
    match (refresh, release) {
        (Some(refresh), Some(release)) => Some(refresh.min(release)),
        (refresh, release) => refresh.or(release),
    }
}

#[cfg(test)]
mod tests {
    use super::super::workbuddy_connection::rebuild_allowed;
    use super::*;

    #[test]
    fn the_scheduler_wakes_for_whichever_comes_first() {
        let now = Instant::now();
        let later = now + Duration::from_secs(60);
        assert_eq!(earliest(Some(later), Some(now)), Some(now));
        assert_eq!(earliest(Some(later), None), Some(later));
        assert_eq!(earliest(None, Some(now)), Some(now));
        assert_eq!(earliest(None, None), None);
    }

    #[test]
    fn a_browser_that_was_never_released_is_rebuilt_at_once() {
        // The first check of a session must not be held back by a cooldown for a closed browser.
        let connection = Connection::default();
        assert_eq!(connection.released_ago(), None);
        assert!(rebuild_allowed(&connection));
    }
}
