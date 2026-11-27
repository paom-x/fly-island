import { useEffect, useRef, useState } from "react";
import { getSystemStats } from "./api";
import { SYSTEM_METRICS, type SystemMetric, type SystemStats } from "./types";

/** How often the expanded page takes a reading; the collapsed bar uses two seconds too. */
export const SYSTEM_HISTORY_REFRESH_MS = 2_000;

/** Readings kept per metric: two minutes at two seconds apiece. */
export const SYSTEM_HISTORY_POINTS = 60;

export type SystemHistory = Record<SystemMetric, number[]>;

export type SystemHistoryState = {
  stats: SystemStats | null;
  history: SystemHistory;
};

function readingOf(stats: SystemStats, metric: SystemMetric): number | null {
  switch (metric) {
    case "cpu":
      return stats.cpuPercent;
    case "memory":
      return stats.memoryPercent;
    case "network":
      return stats.networkDown;
    case "gpu":
      return stats.gpuPercent;
    case "temperature":
      return stats.temperature;
  }
}

const EMPTY_HISTORY: SystemHistory = { cpu: [], memory: [], network: [], gpu: [], temperature: [] };

/**
 * Readings for the machine page: the newest reading plus the last two minutes of each
 * counter. Nothing runs while the page is closed, so a closed page costs no polling.
 */
export function useSystemHistory(active: boolean): SystemHistoryState {
  const [state, setState] = useState<SystemHistoryState>({ stats: null, history: EMPTY_HISTORY });
  const inFlight = useRef(false);
  const stopped = useRef(false);

  useEffect(() => {
    stopped.current = false;
    // A closed page forgets its readings: they age within seconds and would be a lie later.
    if (!active) {
      setState({ stats: null, history: EMPTY_HISTORY });
      return () => { stopped.current = true; };
    }
    async function read() {
      if (inFlight.current) return;
      inFlight.current = true;
      try {
        const stats = await getSystemStats();
        if (stopped.current) return;
        setState((previous) => {
          const history = { ...previous.history };
          for (const metric of SYSTEM_METRICS) {
            const value = readingOf(stats, metric);
            // A counter this machine cannot take keeps its older points out of the chart.
            history[metric] = value === null
              ? []
              : [...history[metric], value].slice(-SYSTEM_HISTORY_POINTS);
          }
          return { stats, history };
        });
      } catch {
        // A failed reading keeps the last one on screen; the next tick tries again.
      } finally {
        inFlight.current = false;
      }
    }
    void read();
    const timer = window.setInterval(() => { void read(); }, SYSTEM_HISTORY_REFRESH_MS);
    return () => {
      stopped.current = true;
      window.clearInterval(timer);
    };
  }, [active]);

  return state;
}
