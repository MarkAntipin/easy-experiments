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
  /**
   * Segment-level rollout, 0-100. Variant slices are scaled by this share and
   * the remainder shows as a gray "excluded by rollout" wedge. Defaults to 100.
   */
  rolloutPercent?: number;
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

function clampHundred(n: number): number {
  if (Number.isNaN(n)) return 0;
  return Math.max(0, Math.min(100, n));
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
  rolloutPercent = 100,
}: SegmentDistributionProps) {
  const rollout = clampHundred(Number(rolloutPercent));
  const rolloutFrac = rollout / 100;

  const items = distributions.map((d) => ({
    variantKey: d.variantKey,
    pct: clampNonNeg(Number(d.percent) || 0),
  }));
  const distSum = items.reduce((acc, i) => acc + i.pct, 0);
  const overshoot = Math.max(0, distSum - 100);
  const undershoot = Math.max(0, 100 - distSum);
  const balanced = Math.abs(distSum - 100) < EPSILON;
  const overAllocated = distSum > 100 + EPSILON;

  // When over-allocated we scale slices proportionally so they fill the
  // rolled-out share of the wheel; the over-allocation is surfaced through the
  // status pill and explicit warning text below. Under-allocation within the
  // segment leaves an amber hatched wedge; rollout < 100 leaves a separate
  // gray "excluded by rollout" wedge.
  const denom = overAllocated ? distSum : 100;
  const angleFor = (pct: number) => (pct / denom) * rolloutFrac * 360;

  let cursor = 0;
  const slices = items.map((item, i) => {
    const sweep = angleFor(item.pct);
    const start = cursor;
    const end = cursor + sweep;
    cursor = end;
    const effectivePct = (item.pct / denom) * rollout;
    return {
      key: `${item.variantKey || '__empty__'}-${i}`,
      variantKey: item.variantKey,
      effectivePct,
      start,
      end,
      color: variantColorByKey(variantKeys, item.variantKey),
    };
  });

  const unallocatedSweep =
    !overAllocated && undershoot > 0
      ? (undershoot / 100) * rolloutFrac * 360
      : 0;
  const unallocatedStart = cursor;
  const unallocatedEnd = cursor + unallocatedSweep;
  cursor = unallocatedEnd;
  const unallocatedEffectivePct = (undershoot / 100) * rollout;

  const excludedSweep = rollout < 100 ? ((100 - rollout) / 100) * 360 : 0;
  const excludedStart = cursor;
  const excludedEnd = cursor + excludedSweep;
  const excludedPct = 100 - rollout;

  let statusText: string;
  let statusClass: string;
  if (overAllocated) {
    statusText = `${formatPct(distSum)}% within · over by ${formatPct(overshoot)}%`;
    statusClass = 'text-red-600';
  } else if (undershoot > 0) {
    statusText = `${formatPct(undershoot)}% unallocated within`;
    statusClass = 'text-amber-600';
  } else if (rollout < 100) {
    statusText = `${formatPct(rollout)}% rolled out`;
    statusClass = 'text-slate-600';
  } else if (balanced) {
    statusText = '100%';
    statusClass = 'text-emerald-600';
  } else {
    statusText = `${formatPct(distSum)}%`;
    statusClass = 'text-slate-600';
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
          <pattern
            id="seg-dist-excluded"
            patternUnits="userSpaceOnUse"
            width="6"
            height="6"
            patternTransform="rotate(45)"
          >
            <rect width="6" height="6" fill="#f1f5f9" />
            <rect width="3" height="6" fill="#cbd5e1" />
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
        {excludedSweep > 0 ? (
          <path
            d={donutSlicePath(excludedStart, excludedEnd)}
            fill="url(#seg-dist-excluded)"
          />
        ) : null}
        <circle cx={CX} cy={CY} r={R_INNER} fill="white" />
      </svg>

      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-2">
          <span className="text-sm font-semibold uppercase tracking-wide text-slate-500">
            Split into variants
          </span>
          <span
            className={cn(
              'inline-flex items-center gap-1 text-sm font-medium tabular-nums',
              statusClass,
            )}
          >
            {overAllocated ? <AlertTriangle aria-hidden className="h-4 w-4" /> : null}
            {statusText}
          </span>
        </div>

        <ul className="mt-2 flex flex-col gap-1 text-sm text-slate-600">
          {slices.map((s) => (
            <li key={`legend-${s.key}`} className="flex items-center gap-2">
              <span
                className="inline-block h-3 w-3 shrink-0 rounded-sm"
                style={{ backgroundColor: s.color.hex }}
              />
              <span className="min-w-0 truncate font-medium text-ink-900">
                {s.variantKey || '—'}
              </span>
              <span className="ml-auto shrink-0 tabular-nums text-slate-500">
                {formatPct(s.effectivePct)}%
              </span>
            </li>
          ))}
          {unallocatedSweep > 0 ? (
            <li className="flex items-center gap-2 text-amber-700">
              <span className="inline-block h-3 w-3 shrink-0 rounded-sm bg-amber-300" />
              <span className="min-w-0 truncate font-medium">unallocated</span>
              <span className="ml-auto shrink-0 tabular-nums">
                {formatPct(unallocatedEffectivePct)}%
              </span>
            </li>
          ) : null}
          {excludedSweep > 0 ? (
            <li className="flex items-center gap-2 text-slate-500">
              <span className="inline-block h-3 w-3 shrink-0 rounded-sm bg-slate-300" />
              <span className="min-w-0 truncate font-medium">
                excluded by rollout
              </span>
              <span className="ml-auto shrink-0 tabular-nums">
                {formatPct(excludedPct)}%
              </span>
            </li>
          ) : null}
        </ul>

        {overAllocated ? (
          <p className="mt-2 text-sm font-medium text-red-600">
            Total exceeds 100%. Reduce distributions by{' '}
            {formatPct(overshoot)}%.
          </p>
        ) : null}
      </div>
    </div>
  );
}
