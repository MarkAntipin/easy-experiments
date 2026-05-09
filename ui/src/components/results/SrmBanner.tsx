import { AlertTriangle } from 'lucide-react';
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

function formatPValue(p: number): string {
  if (p < 0.001) return '<0.001';
  if (p < 0.01) return p.toFixed(3);
  return p.toFixed(2);
}

export function SrmBanner({ srm, variantKeyOrder }: SrmBannerProps) {
  if (srm === null || !srm.warning) {
    return null;
  }

  return (
    <div className="rounded-lg border border-rose-200 bg-rose-50 px-5 py-4">
      <div className="flex items-start gap-3">
        <AlertTriangle aria-hidden className="mt-0.5 h-5 w-5 shrink-0 text-rose-600" />
        <div className="flex-1">
          <div className="text-base font-semibold text-rose-900">
            Sample ratio mismatch detected
          </div>
          <p className="mt-1 text-sm text-rose-800">
            The observed traffic split does not match the configured one (χ² ={' '}
            {srm.chiSquare.toFixed(2)}, p = {formatPValue(srm.pValue)}).{' '}
            Common causes: a broken bucketing call site, a bot filter that drops one
            arm, or a client-side cache. <strong>Don't trust the metrics below
            until this is resolved.</strong>
          </p>

          <ul className="mt-3 grid gap-1.5 text-sm sm:grid-cols-2">
            {srm.expected.map((s) => {
              const color = variantColorByKey(variantKeyOrder, s.variantKey);
              const drift = s.actual - s.expected;
              const driftClass = cn(
                'tabular-nums',
                Math.abs(drift) > 0.01 ? 'text-rose-700 font-medium' : 'text-slate-500',
              );
              return (
                <li
                  key={s.variantKey}
                  className="flex items-center gap-2 rounded-md bg-white/70 px-3 py-1.5"
                >
                  <span
                    className="inline-block h-2.5 w-2.5 rounded-sm"
                    style={{ backgroundColor: color.hex }}
                  />
                  <span className="font-mono text-sm text-slate-700">{s.variantKey}</span>
                  <span className="ml-auto flex items-baseline gap-1.5">
                    <span className="tabular-nums text-slate-600">
                      {formatPct(s.actual)}
                    </span>
                    <span className="text-xs text-slate-400">
                      / {formatPct(s.expected)} expected
                    </span>
                    <span className={driftClass}>
                      {drift > 0 ? '+' : ''}
                      {(drift * 100).toFixed(1)}pp
                    </span>
                  </span>
                </li>
              );
            })}
          </ul>
        </div>
      </div>
    </div>
  );
}
