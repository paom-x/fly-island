import { useEffect, useState } from "react";
import { getSystemStats } from "./api";
import type { SystemStats } from "./types";

/** How often the counters are read: often enough to feel live, rarely enough not to be work. */
export const SYSTEM_STATS_REFRESH_MS = 2_000;

/**
 * Polls the machine's counters while `enabled`, keeping the last reading so a dropped one shows the
 * previous numbers instead of blanking the line. Nothing is polled when disabled — the collapsed
 * island asks only while one of its texts is the system monitor.
 */
export function useSystemStats(enabled: boolean, refreshMs = SYSTEM_STATS_REFRESH_MS): SystemStats | null {
  const [stats, setStats] = useState<SystemStats | null>(null);
  useEffect(() => {
    if (!enabled) {
      setStats(null);
      return;
    }
    let inFlight = false;
    let stopped = false;
    const read = () => {
      // A slow reading must not stack up behind the interval.
      if (inFlight) return;
      inFlight = true;
      getSystemStats()
        .then((next) => {
          if (!stopped) setStats(next);
        })
        .catch(() => {
          // A counter that vanished (a device was unplugged, say) leaves the last reading on
          // screen; the next tick tries again.
        })
        .finally(() => {
          inFlight = false;
        });
    };
    read();
    const timer = window.setInterval(read, refreshMs);
    return () => {
      stopped = true;
      window.clearInterval(timer);
    };
  }, [enabled, refreshMs]);
  return stats;
}
