import type { VariantResult } from '@/api/types';
import { variantColorByKey } from '@/lib/variantColors';
import { cn } from '@/lib/cn';

export interface VariantsTableProps {
  variants: VariantResult[];
  variantKeyOrder: readonly string[];
}

const NUM = new Intl.NumberFormat();

function fmtRate(rate: number | null): string {
  if (rate === null) return '—';
  return `${(rate * 100).toFixed(2)}%`;
}

function fmtLift(lift: number | null): string {
  if (lift === null) return '—';
  const sign = lift > 0 ? '+' : '';
  return `${sign}${(lift * 100).toFixed(1)}%`;
}

export function VariantsTable({ variants, variantKeyOrder }: VariantsTableProps) {
  return (
    <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
      <div className="border-b border-slate-200 px-5 py-3.5">
        <h2 className="text-base font-semibold text-slate-900">Variants</h2>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="bg-slate-50/60 text-xs uppercase tracking-wide text-slate-500">
            <tr>
              <th className="px-5 py-3 text-left font-semibold">Variant</th>
              <th className="px-5 py-3 text-right font-semibold">Exposures</th>
              <th className="px-5 py-3 text-right font-semibold">Converters</th>
              <th className="px-5 py-3 text-right font-semibold">Rate</th>
              <th className="px-5 py-3 text-right font-semibold">Lift vs ctrl</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {variants.map((v) => {
              const color = variantColorByKey(variantKeyOrder, v.variantKey);
              const liftClass =
                v.lift === null
                  ? 'text-slate-400'
                  : v.lift > 0
                    ? 'text-emerald-700'
                    : v.lift < 0
                      ? 'text-rose-700'
                      : 'text-slate-500';
              return (
                <tr key={v.variantKey} className="hover:bg-slate-50/40">
                  <td className="px-5 py-3">
                    <div className="flex items-center gap-2">
                      <span
                        aria-hidden
                        className="inline-block h-3 w-3 shrink-0 rounded-sm"
                        style={{ backgroundColor: color.hex }}
                      />
                      <span className="font-medium text-slate-900">
                        {v.variantKey}
                      </span>
                      {v.isControl ? (
                        <span className="inline-flex items-center rounded-full bg-slate-100 px-2 py-0.5 text-xs font-semibold uppercase tracking-wide text-slate-600">
                          control
                        </span>
                      ) : null}
                    </div>
                  </td>
                  <td className="px-5 py-3 text-right tabular-nums text-slate-700">
                    {NUM.format(v.exposures)}
                  </td>
                  <td className="px-5 py-3 text-right tabular-nums text-slate-700">
                    {NUM.format(v.converters)}
                  </td>
                  <td className="px-5 py-3 text-right tabular-nums font-medium text-slate-900">
                    {fmtRate(v.conversionRate)}
                  </td>
                  <td
                    className={cn(
                      'px-5 py-3 text-right tabular-nums font-medium',
                      liftClass,
                    )}
                  >
                    {fmtLift(v.lift)}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
