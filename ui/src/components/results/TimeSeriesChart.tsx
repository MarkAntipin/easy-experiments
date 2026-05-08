import { useMemo, useState } from 'react';
import type { Granularity, TimeSeriesBucket } from '@/api/types';
import { variantColorByKey } from '@/lib/variantColors';

export interface TimeSeriesChartProps {
  buckets: TimeSeriesBucket[];
  variantKeyOrder: readonly string[];
  granularity: Granularity;
}

const W = 1000;
const H = 260;
const PAD_L = 56;
const PAD_R = 16;
const PAD_T = 16;
const PAD_B = 32;

function niceMax(raw: number): number {
  if (raw <= 0) return 1;
  const exp = Math.floor(Math.log10(raw));
  const base = Math.pow(10, exp);
  for (const m of [1, 1.5, 2, 2.5, 3, 5, 7.5, 10]) {
    if (raw <= m * base) return m * base;
  }
  return 10 * base;
}

function formatBucketLabel(ms: number, granularity: Granularity): string {
  const d = new Date(ms);
  if (granularity === 'hour') {
    return d.toLocaleTimeString(undefined, { hour: 'numeric' });
  }
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

function formatTooltipBucketLabel(ms: number, granularity: Granularity): string {
  const d = new Date(ms);
  if (granularity === 'hour') {
    return d.toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
    });
  }
  return d.toLocaleDateString(undefined, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
  });
}

