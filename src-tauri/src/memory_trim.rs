//! Hands the memory of our background browser processes back to Windows.
//!
//! WebView2 keeps every page it has loaded resident, so the island's browser still holds
//! hundreds of megabytes while the island shows nothing but its collapsed strip. Emptying
//! the working set of those processes while the panel is closed costs a page fault or two
//! on the next hover and gives the memory back to everything else in between. Only the
//! children are trimmed: the host process keeps the window bookkeeping and pages in again
//! straight away, so trimming it buys nothing.

use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};

use crate::{
    island,
    win32::{process_memory, process_tree},
};

/// How long the island must stay collapsed before a trim, and the gap between trims.
/// Long enough that paging out is not undone by the next frame, short enough that an idle
/// island settles within half a minute.
const TRIM_INTERVAL: Duration = Duration::from_secs(30);

/// Starts the trimmer. It sleeps first: a fresh start is busy with the island's first
/// frames, and nothing has accumulated to trim yet.
pub(crate) fn start(app: AppHandle) {
    let worker = thread::Builder::new()
        .name("memory-trim".into())
        .spawn(move || loop {
            thread::sleep(TRIM_INTERVAL);
            if settled(&app) {
                trim_children();
            }
        });
    if let Err(error) = worker {
        eprintln!("failed to start the memory trimmer: {error}");
    }
}

/// Whether nothing of ours is on screen but the collapsed island, so no window is being
/// read or animated while its pages go out to disk.
fn settled(app: &AppHandle) -> bool {
    if !island::is_collapsed() {
        return false;
    }
    app.webview_windows().iter().all(|(label, window)| {
        // A window we cannot ask about is treated as visible: better to skip a trim than
        // to page in front of someone who is reading the window.
        label == island::WINDOW_LABEL || !window.is_visible().unwrap_or(true)
    })
}

/// Empties the working set of every process this one started (the WebView2 children).
fn trim_children() {
    let parents = process_tree::parent_process_ids();
    for process_id in process_tree::descendants_of(std::process::id(), &parents) {
        process_memory::trim_working_set(process_id);
    }
}
