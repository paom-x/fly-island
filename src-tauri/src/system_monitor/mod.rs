//! The island's system monitor: processor load, memory in use, network throughput, and — where
//! the machine exposes the counters — graphics load and package temperature.
//!
//! The counters read by `win32/sysinfo.rs` are running totals since boot, so a reading only means
//! something relative to the previous one: every call differences against the last sample this
//! module kept and also remembers the new one. That makes the command idempotent in cost — the
//! front end may poll it as often as its own refresh interval, and each call is one cheap Win32
//! read per metric rather than a background thread of its own.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::win32::sysinfo;

pub(crate) mod graphics;

/// What the front end renders: percentages 0–100 and byte counts. `gpuPercent` and `temperature`
/// are absent on machines whose counters are missing, which is how the island knows to hide them
/// instead of showing a zero it made up.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemStats {
    pub(crate) cpu_percent: f64,
    pub(crate) memory_percent: f64,
    pub(crate) memory_used: u64,
    pub(crate) memory_total: u64,
    /// Bytes per second over all non-loopback interfaces.
    pub(crate) network_down: u64,
    pub(crate) network_up: u64,
    pub(crate) gpu_percent: Option<f64>,
    /// Degrees Celsius.
    pub(crate) temperature: Option<f64>,
}

/// The totals the previous reading ended at, and when it was taken.
#[derive(Debug, Clone, Copy)]
struct Sample {
    idle_ticks: u64,
    total_ticks: u64,
    received: u64,
    sent: u64,
    at: Instant,
}

static LAST: Mutex<Option<Sample>> = Mutex::new(None);

/// Busy share of the ticks spent between two readings, as a percentage. `None` when the totals did
/// not advance (two reads inside the same tick), because a share of nothing is not zero — it is
/// unknown, and the previous reading is a better answer than a dip to 0%.
fn cpu_percent(previous: (u64, u64), current: (u64, u64)) -> Option<f64> {
    let total = current.1.checked_sub(previous.1)?;
    if total == 0 {
        return None;
    }
    let idle = current.0.saturating_sub(previous.0);
    // Kernel time includes idle time, so `total` already counts idle; the busy part is the rest.
    let busy = total.saturating_sub(idle);
    Some((busy as f64 / total as f64 * 100.0).clamp(0.0, 100.0))
}

/// Bytes per second across the time between two readings, rounded down.
fn per_second(bytes: u64, elapsed: Duration) -> u64 {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        return 0;
    }
    (bytes as f64 / seconds) as u64
}

/// One reading of every counter the island shows.
#[tauri::command]
pub(crate) fn system_stats() -> Result<SystemStats, String> {
    let now = Instant::now();
    let (idle_ticks, total_ticks) = sysinfo::cpu_ticks().ok_or("读不到处理器占用。")?;
    let (used, total) = sysinfo::memory_bytes().ok_or("读不到内存占用。")?;
    let (received, sent) = sysinfo::network_bytes().ok_or("读不到网络流量。")?;

    let previous = {
        let mut last = LAST
            .lock()
            .map_err(|_| "系统监控的采样状态被中断。".to_string())?;
        last.replace(Sample {
            idle_ticks,
            total_ticks,
            received,
            sent,
            at: now,
        })
    };

    let (cpu, network_down, network_up) = match previous {
        Some(sample) => {
            let elapsed = now.saturating_duration_since(sample.at);
            (
                cpu_percent((sample.idle_ticks, sample.total_ticks), (idle_ticks, total_ticks))
                    .unwrap_or(0.0),
                per_second(received.saturating_sub(sample.received), elapsed),
                per_second(sent.saturating_sub(sample.sent), elapsed),
            )
        }
        // The first call only establishes the baseline: it has nothing to difference against, so
        // it reports zero traffic rather than everything since boot.
        None => (0.0, 0, 0),
    };

    Ok(SystemStats {
        cpu_percent: cpu,
        memory_percent: if total == 0 {
            0.0
        } else {
            used as f64 / total as f64 * 100.0
        },
        memory_used: used,
        memory_total: total,
        network_down,
        network_up,
        gpu_percent: crate::system_monitor::graphics::gpu_percent(),
        temperature: crate::system_monitor::graphics::temperature(),
    })
}

/// Hands the command list to `lib.rs`'s accumulating macro.
macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)*
            crate::system_monitor::system_stats,
        ] $($rest)*)
    };
}
pub(crate) use tauri_commands;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_busy_share_leaves_idle_out() {
        // Half of the new ticks were idle, so half of the processor was busy.
        assert_eq!(cpu_percent((100, 200), (150, 300)), Some(50.0));
        assert_eq!(cpu_percent((100, 200), (200, 300)), Some(0.0));
        assert_eq!(cpu_percent((100, 200), (100, 300)), Some(100.0));
    }

    #[test]
    fn a_reading_that_did_not_advance_says_nothing() {
        // Two reads inside the same tick must not be reported as an idle machine.
        assert_eq!(cpu_percent((100, 200), (100, 200)), None);
        // A counter that went backwards (it should not) is not a share either.
        assert_eq!(cpu_percent((100, 300), (100, 200)), None);
    }

    #[test]
    fn traffic_is_spread_over_the_time_between_readings() {
        assert_eq!(per_second(2_000, Duration::from_secs(2)), 1_000);
        assert_eq!(per_second(0, Duration::from_secs(5)), 0);
        assert_eq!(per_second(1_000, Duration::ZERO), 0);
    }
}

