import { useCallback, useEffect, useState } from "react";
import { Activity, Bot, ListTodo, MicVocal } from "lucide-react";
import { ReorderList } from "../../components/reorder/ReorderList";
import { SettingsChips } from "../../components/settings/SettingsChips";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { SettingsToggleRow } from "../../components/settings/SettingsToggleRow";
import { useTranslation } from "../../i18n";
import { getSystemStats } from "../system-monitor/api";
import { availableMetrics, SYSTEM_METRICS, type SystemMetric, type SystemStats } from "../system-monitor/types";
import type { CollapsedItem, IslandSettings } from "./types";

const ICONS = { agent: Bot, lyrics: MicVocal, todo: ListTodo, system: Activity } as const;

/**
 * A graphics counter answers with nothing on its very first read (the query is still collecting a
 * sample), so one reading is not enough to decide a counter is missing from this machine.
 */
const PROBE_ATTEMPTS = 3;
const PROBE_INTERVAL_MS = 1_200;

/**
 * Appearance → Collapsed island: the order in which a running agent, the lyric line, a todo and
 * the machine's counters claim the collapsed island's text (drag a row, or use the arrow keys on
 * its handle), whether the playing song's cover shows there, and which counters the text shows.
 */
export function CollapsedIslandSection({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  const text = useTranslation().settings.appearance.collapsedIsland;
  const order = settings.collapsedPriority;
  // Which counters this machine can take: a machine's counter set does not change while the
  // settings window is open, so a few probes settle it. Until one arrives nothing is hidden.
  const [seen, setSeen] = useState<SystemStats | null>(null);
  useEffect(() => {
    let alive = true;
    let attempts = 0;
    let timer: number | undefined;
    const probe = () => {
      getSystemStats()
        .then((stats) => {
          if (!alive) return;
          setSeen(stats);
          attempts += 1;
          const complete = availableMetrics(stats, SYSTEM_METRICS).length === SYSTEM_METRICS.length;
          if (!complete && attempts < PROBE_ATTEMPTS) timer = window.setTimeout(probe, PROBE_INTERVAL_MS);
        })
        .catch(() => {
          // Without a reading every counter stays offered: the bar tells the truth about the rest.
        });
    };
    probe();
    return () => {
      alive = false;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, []);
  const offered: readonly SystemMetric[] = seen ? availableMetrics(seen, SYSTEM_METRICS) : SYSTEM_METRICS;
  const reorder = useCallback(
    (collapsedPriority: CollapsedItem[]) => onSettingsChange({ ...settings, collapsedPriority }),
    [onSettingsChange, settings],
  );
  // Pressing keeps the order the user pressed in; releasing takes the counter out of the rotation.
  const toggleMetric = (metric: SystemMetric, on: boolean) => {
    const without = settings.systemMetrics.filter((other) => other !== metric);
    const systemMetrics = on ? [...without, metric] : without;
    onSettingsChange({ ...settings, systemMetrics });
  };

  return (
    <>
      <SettingsGroup title={text.title} footer={text.priorityFooter}>
        <ReorderList
          rows={order.map((item) => {
            const Icon = ICONS[item];
            return {
              id: item,
              ...text.items[item],
              icon: <Icon className="reorder-list__icon" size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden="true" />,
            };
          })}
          onReorder={reorder}
          moveLabel={text.moveLabel}
        />
        <SettingsToggleRow
          title={text.showCover}
          description={text.showCoverHint}
          checked={settings.collapsedShowCover}
          onChange={(collapsedShowCover) => onSettingsChange({ ...settings, collapsedShowCover })}
        />
      </SettingsGroup>
      <SettingsGroup title={text.metricsTitle} footer={text.metricsFooter}>
        <SettingsRow
          title={text.metricsRow}
          control={
            <SettingsChips
              values={settings.systemMetrics}
              options={offered.map((metric) => ({ value: metric, label: text.metrics[metric] }))}
              ariaLabel={text.metricsRow}
              onToggle={toggleMetric}
            />
          }
        />
      </SettingsGroup>
    </>
  );
}
