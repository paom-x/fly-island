//! Fly Island's hook entries in ZCode's `cli/config.json`.
//!
//! ZCode keeps its hooks in JSON (not Claude Code's `settings.json` shape) and validates a hook's
//! stdout against a strict schema, so these entries only report the turn's phase and print
//! nothing: an empty stdout with exit code 0 leaves whatever ZCode itself would ask untouched.
//! `docs/investigations/zcode-hooks-and-approval-2026-11.md` records the measured contract.
//!
//! Only the island's own entries are managed: a user (or a plugin marketplace package) may keep
//! hooks of their own in the same file, and `hooks.enabled` may have to be true for ours to run.

use std::{fs, path::Path};

use serde_json::{Map, Value};

use super::hooks_state::HookInstallState;
use super::{
    write_text_file, AGENT_APPROVAL_SCRIPT_FILE_NAME, AGENT_SESSION_SCRIPT_FILE_NAME,
    AGENT_STATUS_SCRIPT_FILE_NAME, FLY_ISLAND_AGENT_HOOK_SIGNATURE,
};

/// ZCode's own hook chain is bounded by this too: the island's `SessionStart` walks the process
/// tree once to record which window runs the session (see fly-island-agent-session.ps1).
const SESSION_START_TIMEOUT_MS: i64 = 15_000;
/// The status hooks only rewrite one small JSON file.
const STATUS_TIMEOUT_MS: i64 = 5_000;
/// ZCode drops a hook's output that is larger than this; only the approval hook prints anything.
const MAX_OUTPUT_BYTES: i64 = 65_536;
/// The approval endpoint hands a request back after 120 s (`agent_approval::WAIT_LIMIT`) and the
/// script waits 140 s for it, so ZCode must allow the hook longer than either.
const APPROVAL_TIMEOUT_MS: i64 = 150_000;

/// One hook the island runs, as ZCode's `type: "process"` entry takes it.
struct ZcodeHook {
    event: &'static str,
    /// A regular expression over tool names; the three non-tool events carry none.
    matcher: Option<&'static str>,
    script: &'static str,
    provider_args: [&'static str; 2],
    timeout_ms: i64,
    status_message: &'static str,
    /// Installed only while the island answers approvals: see `active_hooks`.
    approval_only: bool,
}

/// The tools the island approves: the ones ZCode stops for when it asks before changes. Its alias
/// forms are matched too, so a `Write` and the `ApplyPatch` that stands for it both arrive.
const APPROVAL_MATCHER: &str = "^(Bash|Write|Edit|MultiEdit|ApplyPatch)$";

/// The events the island listens to. `PermissionRequest` is deliberately absent: ZCode ignores a
/// hook's answer there, so the island decides on `PreToolUse` instead (the investigation above
/// records the measurement).
const ZCODE_HOOKS: [ZcodeHook; 4] = [
    ZcodeHook {
        event: "SessionStart",
        matcher: None,
        script: AGENT_SESSION_SCRIPT_FILE_NAME,
        provider_args: ["zcode", "start"],
        timeout_ms: SESSION_START_TIMEOUT_MS,
        status_message: "正在记录 Fly Island 会话",
        approval_only: false,
    },
    ZcodeHook {
        event: "UserPromptSubmit",
        matcher: None,
        script: AGENT_STATUS_SCRIPT_FILE_NAME,
        provider_args: ["zcode", "running"],
        timeout_ms: STATUS_TIMEOUT_MS,
        status_message: "正在更新 Fly Island 状态",
        approval_only: false,
    },
    ZcodeHook {
        event: "Stop",
        matcher: None,
        script: AGENT_STATUS_SCRIPT_FILE_NAME,
        provider_args: ["zcode", "completed"],
        timeout_ms: STATUS_TIMEOUT_MS,
        status_message: "正在更新 Fly Island 状态",
        approval_only: false,
    },
    ZcodeHook {
        event: "PreToolUse",
        matcher: Some(APPROVAL_MATCHER),
        script: AGENT_APPROVAL_SCRIPT_FILE_NAME,
        provider_args: ["zcode", "pre-tool-use"],
        timeout_ms: APPROVAL_TIMEOUT_MS,
        status_message: "正在等待 Fly Island 审批",
        approval_only: true,
    },
];

/// The entries to write right now. Approvals are opt-in and off by default, and Z Code asks its
/// `PreToolUse` hook on every tool call, so keeping that entry installed would spend a process
/// start per call on users who never approve on the island.
fn active_hooks(approval: bool) -> impl Iterator<Item = &'static ZcodeHook> {
    ZCODE_HOOKS.iter().filter(move |hook| !hook.approval_only || approval)
}

