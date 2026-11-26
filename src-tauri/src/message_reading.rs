//! Whether the island may read what a notification says, or only that one arrived.
//!
//! Two modes sit behind Settings → Messages:
//!
//! * `content` (the default) — a notification's sender and text are read and shown
//!   on the island. They only ever live in this process's memory: no history file,
//!   no log, nothing on disk, so closing the island forgets them.
//! * `unread` — only the fact that something arrived is kept. The QQ listener skips
//!   the notification's text elements (their words are never fetched from Windows)
//!   and the WeChat reader drops its summaries, so the island shows a bare
//!   "QQ · new message" instead of a sender and a line of conversation.
//!
//! Both listeners run on their own threads, so the flag is a plain atomic read on
//! every notification instead of a locked setting.

use std::sync::atomic::{AtomicBool, Ordering};

/// Content reading is the long-standing behaviour, so a fresh install starts here.
static READING_CONTENT: AtomicBool = AtomicBool::new(true);

/// True while a notification's sender and text may be read.
pub(crate) fn reads_content() -> bool {
    READING_CONTENT.load(Ordering::Relaxed)
}

/// Switches the mode; `content` false means "only unread".
fn set_reads_content(content: bool) {
    READING_CONTENT.store(content, Ordering::Relaxed);
}

/// Called by Settings → Messages whenever the reading mode changes.
#[tauri::command]
pub(crate) fn set_message_reading(content: bool) {
    set_reads_content(content);
}

macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)*
            crate::message_reading::set_message_reading,
        ] $($rest)*)
    };
}

pub(crate) use tauri_commands;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_content_by_default_and_follows_the_switch() {
        assert!(reads_content());
        set_reads_content(false);
        assert!(!reads_content());
        set_reads_content(true);
        assert!(reads_content());
    }
}
