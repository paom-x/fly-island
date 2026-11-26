import type { SystemMetric, SystemStats } from "./types";

/**
 * A reading as wide as any real one, per metric: the collapsed island sizes its window once for
 * the widest ticked counter, so the rotations never resize the native window.
 */
export const METRIC_SAMPLE_VALUE: Record<SystemMetric, string> = {
  cpu: "100%",
  memory: "100%",
  network: "↓99 MB/s ↑99 MB/s",
  gpu: "100%",
  temperature: "99°C",
};

/**
 * A byte rate for the collapsed island: one decimal below ten so a quiet connection still moves,
 * whole numbers above it, and 0 B/s when a reading is missing or nothing came through.
 */
export function formatRate(bytesPerSecond: number): string {
  if (!Number.isFinite(bytesPerSecond) || bytesPerSecond <= 0) return "0 B/s";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytesPerSecond;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const shown = value >= 10 || unit === 0 ? Math.round(value) : Math.round(value * 10) / 10;
  return `${shown} ${units[unit]}/s`;
}

/** The value half of one metric's line, without its label. */
export function formatMetric(stats: SystemStats, metric: SystemMetric): string {
  switch (metric) {
    case "cpu":
      return `${Math.round(stats.cpuPercent)}%`;
    case "memory":
      return `${Math.round(stats.memoryPercent)}%`;
    case "network":
      // Down first: that is the direction a user waiting on a page cares about.
      return `↓${formatRate(stats.networkDown)} ↑${formatRate(stats.networkUp)}`;
    case "gpu":
      return stats.gpuPercent === null ? "" : `${Math.round(stats.gpuPercent)}%`;
    case "temperature":
      return stats.temperature === null ? "" : `${Math.round(stats.temperature)}°C`;
  }
}
