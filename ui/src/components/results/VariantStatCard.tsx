import { ArrowDownRight, ArrowUpRight, Minus } from 'lucide-react';
import type { VariantResult } from '@/api/types';
import { variantColorByKey } from '@/lib/variantColors';
import { cn } from '@/lib/cn';
import { SignificanceBadge, significanceHelper } from './SignificanceBadge';

export interface VariantStatCardProps {
  variant: VariantResult;
  variantKeyOrder: readonly string[];
  /** True iff there's a treatment in the experiment that beat this one
   * with significance — used to apply a subtle "winner" treatment. */
  isWinner: boolean;
}

const NUM_FMT = new Intl.NumberFormat();

function formatPct(n: number, digits = 2): string {
  return `${(n * 100).toFixed(digits)}%`;
}

function formatLift(lift: number): string {
  const sign = lift > 0 ? '+' : '';
  return `${sign}${(lift * 100).toFixed(1)}%`;
}

export function VariantStatCard({
  variant: v,
  variantKeyOrder,
  isWinner,
}: VariantStatCardProps) {
  const color = variantColorByKey(variantKeyOrder, v.variantKey);
  const noExposures = v.exposures === 0;

  const liftIcon =
    v.lift === null ? null : v.lift > 0 ? ArrowUpRight : v.lift < 0 ? ArrowDownRight : Minus;
  const liftClass =
    v.lift === null
      ? 'text-slate-500'
      : v.lift > 0
        ? 'text-emerald-700'
        : v.lift < 0
          ? 'text-rose-700'
          : 'text-slate-500';

  return (
    <div
      className={cn(
        'relative flex flex-col gap-4 overflow-hidden rounded-xl border bg-white p-5 transition',
        isWinner ? 'shadow-brand-glow' : '',
      )}
      style={{
        borderColor: isWinner ? color.hex : '#e2e8f0',
      }}
    >
      {/* Color bar at the top — ties this card to the chart row */}
      <div
        aria-hidden
        className="absolute inset-x-0 top-0 h-1"
        style={{ backgroundColor: color.hex }}
      />

      <div className="flex items-start justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <span
            aria-hidden
            className="inline-block h-3 w-3 shrink-0 rounded-sm"
            style={{ backgroundColor: color.hex }}
          />
          <span className="truncate text-lg font-semibold text-slate-900">
            {v.variantKey}
          </span>
          {v.isControl ? (
            <span className="inline-flex items-center rounded-full bg-slate-100 px-2 py-0.5 text-xs font-semibold uppercase tracking-wide text-slate-600">
              control
            </span>
          ) : null}
          {isWinner ? (
            <span
              className="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-semibold uppercase tracking-wide text-white"
              style={{ backgroundColor: color.hex }}
            >
              ★ winner
            </span>
          ) : null}
        </div>
      </div>

      {/* Headline rate */}
      <div>
        {noExposures ? (
          <div className="text-2xl font-medium italic text-slate-400">
            Awaiting exposures
          </div>
        ) : (
          <div className="flex items-baseline gap-2">
            <span className="text-4xl font-semibold tabular-nums text-slate-900">
              {v.conversionRate !== null ? formatPct(v.conversionRate) : '—'}
            </span>
            {v.ci95 ? (
              <span className="text-sm text-slate-500">
                95% CI {formatPct(v.ci95[0])} – {formatPct(v.ci95[1])}
              </span>
            ) : null}
          </div>
        )}
      </div>

      {/* Lift + significance row */}
      {!v.isControl && !noExposures ? (
        <div className="flex flex-wrap items-center gap-2">
          {v.lift !== null && liftIcon ? (
            <span
              className={cn(
                'inline-flex items-center gap-1 rounded-full bg-slate-100 px-2.5 py-1 text-sm font-semibold tabular-nums',
                liftClass,
              )}
            >
              {(() => {
                const Icon = liftIcon;
                return <Icon aria-hidden className="h-4 w-4" />;
              })()}
              {formatLift(v.lift)}
              <span className="font-normal text-slate-500">vs control</span>
            </span>
          ) : null}
          <SignificanceBadge pValue={v.pValue} lift={v.lift} />
        </div>
      ) : null}

      {!v.isControl && !noExposures && v.pValue !== null ? (
        <p className="-mt-2 text-sm text-slate-500">
          {significanceHelper(v.pValue, v.lift)}
        </p>
      ) : null}

      {/* Footer counts */}
      <div className="grid grid-cols-2 gap-3 border-t border-slate-100 pt-4 text-sm">
        <Stat label="Exposures" value={NUM_FMT.format(v.exposures)} />
        <Stat label="Converters" value={NUM_FMT.format(v.converters)} />
        <Stat
          label="Total events"
          value={NUM_FMT.format(v.totalConversions)}
          hint="raw event count, not unique users"
        />
        <Stat
          label="Total value"
          value={
            v.totalValue === Math.round(v.totalValue)
              ? NUM_FMT.format(v.totalValue)
              : v.totalValue.toFixed(2)
          }
          hint="sum of metric value across events"
        />
      </div>
    </div>
  );
}

function Stat({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div title={hint}>
      <div className="text-xs font-semibold uppercase tracking-wide text-slate-400">
        {label}
      </div>
      <div className="mt-0.5 text-base font-medium tabular-nums text-slate-900">
        {value}
      </div>
    </div>
  );
}
