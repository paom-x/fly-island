import { useEffect, useState } from "react";
import { useTranslation } from "../../i18n";
import { formatMetric } from "./format";
import { availableMetrics, type SystemMetric, type SystemStats } from "./types";

/** How long one metric owns the line before the next one takes it. */
export const ROTATE_MS = 4_000;

/**
 * The collapsed island's system-monitor text: one metric at a time, so a long list of numbers never
 * crowds the bar. Rotation only runs while the bar is on screen (`active`); each metric keeps its
 * own reading, so a switch never shows a stale number from another metric.
 */
export function CollapsedSystemContent({
  stats,
  metrics,
  active,
  prefix,
}: {
  stats: SystemStats | null;
  metrics: readonly SystemMetric[];
  active: boolean;
  prefix: string;
}) {
  const t = useTranslation();
  const [index, setIndex] = useState(0);
  const shown = stats ? availableMetrics(stats, metrics) : [];
  useEffect(() => {
    if (!active || shown.length < 2) return;
    const timer = window.setInterval(() => setIndex((current) => current + 1), ROTATE_MS);
    return () => window.clearInterval(timer);
  }, [active, shown.length]);
  if (!stats || shown.length === 0) return null;
  const metric = shown[index % shown.length];
  const label = t.island.systemMetrics[metric];
  const value = formatMetric(stats, metric);
  return (
    <span className="island__active-task" aria-label={`${label} ${value}`}>
      <span className="island__active-task-title">{prefix}{label} {value}</span>
    </span>
  );
}
