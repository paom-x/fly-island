//! Z Code CLI sessions, read from its SQLite store (`~/.zcode/cli/db/db.sqlite`). A `session` row
//! carries the title, the folder and how its last turn ended; its newest user message is the
//! prompt, and the newest assistant message that has a text part is the answer. An interrupted
//! turn leaves a synthetic `[Request interrupted by user]` message behind, which is skipped.
//!
//! `session_runtime_presence` holds one row per client that has a session open, and the rows
//! outlive the clients, so a session only counts as running while its process id is still alive.

use std::{collections::HashSet, path::{Path, PathBuf}};

use rusqlite::{params, Connection};
use serde_json::Value;

use super::{
    markers::AGENT_RUNNING_STALE_AFTER_MS, session_host::SessionHost, sessions::AgentSessionSummary,
    sqlite_read,
    title::{folder_title, shorten},
};

const MAX_SESSIONS: usize = 12;
/// How many of a session's newest messages are parsed to find its prompt and answer: both can sit
/// a few messages back, behind synthetic markers and steps without text.
const MESSAGE_SCAN: usize = 30;
/// Text parts are fetched for at most this many of those messages.
const TEXT_SCAN: usize = 3;

fn database_path(home: &Path) -> PathBuf {
    home.join(".zcode").join("cli").join("db").join("db.sqlite")
}

/// Sessions a live Z Code client reports as open. Windows reuses process ids, so an id that now
/// names an unrelated process can pass this check; the damage is bounded because such a session
/// only reads as running while it is also fresh, and turns `stale` once it goes quiet.
fn running_sessions(connection: &Connection) -> HashSet<String> {
    let Ok(mut statement) = connection.prepare("select session_id, process_id from session_runtime_presence") else {
        return HashSet::new();
    };
    let Ok(rows) = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
    }) else {
        return HashSet::new();
    };
    rows.flatten()
        .filter(|(_, process_id)| process_id.is_some_and(|id| u32::try_from(id).is_ok_and(alive)))
        .map(|(session, _)| session)
        .collect()
}

fn alive(process_id: u32) -> bool {
    crate::win32::process::image_path(process_id).is_some()
}

struct Message {
    id: String,
    data: Value,
}

fn newest_messages(connection: &Connection, session: &str) -> Vec<Message> {
    let sql = "select id, data from message where session_id = ?1 \
               order by time_updated desc, sequence desc limit ?2";
    let Ok(mut statement) = connection.prepare(sql) else { return Vec::new() };
    let Ok(rows) = statement.query_map(params![session, MESSAGE_SCAN as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return Vec::new();
    };
    rows.flatten()
        .filter_map(|(id, data)| Some(Message { id, data: serde_json::from_str(&data).ok()? }))
        .collect()
}

/// The text of one message's parts, newest first when the answer is wanted and oldest first for a
/// prompt. A step's parts precede its text, so insertion order alone says which one is the answer.
fn parts_text(connection: &Connection, message_id: &str, newest_first: bool) -> Option<String> {
    let order = if newest_first { "desc" } else { "asc" };
    let sql = format!("select data from part where message_id = ?1 order by sequence {order} limit 40");
    let mut statement = connection.prepare(&sql).ok()?;
    // The rows borrow the statement, so read them out before looking for the text part.
    let data = statement
        .query_map(params![message_id], |row| row.get::<_, String>(0))
        .ok()?
        .flatten()
        .collect::<Vec<_>>();
    data.iter()
        .filter_map(|data| serde_json::from_str::<Value>(data).ok())
        .find(|part| part["type"] == "text")
        .and_then(|part| part["text"].as_str().map(str::to_string))
}

#[derive(Default)]
struct Turn {
    prompt: Option<String>,
    prompt_at: Option<i64>,
    reply: Option<String>,
    model: Option<String>,
}

fn read_turn(connection: &Connection, session: &str) -> Turn {
    let mut turn = Turn::default();
    let mut replies_left = TEXT_SCAN;
    let mut prompts_left = TEXT_SCAN;
    for message in newest_messages(connection, session) {
        match message.data["role"].as_str().unwrap_or_default() {
            "assistant" => {
                if turn.model.is_none() {
                    turn.model = message.data["modelId"].as_str().map(str::to_string);
                }
                if turn.reply.is_none() && replies_left > 0 {
                    replies_left -= 1;
                    turn.reply = parts_text(connection, &message.id, true).and_then(|text| shorten(text.trim(), 200));
                }
            }
            "user" => {
                if turn.model.is_none() {
                    turn.model = message.data["modelSelection"]["modelId"].as_str().map(str::to_string);
                }
                // Interrupt markers are shown to the model, not typed by the user.
                let synthetic = message.data["synthetic"].as_bool().unwrap_or(false);
                if turn.prompt.is_none() && !synthetic && prompts_left > 0 {
                    prompts_left -= 1;
                    turn.prompt = parts_text(connection, &message.id, false).and_then(|text| shorten(text.trim(), 80));
                    turn.prompt_at = turn.prompt.as_ref().and(message.data["time"]["created"].as_i64());
                }
            }
            _ => {}
        }
    }
    turn
}

/// How a session's newest turn ended. Z Code keeps it in `last_outcome`, and an interrupted turn
/// (the user stopped it) is over, not failed.
fn phase(running: bool, outcome: Option<&str>, updated_at: i64, now: i64) -> &'static str {
    if running {
        return if now.saturating_sub(updated_at) > AGENT_RUNNING_STALE_AFTER_MS { "stale" } else { "running" };
    }
    if outcome == Some("failed") {
        "failed"
    } else {
        "completed"
    }
}

