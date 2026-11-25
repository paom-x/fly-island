//! Parent process ids of every running process, for walking from a process up to
//! the program that started it (an agent session's terminal or editor).

use std::{collections::HashMap, mem::size_of};

use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

use super::process::ProcessHandle;

/// Process id -> parent process id, from one snapshot of all processes. Empty when the
/// snapshot cannot be taken. A parent id may belong to a process that has since exited,
/// or been reused by a newer one; callers check start times where that matters.
pub(crate) fn parent_process_ids() -> HashMap<u32, u32> {
    let mut parents = HashMap::new();
    // SAFETY: a process snapshot takes no pointers; the returned handle is owned by
    // ProcessHandle, which closes it (CloseHandle closes snapshot handles too).
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }).map(ProcessHandle) else {
        return parents;
    };
    let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
    // SAFETY: the snapshot handle is live and `entry` is a PROCESSENTRY32W with dwSize set.
    let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
    while more {
        parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
        // SAFETY: as for Process32FirstW.
        more = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
    }
    parents
}

/// Every process that descends from `root_id`, root excluded, from the parent map above.
/// A process whose parent has exited is not a descendant of anything, which is why the
/// map is walked instead of reading parent ids one by one on demand.
pub(crate) fn descendants_of(root_id: u32, parents: &HashMap<u32, u32>) -> Vec<u32> {
    let mut descendants = Vec::new();
    let mut walked = vec![root_id];
    while let Some(parent) = walked.pop() {
        for (&process_id, &process_parent) in parents {
            if process_parent == parent && process_id != root_id && !descendants.contains(&process_id) {
                descendants.push(process_id);
                walked.push(process_id);
            }
        }
    }
    descendants
}

/// Ids of the running processes whose executable file name is `exe_name` ("codex.exe"), matched
/// case-insensitively. Empty when the snapshot cannot be taken.
pub(crate) fn process_ids_named(exe_name: &str) -> Vec<u32> {
    let mut ids = Vec::new();
    // SAFETY: as in parent_process_ids.
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }).map(ProcessHandle) else {
        return ids;
    };
    let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
    // SAFETY: the snapshot handle is live and `entry` is a PROCESSENTRY32W with dwSize set.
    let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
    while more {
        let length = entry.szExeFile.iter().position(|&unit| unit == 0).unwrap_or(entry.szExeFile.len());
        if String::from_utf16_lossy(&entry.szExeFile[..length]).eq_ignore_ascii_case(exe_name) {
            ids.push(entry.th32ProcessID);
        }
        // SAFETY: as for Process32FirstW.
        more = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descendants_walk_down_from_the_root_only() {
        let parents = HashMap::from([(10, 1), (11, 10), (12, 11), (13, 1), (14, 12)]);
        let mut descendants = descendants_of(10, &parents);
        descendants.sort();
        assert_eq!(descendants, vec![11, 12, 14]);
        assert!(descendants_of(10, &HashMap::new()).is_empty());
    }

    #[test]
    fn the_current_process_is_listed_with_its_parent() {
        let parents = parent_process_ids();
        assert!(parents.get(&std::process::id()).is_some_and(|parent| *parent != 0));
    }

    #[test]
    fn the_current_process_is_found_by_its_file_name() {
        let exe = std::env::current_exe().ok().and_then(|path| path.file_name().map(|name| name.to_string_lossy().to_uppercase()));
        let exe = exe.unwrap_or_default();
        assert!(process_ids_named(&exe).contains(&std::process::id()));
        assert!(process_ids_named("fly-island-surely-not-running.exe").is_empty());
    }
}