pub(super) fn install_zcode_status_hooks(config_path: &Path, app_dir: &Path) -> Result<(), String> {
    // `~/.zcode/cli` appears the first time ZCode's CLI side runs. Without it ZCode is not
    // installed (or never launched), and the island does not create config for software the user
    // does not have.
    if !config_path.parent().is_some_and(Path::is_dir) {
        return Ok(());
    }
    let existing = fs::read_to_string(config_path).unwrap_or_default();
    let mut config = parse_config(&existing)?;
    remove_managed_zcode_hooks(&mut config);
    merge_managed_zcode_hooks(&mut config, app_dir)?;
    let text = serde_json::to_string_pretty(&config)
        .map_err(|error| format!("Failed to serialize {}: {error}", config_path.display()))?;
    write_text_file(config_path, &(text + "\n"))
}

/// ZCode's config is JSON, so a broken file is the user's to fix; overwriting it would drop their
/// MCP servers and enabled plugins.
fn parse_config(text: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    match serde_json::from_str::<Value>(text) {
        Ok(value @ Value::Object(_)) => Ok(value),
        Ok(_) => Err("ZCode's config.json is not a JSON object; the island leaves it alone".to_string()),
        Err(error) => Err(format!(
            "ZCode's config.json is not valid JSON ({error}); fix or remove it and install again"
        )),
    }
}

/// Whether an entry belongs to the island: its commands name the scripts by the shared prefix.
fn is_managed_hook(entry: &Value) -> bool {
    entry.to_string().contains(FLY_ISLAND_AGENT_HOOK_SIGNATURE)
}

/// The event map of a config that has one. A file without a `hooks` section holds nothing of the
/// island's to remove; a `hooks` value that is not an object is left as it is.
fn events_object_mut(config: &mut Value) -> Option<&mut Map<String, Value>> {
    config.get_mut("hooks")?.get_mut("events")?.as_object_mut()
}

/// Drops the island's entries from every event and the events that end up empty. Hooks of other
/// sources in the same arrays stay, and an event key the island never writes is not touched.
pub(super) fn remove_managed_zcode_hooks(config: &mut Value) {
    let Some(events) = events_object_mut(config) else {
        return;
    };
    let names = events.keys().cloned().collect::<Vec<_>>();
    for name in names {
        let Some(entries) = events.get_mut(&name).and_then(Value::as_array_mut) else {
            continue;
        };
        entries.retain(|entry| !is_managed_hook(entry));
        if entries.is_empty() {
            events.remove(&name);
        }
    }
}

/// Adds the island's entries and switches the file's hooks on. `enabled` is the one setting the
/// island has to change: ZCode runs no configured hook while it is false (the default).
fn merge_managed_zcode_hooks(config: &mut Value, app_dir: &Path) -> Result<(), String> {
    merge_managed_zcode_hooks_when(config, app_dir, crate::agent_approval::enabled())
}

/// The same, with the approval switch passed in so tests can exercise both sides of it.
fn merge_managed_zcode_hooks_when(
    config: &mut Value,
    app_dir: &Path,
    approval: bool,
) -> Result<(), String> {
    let root = config
        .as_object_mut()
        .ok_or_else(|| "ZCode's config.json is not a JSON object".to_string())?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| "ZCode's config.json has a hooks value that is not an object".to_string())?;
    hooks.insert("enabled".to_string(), Value::Bool(true));
    // Kept for the island's own hooks only; ZCode's per-event timeouts win over this one anyway.
    hooks
        .entry("maxOutputBytes")
        .or_insert_with(|| Value::from(MAX_OUTPUT_BYTES));
    let events = hooks
        .entry("events")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "ZCode's config.json has a hooks.events value that is not an object".to_string())?;

    for hook in active_hooks(approval) {
        let entries = events
            .entry(hook.event.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        let entries = entries.as_array_mut().ok_or_else(|| {
            format!("ZCode's config.json has a hooks.events.{} value that is not a list", hook.event)
        })?;
        entries.push(build_hook_entry(hook, app_dir));
    }
    Ok(())
}

