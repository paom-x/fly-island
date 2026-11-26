/** One metric the island can show from the machine's counters. */
export type SystemMetric = "cpu" | "memory" | "network" | "gpu" | "temperature";

/** Every metric, in the order the settings page lists them. */
export const SYSTEM_METRICS: readonly SystemMetric[] = ["cpu", "memory", "network", "gpu", "temperature"];

/** One reading, as the `system_stats` command returns it. */
export type SystemStats = {
  cpuPercent: number;
  memoryPercent: number;
  memoryUsed: number;
  memoryTotal: number;
  /** Bytes per second over every non-loopback interface. */
  networkDown: number;
  networkUp: number;
  /** Absent on machines without a graphics performance counter. */
  gpuPercent: number | null;
  /** Degrees Celsius; absent on machines without thermal zones. */
  temperature: number | null;
};

/** The metrics this machine can actually show: a reading of null means no such counter. */
export function availableMetrics(stats: SystemStats, wanted: readonly SystemMetric[]): SystemMetric[] {
  return wanted.filter((metric) => {
    if (metric === "gpu") return stats.gpuPercent !== null;
    if (metric === "temperature") return stats.temperature !== null;
    return true;
  });
}
