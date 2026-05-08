import type { ReactNode } from 'react';
import { Activity, Target, Users, Zap } from 'lucide-react';
import type { ResultsResponse } from '@/api/types';
import { cn } from '@/lib/cn';

const NUM_FMT = new Intl.NumberFormat();

function formatInt(n: number): string {
  return NUM_FMT.format(Math.round(n));
}

function daysBetween(startMs: number, endMs: number): number {
  const days = (endMs - startMs) / (1000 * 60 * 60 * 24);
  if (!Number.isFinite(days) || days < 0) return 0;
  return days;
}

function formatDays(n: number): string {
  if (n < 1) {
    const hours = Math.max(0, Math.round(n * 24));
    return `${hours}h`;
  }
  if (n < 10) return `${n.toFixed(1)}`;
  return `${Math.round(n)}`;
}

interface KpiCardProps {
  icon: typeof Activity;
  label: string;
  value: ReactNode;
  unit?: string;
  hint?: string;
  emphasis?: boolean;
}

function KpiCard({ icon: Icon, label, value, unit, hint, emphasis }: KpiCardProps) {
  return (
    <div
      className={cn(
        'group relative flex flex-col gap-1.5 overflow-hidden rounded-xl border p-5 transition',
        emphasis
          ? 'border-transparent bg-brand-gradient text-white shadow-brand-glow'
          : 'border-slate-200 bg-white hover:border-slate-300',
      )}
    >
      <div
        className={cn(
          'flex items-center gap-2 text-xs font-semibold uppercase tracking-wide',
          emphasis ? 'text-white/80' : 'text-slate-500',
        )}
      >
        <Icon
          aria-hidden
          className={cn('h-4 w-4', emphasis ? 'text-white/90' : 'text-slate-400')}
        />
        {label}
      </div>
      <div className="flex items-baseline gap-1.5">
        <span
          className={cn(
            'text-3xl font-semibold tabular-nums leading-none',
            emphasis ? 'text-white' : 'text-slate-900',
          )}
        >
          {value}
        </span>
        {unit ? (
          <span
            className={cn(
              'text-sm font-medium',
              emphasis ? 'text-white/80' : 'text-slate-500',
            )}
          >
            {unit}
          </span>
        ) : null}
      </div>
      {hint ? (
        <div
          className={cn(
            'mt-0.5 text-sm',
            emphasis ? 'text-white/80' : 'text-slate-500',
          )}
        >
          {hint}
        </div>
      ) : null}
    </div>
  );
}

export function HeroKPI({ results }: { results: ResultsResponse }) {
  const totalExposures = results.variants.reduce((acc, v) => acc + v.exposures, 0);
  const uniqueConverters = results.variants.reduce(
    (acc, v) => acc + v.converters,
    0,
  );
  const overallRate =
    totalExposures > 0 ? uniqueConverters / totalExposures : null;
  const days = daysBetween(results.windowStartMs, results.windowEndMs);

  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      <KpiCard
        emphasis
        icon={Users}
        label="Total exposures"
        value={formatInt(totalExposures)}
        hint={
          totalExposures > 0
            ? `Across ${results.variants.length} variants`
            : 'Waiting for first exposures…'
        }
      />
      <KpiCard
        icon={Target}
        label="Converters"
        value={formatInt(uniqueConverters)}
        hint={
          overallRate !== null
            ? `${(overallRate * 100).toFixed(2)}% overall rate`
            : 'No conversions yet'
        }
      />
      <KpiCard
        icon={Zap}
        label="Primary metric"
        value={<span className="font-mono text-2xl">{results.metricName}</span>}
        hint="Reported in the cards below"
      />
      <KpiCard
        icon={Activity}
        label="Window"
        value={formatDays(days)}
        unit={days < 1 ? '' : 'days'}
        hint={`${formatDateOnly(results.windowStartMs)} → ${formatDateOnly(results.windowEndMs)}`}
      />
    </div>
  );
}

function formatDateOnly(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}