fn build_hook_entry(hook: &ZcodeHook, app_dir: &Path) -> Value {
    let script_path = app_dir.join(hook.script).to_string_lossy().to_string();
    let process = serde_json::json!({
        "type": "process",
        "command": "powershell.exe",
        "args": [
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", script_path,
            hook.provider_args[0],
            hook.provider_args[1],
        ],
        "timeoutMs": hook.timeout_ms,
        "statusMessage": hook.status_message,
    });
    // ZCode's matcher is a regular expression over the tool name, so only the tool events carry
    // one; without it an entry matches every tool.
    match hook.matcher {
        Some(matcher) => serde_json::json!({ "matcher": matcher, "hooks": [process] }),
        None => serde_json::json!({ "hooks": [process] }),
    }
}

/// Current when every hook Install/Repair writes is already there. A file with none of them is
/// "not installed", even when it holds other sources' hooks.
pub(super) fn zcode_hooks_state(config_json: Option<&str>, app_dir: &Path) -> HookInstallState {
    let Some(config) = config_json.and_then(|text| serde_json::from_str::<Value>(text).ok()) else {
        return HookInstallState::NotInstalled;
    };
    let hooks = &config["hooks"];
    if !hooks.to_string().contains(FLY_ISLAND_AGENT_HOOK_SIGNATURE) {
        return HookInstallState::NotInstalled;
    }
    if hooks["enabled"] != Value::Bool(true) {
        // The entries are there but ZCode runs none of them: a repair has something to do.
        return HookInstallState::Outdated;
    }

    let events = &hooks["events"];
    let all_current = active_hooks(crate::agent_approval::enabled()).all(|hook| {
        events[hook.event]
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| *entry == build_hook_entry(hook, app_dir)))
    });
    if all_current {
        HookInstallState::Current
    } else {
        HookInstallState::Outdated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_dir() -> &'static Path {
        Path::new(r"C:\Users\me\AppData\Roaming\com.flyisland.app")
    }

    fn installed() -> Value {
        let mut config = serde_json::json!({
            "mcp": { "servers": { "magpie": { "url": "http://127.0.0.1:3425/mcp" } } },
            "plugins": { "enabledPlugins": { "github": true } },
        });
        remove_managed_zcode_hooks(&mut config);
        // Approved on the island, so the tool entry is part of what gets written.
        merge_managed_zcode_hooks_when(&mut config, app_dir(), true).expect("merge");
        config
    }

    #[test]
    fn install_keeps_the_rest_of_the_config() {
        let config = installed();
        assert_eq!(config["mcp"]["servers"]["magpie"]["url"], "http://127.0.0.1:3425/mcp");
        assert_eq!(config["plugins"]["enabledPlugins"]["github"], true);
        assert_eq!(config["hooks"]["enabled"], true);
        assert_eq!(config["hooks"]["maxOutputBytes"], MAX_OUTPUT_BYTES);
        for hook in &ZCODE_HOOKS {
            let entries = config["hooks"]["events"][hook.event].as_array().expect("event list");
            assert_eq!(entries.len(), 1, "{} holds one entry", hook.event);
            let process = &entries[0]["hooks"][0];
            assert_eq!(process["type"], "process");
            assert_eq!(process["command"], "powershell.exe");
            assert_eq!(process["timeoutMs"], hook.timeout_ms);
            assert_eq!(process["args"][5], hook.provider_args[0]);
            assert_eq!(process["args"][6], hook.provider_args[1]);
        }
        // SessionStart records the session, so it gets the session script.
        let start = &config["hooks"]["events"]["SessionStart"][0]["hooks"][0];
        assert!(start["args"][4].as_str().unwrap().ends_with(AGENT_SESSION_SCRIPT_FILE_NAME));
        let stop = &config["hooks"]["events"]["Stop"][0]["hooks"][0];
        assert!(stop["args"][4].as_str().unwrap().ends_with(AGENT_STATUS_SCRIPT_FILE_NAME));
        // The tool entry waits on the island's approval and only covers the tools it decides.
        let approve = &config["hooks"]["events"]["PreToolUse"][0];
        assert_eq!(approve["matcher"], APPROVAL_MATCHER);
        let approve = &approve["hooks"][0];
        assert!(approve["args"][4].as_str().unwrap().ends_with(AGENT_APPROVAL_SCRIPT_FILE_NAME));
        assert_eq!(approve["args"][5], "zcode");
        assert_eq!(approve["args"][6], "pre-tool-use");
        assert_eq!(approve["timeoutMs"], APPROVAL_TIMEOUT_MS);
    }

    #[test]
    fn install_is_idempotent_and_keeps_foreign_hooks() {
        let mut config = serde_json::json!({
            "hooks": {
                "enabled": true,
                "events": { "PreToolUse": [{ "matcher": "Edit", "hooks": [{ "type": "process", "command": "node", "args": ["a"] }] }] },
            },
        });
        merge_managed_zcode_hooks_when(&mut config, app_dir(), true).expect("first");
        remove_managed_zcode_hooks(&mut config);
        merge_managed_zcode_hooks_when(&mut config, app_dir(), true).expect("second");
        assert_eq!(config["hooks"]["events"]["UserPromptSubmit"].as_array().unwrap().len(), 1);
        // The foreign Edit entry stays and the island's own tool entry is added next to it.
        assert_eq!(config["hooks"]["events"]["PreToolUse"].as_array().unwrap().len(), 2);
        assert_eq!(config["hooks"]["events"]["PreToolUse"][0]["matcher"], "Edit");
        assert_eq!(config["hooks"]["events"]["PreToolUse"][1]["matcher"], APPROVAL_MATCHER);
    }

    #[test]
    fn the_approval_hook_is_left_out_while_approvals_are_off() {
        // Z Code asks this hook on every tool call, so an unused one would cost a process start per
        // call: it is written only while the island answers.
        let mut config = serde_json::json!({
            "hooks": {
                "enabled": true,
                "events": { "PreToolUse": [{ "matcher": "Edit", "hooks": [{ "type": "process", "command": "node", "args": ["a"] }] }] },
            },
        });
        merge_managed_zcode_hooks_when(&mut config, app_dir(), false).expect("merge");
        let tool_entries = config["hooks"]["events"]["PreToolUse"].as_array().expect("foreign entry kept");
        assert_eq!(tool_entries.len(), 1);
        assert_eq!(tool_entries[0]["matcher"], "Edit");
        // The status events are still written.
        assert_eq!(config["hooks"]["events"]["UserPromptSubmit"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn uninstall_leaves_an_empty_hooks_object_and_foreign_events() {
        let mut config = installed();
        config["hooks"]["events"]["PreToolUse"] = serde_json::json!([
            { "matcher": "Bash", "hooks": [{ "type": "process", "command": "node", "args": ["mimosa.js"] }] }
        ]);
        remove_managed_zcode_hooks(&mut config);
        assert!(config["hooks"]["events"].get("SessionStart").is_none());
        assert!(config["hooks"]["events"].get("UserPromptSubmit").is_none());
        assert!(config["hooks"]["events"].get("Stop").is_none());
        assert!(config["hooks"]["events"].get("PreToolUse").is_some());
        assert_eq!(config["mcp"]["servers"]["magpie"]["url"], "http://127.0.0.1:3425/mcp");
    }

    #[test]
    fn state_tracks_the_written_entries() {
        assert_eq!(zcode_hooks_state(None, app_dir()), HookInstallState::NotInstalled);
        let foreign = serde_json::json!({
            "hooks": { "enabled": true, "events": { "PreToolUse": [{ "hooks": [{ "type": "process", "command": "node" }] }] } },
        });
        assert_eq!(
            zcode_hooks_state(Some(&foreign.to_string()), app_dir()),
            HookInstallState::NotInstalled
        );

        let current = installed().to_string();
        assert_eq!(zcode_hooks_state(Some(&current), app_dir()), HookInstallState::Current);

        // Entries from an older build that named a different script are outdated.
        let mut stale = installed();
        stale["hooks"]["events"]["Stop"][0]["hooks"][0]["args"][4] =
            Value::from(r"C:\old\fly-island-agent-status-old.ps1");
        assert_eq!(
            zcode_hooks_state(Some(&stale.to_string()), app_dir()),
            HookInstallState::Outdated
        );

        // The entries are there but switched off, which is how a user turns them off.
        let mut off = installed();
        off["hooks"]["enabled"] = Value::Bool(false);
        assert_eq!(
            zcode_hooks_state(Some(&off.to_string()), app_dir()),
            HookInstallState::Outdated
        );
    }

    #[test]
    fn broken_json_is_refused_rather_than_replaced() {
        assert!(parse_config("{ not json").is_err());
        assert!(parse_config("[]").is_err());
        assert_eq!(parse_config("  ").expect("empty"), serde_json::json!({}));
    }
}
