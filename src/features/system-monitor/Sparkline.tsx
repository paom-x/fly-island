import { useEffect, useId, useRef, useState, type PointerEvent } from "react";

type SparklineProps = {
  /** Oldest first; the last value is the current one. */
  values: number[];
  /** Ceiling of the chart; 0 means "the largest value seen". */
  ceiling?: number;
  /** Describes the trend for screen readers, e.g. "processor, 12% to 27%". */
  label: string;
  /** What the tooltip says in front of the number. */
  unit?: string;
};

type Point = { x: number; y: number };
type Size = { width: number; height: number };

const PAD = 6;
const MEASURE_RETRY_MS = 200;

/**
 * Project the readings onto the svg's real pixel box. Working in CSS pixels instead of a stretched
 * 0..100 viewBox is what keeps the end dot a circle no matter how wide the card is.
 */
function pointsOf(values: number[], ceiling: number, size: Size): Point[] {
  const top = Math.max(ceiling, ...values, 1);
  const step = values.length > 1 ? (size.width - 2) / (values.length - 1) : 0;
  return values.map((value, index) => ({
    x: values.length === 1 ? size.width / 2 : 1 + index * step,
    y: size.height - PAD - (Math.max(0, value) / top) * (size.height - PAD * 2 - 4),
  }));
}

/** A cubic through the midpoints keeps short ranges readable without spoiling exact peaks. */
function pathOf(points: Point[]): string {
  if (points.length === 0) return "";
  let d = `M ${points[0].x.toFixed(2)},${points[0].y.toFixed(2)}`;
  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1];
    const point = points[index];
    const middle = (previous.x + point.x) / 2;
    d += ` C ${middle.toFixed(2)},${previous.y.toFixed(2)} ${middle.toFixed(2)},${point.y.toFixed(2)} ${point.x.toFixed(2)},${point.y.toFixed(2)}`;
  }
  return d;
}

/** A card's trend: no axes, a faint fill under the line and a dot on the newest reading. */
export function Sparkline({ values, ceiling = 0, label, unit = "" }: SparklineProps) {
  const gradient = useId();
  const svgRef = useRef<SVGSVGElement | null>(null);
  const [size, setSize] = useState<Size>({ width: 0, height: 0 });
  const [hover, setHover] = useState<number | null>(null);
  const isEmpty = values.length === 0;

  // CSS gives the svg its size, so it has no intrinsic box on the first frame. Observe the layout
  // instead of guessing, and retry a zero-sized first measurement until the island is laid out.
  useEffect(() => {
    if (isEmpty) return;
    const svg = svgRef.current;
    if (!svg) return;

    let retry = 0;
    const measure = () => {
      const rect = svg.getBoundingClientRect();
      const width = Math.round(rect.width);
      const height = Math.round(rect.height);
      if (width === 0 || height === 0) {
        retry = window.setTimeout(measure, MEASURE_RETRY_MS);
        return;
      }
      setSize((previous) =>
        previous.width === width && previous.height === height ? previous : { width, height },
      );
    };

    const observer = new ResizeObserver(measure);
    observer.observe(svg);
    measure();
    return () => {
      observer.disconnect();
      window.clearTimeout(retry);
    };
  }, [isEmpty]);

  if (isEmpty) return <div className="system-spark system-spark--empty" aria-hidden="true" />;

  const measured = size.width > 0 && size.height > 0;
  const points = measured ? pointsOf(values, ceiling, size) : [];
  const line = pathOf(points);
  const last = points[points.length - 1];
  const area = last
    ? `${line} L ${last.x.toFixed(2)},${size.height} L ${points[0].x.toFixed(2)},${size.height} Z`
    : "";
  const active = hover === null ? null : points[hover] ?? null;
  const activeValue = hover === null ? values[values.length - 1] : values[hover] ?? values[values.length - 1];

  // Nearest point is picked by svg-local x, so anywhere in a reading's column selects that reading.
  const onPointerMove = (event: PointerEvent<SVGSVGElement>) => {
    if (points.length === 0) return;
    const x = event.clientX - event.currentTarget.getBoundingClientRect().left;
    let nearest = 0;
    let distance = Infinity;
    for (let index = 0; index < points.length; index += 1) {
      const delta = Math.abs(points[index].x - x);
      if (delta < distance) {
        distance = delta;
        nearest = index;
      }
    }
    setHover(nearest);
  };

  return <div className="system-spark">
    <svg
      ref={svgRef}
      className="system-spark__svg"
      viewBox={measured ? `0 0 ${size.width} ${size.height}` : undefined}
      role="img"
      aria-label={label}
      onPointerMove={onPointerMove}
      onPointerLeave={() => setHover(null)}
    >
      {measured && <>
        <defs>
          <linearGradient id={gradient} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="var(--color-accent)" stopOpacity="0.3" />
            <stop offset="1" stopColor="var(--color-accent)" stopOpacity="0" />
          </linearGradient>
        </defs>
        <path d={area} fill={`url(#${gradient})`} />
        <line
          className={`system-spark__guide${active ? " on" : ""}`}
          x1={active?.x ?? 0}
          y1={active?.y ?? 0}
          x2={active?.x ?? 0}
          y2={size.height}
        />
        <path className="system-spark__line" d={line} />
        <circle className={`system-spark__hover${active ? " on" : ""}`} r={3.5} cx={active?.x ?? 0} cy={active?.y ?? 0} />
        <circle className="system-spark__halo" r={5} cx={last.x} cy={last.y} />
        <circle className="system-spark__end" r={3} cx={last.x} cy={last.y} />
      </>}
    </svg>
    <span
      className={`system-spark__tip${active ? " on" : ""}`}
      style={{ left: `${active?.x ?? last?.x ?? 0}px`, top: `${active?.y ?? last?.y ?? 0}px` }}
    >
      {Math.round(activeValue)}{unit}
    </span>
  </div>;
}
