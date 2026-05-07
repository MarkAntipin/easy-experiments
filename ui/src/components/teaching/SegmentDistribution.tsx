import { AlertTriangle } from 'lucide-react';
import { variantColorByKey } from '@/lib/variantColors';
import { cn } from '@/lib/cn';

interface DistributionInput {
  variantKey: string;
  percent: number;
}

export interface SegmentDistributionProps {
  distributions: DistributionInput[];
  variantKeys: readonly string[];
}

const SIZE = 128;
const R_OUTER = 56;
const R_INNER = 32;
const CX = SIZE / 2;
const CY = SIZE / 2;
const EPSILON = 0.0001;

function clampNonNeg(n: number): number {
  if (Number.isNaN(n)) return 0;
  return Math.max(0, n);
}

function polarToCartesian(angleDeg: number, r: number) {
  const a = (angleDeg - 90) * (Math.PI / 180);
  return { x: CX + r * Math.cos(a), y: CY + r * Math.sin(a) };
}

function donutSlicePath(startAngle: number, endAngle: number): string {
  const sweep = endAngle - startAngle;
  if (sweep <= 0) return '';
  // SVG can't render a single arc that sweeps a full 360°; nudge it down so the
  // path stays well-formed for the all-one-slice / all-unallocated cases.
  const eff = sweep >= 360 ? startAngle + 359.999 : endAngle;
  const so = polarToCartesian(startAngle, R_OUTER);
  const eo = polarToCartesian(eff, R_OUTER);
  const si = polarToCartesian(eff, R_INNER);
  const ei = polarToCartesian(startAngle, R_INNER);
  const large = eff - startAngle > 180 ? 1 : 0;
  return [
    `M ${so.x} ${so.y}`,
    `A ${R_OUTER} ${R_OUTER} 0 ${large} 1 ${eo.x} ${eo.y}`,
    `L ${si.x} ${si.y}`,
    `A ${R_INNER} ${R_INNER} 0 ${large} 0 ${ei.x} ${ei.y}`,
    'Z',
  ].join(' ');
}

function formatPct(n: number): string {
  if (Math.abs(n - Math.round(n)) < EPSILON) return String(Math.round(n));
  return n.toFixed(1);
}

export function SegmentDistribution({
  distributions,
  variantKeys,
}: SegmentDistributionProps) {
  const items = distributions.map((d) => ({
    variantKey: d.variantKey,
    pct: clampNonNeg(Number(d.percent) || 0),
  }));
  const distSum = items.reduce((acc, i) => acc + i.pct, 0);
  const overshoot = Math.max(0, distSum - 100);
  const undershoot = Math.max(0, 100 - distSum);
  const balanced = Math.abs(distSum - 100) < EPSILON;
  const overAllocated = distSum > 100 + EPSILON;

  // When over-allocated we scale slices to sum so the chart still renders as a
  // full circle; the over-allocation is surfaced through the status pill and
  // explicit warning text below. Under-allocation leaves a hatched wedge.
  const denom = overAllocated ? distSum : 100;
  const angleFor = (pct: number) => (pct / denom) * 360;

  let cursor = 0;
  const slices = items.map((item, i) => {
    const sweep = angleFor(item.pct);
    const start = cursor;
    const end = cursor + sweep;
    cursor = end;
    return {
      key: `${item.variantKey || '__empty__'}-${i}`,
      variantKey: item.variantKey,
      pct: item.pct,
      start,
      end,
      color: variantColorByKey(variantKeys, item.variantKey),
    };
  });
  const unallocatedSweep = undershoot > 0 ? (undershoot / 100) * 360 : 0;
  const unallocatedStart = cursor;
  const unallocatedEnd = cursor + unallocatedSweep;

  let statusText: string;
  let statusClass: string;
  if (balanced) {
    statusText = '100%';
    statusClass = 'text-emerald-600';
  } else if (overAllocated) {
    statusText = `${formatPct(distSum)}% · over by ${formatPct(overshoot)}%`;
    statusClass = 'text-red-600';
  } else {
    statusText = `${formatPct(distSum)}% · ${formatPct(undershoot)}% unallocated`;
    statusClass = 'text-amber-600';
  }

  return (
    <div
      className={cn(
        'flex items-center gap-4 rounded-md border p-3',
        overAllocated
          ? 'border-red-200 bg-red-50/40'
          : 'border-slate-200 bg-white',
      )}
    >
      <svg
        width={SIZE}
        height={SIZE}
        viewBox={`0 0 ${SIZE} ${SIZE}`}
        className="shrink-0"
        role="img"
        aria-label="Variant distribution"
      >
        <defs>
          <pattern
            id="seg-dist-unalloc"
            patternUnits="userSpaceOnUse"
            width="6"
            height="6"
            patternTransform="rotate(45)"
          >
            <rect width="6" height="6" fill="#fef3c7" />
            <rect width="3" height="6" fill="#fde68a" />
          </pattern>
        </defs>
        <circle cx={CX} cy={CY} r={R_OUTER} fill="#f1f5f9" />
        {slices.map((s) =>
          s.end > s.start ? (
            <path
              key={s.key}
              d={donutSlicePath(s.start, s.end)}
              fill={s.color.hex}
            />
          ) : null,
        )}
        {unallocatedSweep > 0 ? (
          <path
            d={donutSlicePath(unallocatedStart, unallocatedEnd)}
            fill="url(#seg-dist-unalloc)"
          />
        ) : null}
        <circle cx={CX} cy={CY} r={R_INNER} fill="white" />
      </svg>

      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-2">
          <span className="text-[11px] font-semibold uppercase tracking-wide text-slate-500">
            Split into variants
          </span>
          <span
            className={cn(
              'inline-flex items-center gap-1 text-xs font-medium tabular-nums',
              statusClass,
            )}
          >
            {overAllocated ? <AlertTriangle aria-hidden className="h-3.5 w-3.5" /> : null}
            {statusText}
          </span>
        </div>

        <ul className="mt-2 flex flex-col gap-1 text-[11px] text-slate-600">
          {slices.map((s) => (
            <li key={`legend-${s.key}`} className="flex items-center gap-1.5">
              <span
                className="inline-block h-2.5 w-2.5 shrink-0 rounded-sm"
                style={{ backgroundColor: s.color.hex }}
              />
              <span className="min-w-0 truncate font-medium text-ink-900">
                {s.variantKey || '—'}
              </span>
              <span className="ml-auto shrink-0 tabular-nums text-slate-500">
                {formatPct(s.pct)}%
              </span>
            </li>
          ))}
          {undershoot > 0 ? (
            <li className="flex items-center gap-1.5 text-amber-700">
              <span className="inline-block h-2.5 w-2.5 shrink-0 rounded-sm bg-amber-300" />
              <span className="min-w-0 truncate font-medium">unallocated</span>
              <span className="ml-auto shrink-0 tabular-nums">
                {formatPct(undershoot)}%
              </span>
            </li>
          ) : null}
        </ul>

        {overAllocated ? (
          <p className="mt-2 text-[11px] font-medium text-red-600">
            Total exceeds 100%. Reduce distributions by{' '}
            {formatPct(overshoot)}%.
          </p>
        ) : null}
      </div>
    </div>
  );
}
