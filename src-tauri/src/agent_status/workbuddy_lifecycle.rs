//! When WorkBuddy's usage is refreshed in the background, and what wakes the scheduler up.

use std::time::{Duration, Instant};
use tauri::AppHandle;
use tokio::{sync::Notify, time};

use super::workbuddy_connection::{connection, maintenance_at, refresh_and_publish};

/// How long after launch the first background refresh may run, so it never competes with startup.
const STARTUP_DELAY: Duration = Duration::from_secs(30);
/// Woken up whenever something that decides the next refresh time changed.
pub(super) static SCHEDULE_CHANGED: Notify = Notify::const_new();

/// Keeps the cached usage fresh: sleeps until the next moment worth checking, then refreshes once.
pub(crate) fn init(app: &AppHandle) {
    let app = app.clone();
    let startup_at = Instant::now() + STARTUP_DELAY;
    tauri::async_runtime::spawn(async move {
        loop {
            let next = {
                let mut connection = connection().lock().await;
                if maintenance_at(&app, &connection, startup_at)
                    .is_some_and(|at| at <= Instant::now())
                {
                    refresh_and_publish(&app, &mut connection, true).await;
                }
                maintenance_at(&app, &connection, startup_at)
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
