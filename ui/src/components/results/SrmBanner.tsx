import { useState } from 'react';
import { AlertTriangle, CheckCircle2, ChevronDown, ChevronUp } from 'lucide-react';
import type { SrmResult } from '@/api/types';
import { variantColorByKey } from '@/lib/variantColors';
import { cn } from '@/lib/cn';

export interface SrmBannerProps {
  srm: SrmResult | null;
  variantKeyOrder: readonly string[];
}

function formatPct(n: number): string {
  return `${(n * 100).toFixed(1)}%`;
}

export function SrmBanner({ srm, variantKeyOrder }: SrmBannerProps) {
  const [open, setOpen] = useState(false);

  if (srm === null) {
    return null;
  }

  const healthy = !srm.warning;

  return (
    <div
      className={cn(
        'rounded-lg border',
        healthy
          ? 'border-emerald-200 bg-emerald-50/70'
          : 'border-rose-200 bg-rose-50',
      )}
    >
      <div className="flex flex-col gap-2 px-4 py-2.5 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-sm">
          {healthy ? (
            <span className="inline-flex items-center gap-2 font-medium text-emerald-900">
              <CheckCircle2 aria-hidden className="h-4 w-4 text-emerald-600" />
              Traffic split healthy
            </span>
          ) : (
            <span className="inline-flex items-center gap-2 font-semibold text-rose-900">
              <AlertTriangle aria-hidden className="h-4 w-4 text-rose-600" />
              Sample ratio mismatch detected
            </span>
          )}
          <span className="flex flex-wrap items-center gap-x-3 gap-y-1">
            {srm.expected.map((s) => {
              const color = variantColorByKey(variantKeyOrder, s.variantKey);
              return (
                <span
                  key={s.variantKey}
                  className="inline-flex items-center gap-1.5 text-slate-700"
                >
                  <span
                    aria-hidden
                    className="inline-block h-2 w-2 rounded-sm"
                    style={{ backgroundColor: color.hex }}
                  />
                  <span className="font-mono text-xs">{s.variantKey}</span>
                  <span className="tabular-nums">{formatPct(s.actual)}</span>
                  {!healthy ? (
                    <span className="text-xs text-slate-500">
                      / {formatPct(s.expected)} expected
                    </span>
                  ) : null}
                </span>
              );
            })}
          </span>
        </div>
        <button
          type="button"
          onClick={() => setOpen((v) => !v)}
          className={cn(
            'inline-flex shrink-0 items-center gap-1 self-start rounded-md px-2 py-1 text-xs font-medium transition sm:self-auto',
            healthy
              ? 'text-emerald-800 hover:bg-emerald-100'
              : 'text-rose-800 hover:bg-rose-100',
          )}
          aria-expanded={open}
        >
          How is this counted?
          {open ? (
            <ChevronUp aria-hidden className="h-3.5 w-3.5" />
          ) : (
            <ChevronDown aria-hidden className="h-3.5 w-3.5" />
          )}
        </button>
      </div>

      {open ? (
        <div
          className={cn(
            'border-t px-4 py-3 text-sm',
            healthy
              ? 'border-emerald-200/70 bg-white/60 text-slate-700'
              : 'border-rose-200/70 bg-white/60 text-slate-700',
          )}
        >
          <p>
            <strong>Actual %</strong> is the share of exposures that landed on
            each variant: variant exposures ÷ total exposures.
          </p>
          <p className="mt-1.5">
            <strong>Expected %</strong> comes from the distribution you
            configured for the experiment.
          </p>
        </div>
      ) : null}
    </div>
  );
}