export function TimeSeriesChart({
  buckets,
  variantKeyOrder,
  granularity,
}: TimeSeriesChartProps) {
  const [hoverIdx, setHoverIdx] = useState<number | null>(null);

  const series = useMemo(() => {
    if (buckets.length === 0) return [];
    return variantKeyOrder.map((variantKey) => {
      const color = variantColorByKey(variantKeyOrder, variantKey);
      const points = buckets.map((b) => ({
        ms: b.bucketStartMs,
        count: b.perVariant[variantKey] ?? 0,
      }));
      return { variantKey, color, points };
    });
  }, [buckets, variantKeyOrder]);

  const yMax = useMemo(() => {
    let max = 0;
    for (const s of series) {
      for (const p of s.points) max = Math.max(max, p.count);
    }
    return niceMax(max);
  }, [series]);

  const xAt = (i: number): number => {
    if (buckets.length <= 1) return PAD_L;
    return PAD_L + (i / (buckets.length - 1)) * (W - PAD_L - PAD_R);
  };
  const yAt = (n: number): number =>
    H - PAD_B - (n / yMax) * (H - PAD_T - PAD_B);

  const yTicks = [0, 0.25, 0.5, 0.75, 1].map((t) => Math.round(yMax * t));

  // X-axis: render up to ~6 evenly spaced labels.
  const labelEvery = Math.max(1, Math.ceil(buckets.length / 6));

  if (buckets.length === 0) {
    return (
      <div className="flex h-48 items-center justify-center rounded-lg border border-dashed border-slate-300 bg-white text-sm text-slate-500">
        No exposures recorded in this window.
      </div>
    );
  }

  // Tooltip
  const hoverBucket = hoverIdx !== null ? buckets[hoverIdx] : null;

  return (
    <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
      <div className="border-b border-slate-200 px-5 py-3.5">
        <h2 className="text-base font-semibold text-slate-900">
          Exposure pace over time
        </h2>
        <p className="mt-0.5 text-sm text-slate-500">
          Hover any point to see the per-variant counts for that{' '}
          {granularity === 'hour' ? 'hour' : 'day'}.
        </p>
      </div>

      <div className="relative px-3 pb-3 pt-3">
        <svg
          viewBox={`0 0 ${W} ${H}`}
          width="100%"
          height={H}
          role="img"
          aria-label="Exposures over time per variant"
          preserveAspectRatio="none"
          onMouseMove={(e) => {
            const svg = e.currentTarget;
            const rect = svg.getBoundingClientRect();
            const xPx = e.clientX - rect.left;
            const xVb = (xPx / rect.width) * W;
            if (xVb < PAD_L || xVb > W - PAD_R || buckets.length === 0) {
              setHoverIdx(null);
              return;
            }
            const t = (xVb - PAD_L) / (W - PAD_L - PAD_R);
            const idx = Math.min(
              buckets.length - 1,
              Math.max(0, Math.round(t * (buckets.length - 1))),
            );
            setHoverIdx(idx);
          }}
          onMouseLeave={() => setHoverIdx(null)}
        >
          {/* Y grid + labels */}
          {yTicks.map((y) => {
            const yp = yAt(y);
            return (
              <g key={y}>
                <line
                  x1={PAD_L}
                  x2={W - PAD_R}
                  y1={yp}
                  y2={yp}
                  stroke="#e2e8f0"
                  strokeWidth={1}
                  strokeDasharray={y === 0 ? undefined : '3 3'}
                />
                <text
                  x={PAD_L - 8}
                  y={yp + 3}
                  textAnchor="end"
                  className="fill-slate-500"
                  style={{ fontSize: 11 }}
                >
                  {y.toLocaleString()}
                </text>
              </g>
            );
          })}

          {/* X-axis labels */}
          {buckets.map((b, i) => {
            if (i % labelEvery !== 0 && i !== buckets.length - 1) return null;
            return (
              <text
                key={b.bucketStartMs}
                x={xAt(i)}
                y={H - 10}
                textAnchor="middle"
                className="fill-slate-500"
                style={{ fontSize: 11 }}
              >
                {formatBucketLabel(b.bucketStartMs, granularity)}
              </text>
            );
          })}

          {/* Lines + filled area */}
          {series.map((s) => {
            if (s.points.every((p) => p.count === 0)) return null;
            const linePath = s.points
              .map((p, i) => `${i === 0 ? 'M' : 'L'} ${xAt(i)} ${yAt(p.count)}`)
              .join(' ');
            const areaPath = `${linePath} L ${xAt(s.points.length - 1)} ${yAt(0)} L ${xAt(0)} ${yAt(0)} Z`;
            return (
              <g key={s.variantKey}>
                <path d={areaPath} fill={`${s.color.hex}1a`} />
                <path
                  d={linePath}
                  fill="none"
                  stroke={s.color.hex}
                  strokeWidth={2}
                  strokeLinejoin="round"
                  strokeLinecap="round"
                />
                {s.points.map((p, i) => (
                  <circle
                    key={i}
                    cx={xAt(i)}
                    cy={yAt(p.count)}
                    r={hoverIdx === i ? 4 : 2.5}
                    fill={s.color.hex}
                    style={{ transition: 'r 120ms ease' }}
                  />
                ))}
              </g>
            );
          })}

          {/* Hover indicator line */}
          {hoverIdx !== null ? (
            <line
              x1={xAt(hoverIdx)}
              x2={xAt(hoverIdx)}
              y1={PAD_T}
              y2={H - PAD_B}
              stroke="#94a3b8"
              strokeWidth={1}
              strokeDasharray="3 3"
            />
          ) : null}
        </svg>

        {/* Legend */}
        <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1.5 px-3 text-sm">
          {series.map((s) => (
            <span key={s.variantKey} className="inline-flex items-center gap-1.5">
              <span
                aria-hidden
                className="inline-block h-2.5 w-2.5 rounded-sm"
                style={{ backgroundColor: s.color.hex }}
              />
              <span className="font-mono text-slate-700">{s.variantKey}</span>
            </span>
          ))}
        </div>

        {/* Tooltip — positioned with a small absolute layer */}
        {hoverBucket && hoverIdx !== null ? (
          <Tooltip
            label={formatTooltipBucketLabel(hoverBucket.bucketStartMs, granularity)}
            series={series.map((s) => ({
              key: s.variantKey,
              hex: s.color.hex,
              count: s.points[hoverIdx]?.count ?? 0,
            }))}
            // Anchor roughly under the indicator line as a fraction of width.
            xFrac={Math.max(0.05, Math.min(0.95, xAt(hoverIdx) / W))}
          />
        ) : null}
      </div>
    </div>
  );
}

function Tooltip({
  label,
  series,
  xFrac,
}: {
  label: string;
  series: Array<{ key: string; hex: string; count: number }>;
  xFrac: number;
}) {
  const total = series.reduce((acc, s) => acc + s.count, 0);
  return (
    <div
      className="pointer-events-none absolute top-2 z-10 max-w-[260px] rounded-md border border-slate-200 bg-white px-3 py-2 shadow-md"
      style={{ left: `${xFrac * 100}%`, transform: 'translateX(-50%)' }}
    >
      <div className="text-xs font-semibold text-slate-500">{label}</div>
      <ul className="mt-1.5 flex flex-col gap-1">
        {series.map((s) => (
          <li
            key={s.key}
            className="flex items-center gap-2 text-sm tabular-nums text-slate-800"
          >
            <span
              aria-hidden
              className="inline-block h-2.5 w-2.5 rounded-sm"
              style={{ backgroundColor: s.hex }}
            />
            <span className="font-mono text-slate-700">{s.key}</span>
            <span className="ml-auto font-medium">{s.count.toLocaleString()}</span>
          </li>
        ))}
        <li className="mt-1 flex items-center justify-between border-t border-slate-100 pt-1 text-xs text-slate-500">
          <span>total</span>
          <span className="tabular-nums">{total.toLocaleString()}</span>
        </li>
      </ul>
    </div>
  );
}