pub(super) fn list_sessions(home: &Path, now: i64, window_ms: i64) -> Vec<AgentSessionSummary> {
    let path = database_path(home);
    let Some(connection) = sqlite_read::open(&path) else { return Vec::new() };
    let running = running_sessions(&connection);
    let sql = "select id, title, directory, time_updated, last_outcome from session \
               where time_deleted is null and time_archived is null and task_type <> 'subagent_child' \
               and time_updated >= ?1 order by time_updated desc limit ?2";
    let Ok(mut statement) = connection.prepare(sql) else { return Vec::new() };
    let Ok(rows) = statement.query_map(params![now.saturating_sub(window_ms), MAX_SESSIONS as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    }) else {
        return Vec::new();
    };
    rows.flatten()
        .map(|(id, title, directory, updated_at, outcome)| {
            let turn = read_turn(&connection, &id);
            let title = title
                .map(|title| title.trim().to_string())
                .filter(|title| !title.is_empty())
                .or_else(|| turn.prompt.clone())
                .or_else(|| directory.as_deref().and_then(folder_title));
            AgentSessionSummary {
                title,
                host: Some(SessionHost { name: "Z Code".to_string(), exe: None, kind: "terminal" }),
                turn_started_at: turn.prompt_at,
                owner_prompt_at: turn.prompt_at,
                last_prompt: turn.prompt,
                last_message: turn.reply,
                model: turn.model,
                cwd: directory,
                phase: phase(running.contains(&id), outcome.as_deref(), updated_at, now),
                updated_at,
                ..AgentSessionSummary::blank("zcode", id)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory store");
        connection
            .execute_batch(
                "create table message (id text, session_id text, time_updated integer, sequence integer, data text);
                 create table part (message_id text, sequence integer, data text);
                 create table session_runtime_presence (session_id text, process_id integer);",
            )
            .expect("schema");
        connection
    }

    fn add(connection: &Connection, id: &str, session: &str, at: i64, data: &str, parts: &[&str]) {
        connection
            .execute(
                "insert into message (id, session_id, time_updated, sequence, data) values (?1, ?2, ?3, 0, ?4)",
                params![id, session, at, data],
            )
            .expect("message");
        for (index, part) in parts.iter().enumerate() {
            connection
                .execute(
                    "insert into part (message_id, sequence, data) values (?1, ?2, ?3)",
                    params![id, index as i64, part],
                )
                .expect("part");
        }
    }

    #[test]
    fn the_answer_is_the_newest_text_and_interrupt_markers_are_skipped() {
        let connection = store();
        add(&connection, "m1", "s", 100, r#"{"role": "user", "time": {"created": 100}, "modelSelection": {"modelId": "auto/deepseek-flash"}}"#,
            &[r#"{"type": "text", "text": "fix the tests"}"#]);
        add(&connection, "m2", "s", 200, r#"{"role": "assistant", "modelId": "auto/deepseek-flash"}"#,
            &[r#"{"type": "step-start"}"#, r#"{"type": "text", "text": "All green."}"#]);
        add(&connection, "m3", "s", 300, r#"{"role": "user", "synthetic": true, "metadata": {"source": "interrupt_marker"}}"#,
            &[r#"{"type": "text", "text": "[Request interrupted by user]"}"#]);
        add(&connection, "m4", "s", 400, r#"{"role": "assistant", "modelId": "auto/deepseek-flash"}"#,
            &[r#"{"type": "step-start"}"#]);
        let turn = read_turn(&connection, "s");
        assert_eq!(turn.prompt.as_deref(), Some("fix the tests"));
        assert_eq!(turn.prompt_at, Some(100));
        assert_eq!(turn.reply.as_deref(), Some("All green."));
        assert_eq!(turn.model.as_deref(), Some("auto/deepseek-flash"));
    }

    #[test]
    fn a_session_is_running_only_while_its_client_lives() {
        let connection = store();
        connection
            .execute_batch(&format!(
                "insert into session_runtime_presence values ('live', {}); insert into session_runtime_presence values ('gone', 4);",
                std::process::id()
            ))
            .expect("presence");
        let running = running_sessions(&connection);
        assert!(running.contains("live"));
        assert!(!running.contains("gone"));
    }

    #[test]
    fn a_live_turn_goes_stale_when_its_session_stops_being_touched() {
        assert_eq!(phase(true, Some("completed"), 1_000, 2_000), "running");
        assert_eq!(
            phase(true, Some("completed"), 1_000, 1_000 + AGENT_RUNNING_STALE_AFTER_MS + 1),
            "stale"
        );
        assert_eq!(phase(false, Some("completed"), 1_000, 2_000), "completed");
        assert_eq!(phase(false, Some("failed"), 1_000, 2_000), "failed");
        assert_eq!(phase(false, Some("interrupted"), 1_000, 2_000), "completed");
    }

    /// Reads this machine's Z Code store, which the unit tests above cannot cover; run with
    /// `cargo test zcode_sessions -- --ignored --nocapture` after Z Code changes its schema.
    #[test]
    #[ignore = "只读本机真实的 Z Code 会话库，手动运行"]
    fn reads_this_machines_sessions() {
        let home = crate::system::windows_home_dir().expect("home directory");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as i64)
            .expect("clock after the epoch");
        let sessions = list_sessions(&home, now, 7 * 24 * 60 * 60 * 1000);
        for session in &sessions {
            println!(
                "{} | {:?} | {:?} | {} | {} | {:?}",
                session.session_id, session.title, session.model, session.phase, session.updated_at,
                session.last_message
            );
        }
        assert!(sessions.iter().all(|session| session.provider == "zcode"));
    }
}
