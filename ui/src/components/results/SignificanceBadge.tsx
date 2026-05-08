import { CheckCircle2, CircleHelp, MinusCircle, TrendingDown, TrendingUp } from 'lucide-react';
import { cn } from '@/lib/cn';

export interface SignificanceBadgeProps {
  pValue: number | null;
  /** Lift used to pick the up/down icon for "Significant". Optional. */
  lift?: number | null;
  size?: 'sm' | 'md';
}

interface Resolved {
  label: string;
  helper: string;
  className: string;
  icon: typeof CheckCircle2;
}

function resolve(pValue: number | null, lift: number | null | undefined): Resolved {
  if (pValue === null) {
    return {
      label: 'Not enough data',
      helper: 'Add more exposures to evaluate significance.',
      className: 'bg-slate-100 text-slate-600 ring-slate-200',
      icon: CircleHelp,
    };
  }
  if (pValue < 0.05) {
    const direction = (lift ?? 0) >= 0;
    return {
      label: 'Significant',
      helper: `p = ${formatPValue(pValue)} · likely a real ${direction ? 'lift' : 'drop'}.`,
      className: direction
        ? 'bg-emerald-50 text-emerald-700 ring-emerald-200'
        : 'bg-rose-50 text-rose-700 ring-rose-200',
      icon: direction ? TrendingUp : TrendingDown,
    };
  }
  if (pValue < 0.1) {
    return {
      label: 'Trending',
      helper: `p = ${formatPValue(pValue)} · suggestive but not yet significant.`,
      className: 'bg-amber-50 text-amber-800 ring-amber-200',
      icon: MinusCircle,
    };
  }
  return {
    label: 'Inconclusive',
    helper: `p = ${formatPValue(pValue)} · could be random noise.`,
    className: 'bg-slate-100 text-slate-600 ring-slate-200',
    icon: MinusCircle,
  };
}

function formatPValue(p: number): string {
  if (p < 0.001) return '<0.001';
  if (p < 0.01) return p.toFixed(3);
  return p.toFixed(2);
}

export function SignificanceBadge({ pValue, lift, size = 'md' }: SignificanceBadgeProps) {
  const r = resolve(pValue, lift);
  const Icon = r.icon;
  return (
    <span
      title={r.helper}
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full font-medium ring-1 ring-inset',
        size === 'sm' ? 'px-2 py-0.5 text-xs' : 'px-2.5 py-1 text-sm',
        r.className,
      )}
    >
      <Icon aria-hidden className={size === 'sm' ? 'h-3.5 w-3.5' : 'h-4 w-4'} />
      {r.label}
    </span>
  );
}

export function significanceHelper(
  pValue: number | null,
  lift: number | null | undefined,
): string {
  return resolve(pValue, lift).helper;
}
