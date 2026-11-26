//! Machine-wide counters for the island's system monitor: how busy the processor is, how much
//! memory is in use, and how many bytes went over the network.
//!
//! All three are running totals the caller differences itself (see `system_monitor`): reading a
//! total is one cheap call, while Windows' own rate counters would need a query object per metric
//! and a warm-up collection before every reading.

use windows::Win32::Foundation::FILETIME;
use windows::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfTable2, MIB_IF_TABLE2, MIB_IF_TYPE_LOOPBACK,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

/// A `FILETIME` read as one 64-bit count of 100-nanosecond ticks.
fn ticks(time: FILETIME) -> u64 {
    ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
}

/// Idle ticks and total ticks the processor has spent so far. Idle is also counted in kernel time,
/// so the caller subtracts it to get the busy share.
pub(crate) fn cpu_ticks() -> Option<(u64, u64)> {
    let mut idle = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: three distinct out-parameters owned by this function; GetSystemTimes only writes
    // into them and its failure is reported through the returned Result.
    unsafe {
        GetSystemTimes(
            Some(&mut idle as *mut FILETIME),
            Some(&mut kernel as *mut FILETIME),
            Some(&mut user as *mut FILETIME),
        )
    }
    .ok()?;
    Some((ticks(idle), ticks(kernel) + ticks(user)))
}

/// Physical memory in use and installed, in bytes.
pub(crate) fn memory_bytes() -> Option<(u64, u64)> {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: the structure is owned by this function and dwLength names its size, which is what
    // GlobalMemoryStatusEx requires of the buffer it fills.
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    Some((
        status.ullTotalPhys.saturating_sub(status.ullAvailPhys),
        status.ullTotalPhys,
    ))
}

/// Bytes received and sent over every interface except loopback, since boot.
pub(crate) fn network_bytes() -> Option<(u64, u64)> {
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: GetIfTable2 allocates the table and reports failure through the Result; a null
    // table is checked before it is dereferenced below.
    unsafe { GetIfTable2(&mut table) }
        .ok()
        .ok()?;
    if table.is_null() {
        return None;
    }
    let mut received = 0u64;
    let mut sent = 0u64;
    // SAFETY: GetIfTable2 allocated `NumEntries` rows at Table; the slice lives until the table
    // is freed in the same block, and the rows are only read.
    unsafe {
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        for row in rows {
            // Loopback traffic never leaves the machine, so counting it would show a transfer that
            // is not happening on any wire.
            if row.Type == MIB_IF_TYPE_LOOPBACK {
                continue;
            }
            received = received.saturating_add(row.InOctets);
            sent = sent.saturating_add(row.OutOctets);
        }
        FreeMibTable(table as *const core::ffi::c_void);
    }
    Some((received, sent))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filetime_reads_as_one_count() {
        let time = FILETIME {
            dwLowDateTime: 1,
            dwHighDateTime: 2,
        };
        assert_eq!(ticks(time), 1 + (2 << 32));
    }

    #[test]
    fn the_counters_this_machine_has_are_readable() {
        // These never fail on a normal desktop; a failure here means the wrapper passes the wrong
        // buffer, which no other test would catch.
        assert!(cpu_ticks().is_some_and(|(idle, total)| idle <= total));
        assert!(memory_bytes().is_some_and(|(used, total)| used <= total && total > 0));
        assert!(network_bytes().is_some());
    }
}
