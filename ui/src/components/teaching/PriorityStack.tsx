import { ArrowDown } from 'lucide-react';
import { variantColorByKey } from '@/lib/variantColors';

interface PriorityStackSegment {
  /** Stable key used for React reconciliation (the form-array field id). */
  fieldId: string;
  priority: number;
  rolloutPercent: number;
  constraintCount: number;
  distributions: Array<{ variantKey: string; percent: number }>;
  /** 1-based label (segment #N from the form order). */
  positionLabel: number;
}

export interface PriorityStackProps {
  segments: PriorityStackSegment[];
  variantKeys: readonly string[];
}

function clampPercent(n: number) {
  if (Number.isNaN(n)) return 0;
  return Math.max(0, Math.min(100, n));
}

export function PriorityStack({ segments, variantKeys }: PriorityStackProps) {
  if (segments.length < 2) return null;

  const sorted = [...segments].sort((a, b) => {
    const ra = Number(a.priority);
    const rb = Number(b.priority);
    if (Number.isNaN(ra) && Number.isNaN(rb)) return 0;
    if (Number.isNaN(ra)) return 1;
    if (Number.isNaN(rb)) return -1;
    return ra - rb;
  });

  return (
    <div className="rounded-md border border-slate-200 bg-white p-4">
      <div className="mb-2 flex items-baseline justify-between">
        <span className="text-sm font-semibold uppercase tracking-wide text-slate-500">
          Match order
        </span>
        <span className="text-xs italic text-slate-400">
          first match wins
        </span>
      </div>

      <div className="flex gap-3">
        <div className="flex flex-col items-center pt-1">
          <ArrowDown className="h-4 w-4 text-slate-400" />
          <div className="mt-0.5 w-px flex-1 bg-gradient-to-b from-slate-300 to-transparent" />
        </div>

        <ol className="flex min-w-0 flex-1 flex-col gap-1.5">
          {sorted.map((seg) => (
            <PriorityRow
              key={seg.fieldId}
              segment={seg}
              variantKeys={variantKeys}
            />
          ))}
        </ol>
      </div>
    </div>
  );
}

function PriorityRow({
  segment,
  variantKeys,
}: {
  segment: PriorityStackSegment;
  variantKeys: readonly string[];
}) {
  const rollout = clampPercent(Number(segment.rolloutPercent) || 0);
  const constraintLabel =
    segment.constraintCount === 0
      ? 'no filters · fallback'
      : segment.constraintCount === 1
        ? '1 filter'
        : `${segment.constraintCount} filters`;

  return (
    <li className="flex items-center gap-3 rounded border border-slate-100 bg-slate-50/60 px-3 py-2">
      <span className="inline-flex h-7 min-w-[2.75rem] items-center justify-center rounded bg-white px-2 text-sm font-semibold tabular-nums text-ink-900 ring-1 ring-slate-200">
        P{segment.priority}
      </span>
      <span className="hidden text-sm text-slate-500 sm:inline">
        Segment #{segment.positionLabel}
      </span>
      <span className="text-xs text-slate-500">{constraintLabel}</span>

      <div className="ml-auto flex items-center gap-2">
        <div className="hidden h-2 w-28 overflow-hidden rounded-full bg-slate-200 sm:block">
          <div className="flex h-full">
            {segment.distributions.map((d, i) => {
              const c = variantColorByKey(variantKeys, d.variantKey);
              const pct = clampPercent(Number(d.percent) || 0);
              const scaled = (pct / 100) * (rollout / 100) * 100;
              return (
                <div
                  key={`${d.variantKey}-${i}`}
                  style={{
                    width: `${scaled}%`,
                    backgroundColor: c.hex,
                    transition: 'width 280ms ease-out',
                  }}
                  className="h-full"
                />
              );
            })}
          </div>
        </div>
        <span className="w-12 text-right text-xs tabular-nums text-slate-500">
          {rollout}%
        </span>
      </div>
    </li>
  );
}
