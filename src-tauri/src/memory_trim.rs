//! Hands the memory of our background browser processes back to Windows.
//!
//! WebView2 keeps every page it has loaded resident, so the island's browser still holds
//! hundreds of megabytes while the island shows nothing but its collapsed strip. Emptying
//! the working set of those processes while the panel is closed costs a page fault or two
//! on the next hover and gives the memory back to everything else in between. Only the
//! children are trimmed: the host process keeps the window bookkeeping and pages in again
//! straight away, so trimming it buys nothing.
//!
//! How hard this works is the user's call (Settings → General → Memory, `MemoryMode`): the
//! paging-out is what makes the first hover after a long idle slower, so somebody who wants
//! the island to snap open can turn it off.

use std::{
    sync::atomic::{AtomicU8, Ordering},
    thread,
    time::Duration,
};

use tauri::{AppHandle, Manager};

use crate::{
    island,
    win32::{process_memory, process_tree},
};

/// How hard the trimmer works, as chosen in Settings → General → Memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoryMode {
    /// Hands memory back ten seconds after collapsing; the first hover may wait to page in.
    Saver,
    /// The default: half a minute of quiet, which hovering rarely notices.
    Balanced,
    /// Never trims: pages stay warm for the quickest hover, at the largest idle footprint.
    Smooth,
}

impl MemoryMode {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "saver" => Some(Self::Saver),
            "balanced" => Some(Self::Balanced),
            "smooth" => Some(Self::Smooth),
            _ => None,
        }
    }

    /// How long the island must stay collapsed between trims; `None` means never trim.
    fn interval(self) -> Option<Duration> {
        match self {
            Self::Saver => Some(Duration::from_secs(10)),
            Self::Balanced => Some(Duration::from_secs(30)),
            Self::Smooth => None,
        }
    }
}

const SAVER: u8 = 0;
const BALANCED: u8 = 1;
const SMOOTH: u8 = 2;

/// How long the smooth mode sleeps before looking at the mode again, so switching back to
/// a trimming mode takes effect without restarting the app.
const SMOOTH_RECHECK: Duration = Duration::from_secs(30);

static MODE: AtomicU8 = AtomicU8::new(BALANCED);

fn mode() -> MemoryMode {
    match MODE.load(Ordering::Relaxed) {
        SAVER => MemoryMode::Saver,
        SMOOTH => MemoryMode::Smooth,
        _ => MemoryMode::Balanced,
    }
}

/// Stores the mode the front end picked. Sent once at start-up and again on every change.
pub(crate) fn set_mode(name: &str) -> Result<(), String> {
    let mode = MemoryMode::parse(name).ok_or_else(|| format!("Unknown memory mode: {name}"))?;
    let stored = match mode {
        MemoryMode::Saver => SAVER,
        MemoryMode::Balanced => BALANCED,
        MemoryMode::Smooth => SMOOTH,
    };
    MODE.store(stored, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub(crate) fn set_memory_mode(mode: String) -> Result<(), String> {
    set_mode(&mode)
}

macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)*
            crate::memory_trim::set_memory_mode,
        ] $($rest)*)
    };
}

pub(crate) use tauri_commands;

/// Starts the trimmer. It sleeps first: a fresh start is busy with the island's first
/// frames, and nothing has accumulated to trim yet.
pub(crate) fn start(app: AppHandle) {
    // The clock that decides when the browser may be parked starts with the app, not with the
    // first hover: an island nobody ever opens should still shrink.
    crate::webview_memory::note_activity(&app);

    let worker = thread::Builder::new()
        .name("memory-trim".into())
        .spawn(move || loop {
            // A mode change lands on the next pass; the pass is short enough that nobody
            // waits long for it, and the work itself is a few system calls.
            let Some(interval) = mode().interval() else {
                thread::sleep(SMOOTH_RECHECK);
                continue;
            };
            thread::sleep(interval);
            if settled(&app) {
                trim_children();
                // The same quiet window is when the browser can be parked outright. It decides
                // for itself whether the user has been away long enough.
                crate::webview_memory::maybe_park(&app);
            }
        });
    if let Err(error) = worker {
        eprintln!("failed to start the memory trimmer: {error}");
    }
}

/// Whether nothing of ours is on screen but the collapsed island, so no window is being
/// read or animated while its pages go out to disk.
pub(crate) fn settled(app: &AppHandle) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mode_trims_at_its_own_pace() {
        assert_eq!(MemoryMode::parse("saver").and_then(MemoryMode::interval), Some(Duration::from_secs(10)));
        assert_eq!(MemoryMode::parse("balanced").and_then(MemoryMode::interval), Some(Duration::from_secs(30)));
        assert!(MemoryMode::parse("smooth").and_then(MemoryMode::interval).is_none());
    }

    #[test]
    fn a_mode_we_do_not_know_is_refused() {
        assert!(set_mode("turbo").is_err());
        assert!(set_mode("smooth").is_ok());
        assert_eq!(mode(), MemoryMode::Smooth);
        assert!(set_mode("balanced").is_ok());
        assert_eq!(mode(), MemoryMode::Balanced);
    }
}
