import { Activity, ArrowDown, ArrowUp, Cpu, Gauge, MemoryStick, Thermometer } from "lucide-react";
import { useTranslation } from "../../i18n";
import { formatRate } from "./format";
import { Sparkline } from "./Sparkline";
import { useSystemHistory } from "./useSystemHistory";
import { availableMetrics } from "./types";
import "./SystemDashboard.css";

const GIGABYTE = 1024 ** 3;

function gigabytes(bytes: number): string {
  return (bytes / GIGABYTE).toFixed(1);
}

type CardProps = {
  icon: React.ReactNode;
  title: string;
  value: string;
  detail?: string;
  values: number[];
  ceiling?: number;
  unit?: string;
  label: string;
};

function MetricCard({ icon, title, value, detail, values, ceiling, unit, label }: CardProps) {
  return <article className="system-dashboard__card">
    <div className="system-dashboard__head">
      <span className="system-dashboard__icon" aria-hidden="true">{icon}</span>
      <span className="system-dashboard__title">{title}</span>
      <span className="system-dashboard__value">{value}</span>
    </div>
    {detail && <span className="system-dashboard__detail">{detail}</span>}
    <Sparkline values={values} ceiling={ceiling} label={label} unit={unit} />
  </article>;
}

/**
 * The machine page: what the processor, the memory and the rest of the computer are doing.
 * The collapsed bar shows one reading at a time; this page shows them all with a short trend.
 */
export function SystemDashboard({ active }: { active: boolean }) {
  const labels = useTranslation().island.systemPage;
  const { stats, history } = useSystemHistory(active);
  if (!stats) {
    return <section className="system-dashboard" aria-label={labels.title} aria-busy="true">
      <p className="system-dashboard__empty">{labels.reading}</p>
    </section>;
  }

  const readable = new Set(availableMetrics(stats, ["cpu", "memory", "network", "gpu", "temperature"]));
  const percent = (value: number | null) => value === null ? labels.unavailable : `${Math.round(value)}%`;
  const trend = (values: number[], unit: string) => values.length > 1
    ? labels.trend(`${Math.round(values[0])}${unit}`, `${Math.round(values[values.length - 1])}${unit}`)
    : labels.oneReading;

  return <section className="system-dashboard" aria-label={labels.title}>
    <div className="system-dashboard__grid">
      <MetricCard
        icon={<Cpu size={15} strokeWidth={1.6} absoluteStrokeWidth />}
        title={labels.cpu}
        value={percent(stats.cpuPercent)}
        values={history.cpu}
        ceiling={100}
        unit="%"
        label={trend(history.cpu, "%")}
      />
      <MetricCard
        icon={<MemoryStick size={15} strokeWidth={1.6} absoluteStrokeWidth />}
        title={labels.memory}
        value={percent(stats.memoryPercent)}
        detail={labels.memoryDetail(gigabytes(stats.memoryUsed), gigabytes(stats.memoryTotal))}
        values={history.memory}
        ceiling={100}
        unit="%"
        label={trend(history.memory, "%")}
      />
      <MetricCard
        icon={<Gauge size={15} strokeWidth={1.6} absoluteStrokeWidth />}
        title={labels.gpu}
        value={percent(stats.gpuPercent)}
        values={history.gpu}
        ceiling={100}
        unit="%"
        label={trend(history.gpu, "%")}
      />
      <MetricCard
        icon={<Thermometer size={15} strokeWidth={1.6} absoluteStrokeWidth />}
        title={labels.temperature}
        value={stats.temperature === null ? labels.unavailable : `${Math.round(stats.temperature)}°C`}
        values={history.temperature}
        unit="°C"
        label={trend(history.temperature, "°C")}
      />
      <article className="system-dashboard__card system-dashboard__card--network">
        <div className="system-dashboard__head">
          <span className="system-dashboard__icon" aria-hidden="true"><Activity size={15} strokeWidth={1.6} absoluteStrokeWidth /></span>
          <span className="system-dashboard__title">{labels.network}</span>
        </div>
        <div className="system-dashboard__rates">
          <span className="system-dashboard__rate">
            <ArrowDown size={13} strokeWidth={1.8} aria-hidden="true" />
            {formatRate(stats.networkDown)}
          </span>
          <span className="system-dashboard__rate">
            <ArrowUp size={13} strokeWidth={1.8} aria-hidden="true" />
            {formatRate(stats.networkUp)}
          </span>
        </div>
        <Sparkline values={history.network} label={labels.networkTrend} />
      </article>
    </div>
    <p className="system-dashboard__note">
      {readable.has("temperature") ? labels.note : labels.noteWithoutTemperature}
    </p>
  </section>;
}
