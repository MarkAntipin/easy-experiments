import type { ReactNode } from 'react';
import {
  Award,
  AlertOctagon,
  HelpCircle,
  Hourglass,
  type LucideIcon,
} from 'lucide-react';
import type { ResultsResponse, VariantResult } from '@/api/types';
import { variantColorByKey } from '@/lib/variantColors';
import { cn } from '@/lib/cn';

const Z_ALPHA = 1.96;
const Z_BETA = 0.84;

export type VerdictKind =
  | { kind: 'no_data' }
  | { kind: 'winner'; variant: VariantResult; control: VariantResult }
  | { kind: 'harm'; variant: VariantResult; control: VariantResult }
  | {
      kind: 'inconclusive';
      mdeAbs: number | null;
      bestPositive: VariantResult | null;
      requiredNPerArm: number | null;
    };

export function selectVerdict(results: ResultsResponse): VerdictKind {
  const total = results.variants.reduce((a, v) => a + v.exposures, 0);
  if (total === 0) return { kind: 'no_data' };

  const control = results.variants.find((v) => v.isControl) ?? null;

  const significant = results.variants.filter(
    (v) =>
      !v.isControl &&
      v.exposures > 0 &&
      v.pValue !== null &&
      v.pValue < 0.05 &&
      v.lift !== null,
  );

  if (control) {
    const worstHarm = significant
      .filter((v) => (v.lift ?? 0) < 0)
      .sort((a, b) => (a.lift ?? 0) - (b.lift ?? 0))[0];
    if (worstHarm) {
      return { kind: 'harm', variant: worstHarm, control };
    }

    const bestWinner = significant
      .filter((v) => (v.lift ?? 0) > 0)
      .sort((a, b) => (b.lift ?? 0) - (a.lift ?? 0))[0];
    if (bestWinner) {
      return { kind: 'winner', variant: bestWinner, control };
    }
  }

  // Inconclusive — compute MDE and (optionally) required-N to confirm the
  // strongest non-significant positive lift we're already seeing.
  const treatments = results.variants.filter((v) => !v.isControl);
  const nControl = control?.exposures ?? 0;
  const minNTreat =
    treatments.length > 0 ? Math.min(...treatments.map((v) => v.exposures)) : 0;
  const nPerArm = Math.min(nControl, minNTreat);
  const p = control?.conversionRate ?? null;
  let mdeAbs: number | null = null;
  if (nPerArm > 0 && p !== null && p > 0 && p < 1) {
    mdeAbs = (Z_ALPHA + Z_BETA) * Math.sqrt((2 * p * (1 - p)) / nPerArm);
  }

  const bestPositive =
    treatments
      .filter((v) => (v.lift ?? 0) > 0 && v.conversionRate !== null)
      .sort((a, b) => (b.lift ?? 0) - (a.lift ?? 0))[0] ?? null;

  let requiredNPerArm: number | null = null;
  if (bestPositive && control && p !== null && p > 0 && p < 1) {
    const targetDiffAbs = (bestPositive.conversionRate ?? 0) - p;
    if (targetDiffAbs > 0) {
      requiredNPerArm = Math.ceil(
        (2 * p * (1 - p) * (Z_ALPHA + Z_BETA) * (Z_ALPHA + Z_BETA)) /
          (targetDiffAbs * targetDiffAbs),
      );
    }
  }

  return { kind: 'inconclusive', mdeAbs, bestPositive, requiredNPerArm };
}

export function selectWinnerKey(results: ResultsResponse): string | null {
  const v = selectVerdict(results);
  return v.kind === 'winner' ? v.variant.variantKey : null;
}

function fmtRelLift(n: number, digits = 1): string {
  const sign = n > 0 ? '+' : '';
  return `${sign}${(n * 100).toFixed(digits)}%`;
}

function fmtAbsRate(n: number, digits = 2): string {
  return `${(n * 100).toFixed(digits)}%`;
}

function fmtPP(n: number, digits = 2): string {
  const sign = n > 0 ? '+' : '';
  return `${sign}${(n * 100).toFixed(digits)} pp`;
}

function fmtPValue(p: number): string {
  if (p < 0.001) return '<0.001';
  if (p < 0.01) return p.toFixed(3);
  return p.toFixed(2);
}

const NUM = new Intl.NumberFormat();

function fmtInt(n: number): string {
  return NUM.format(Math.round(n));
}

function fmtDays(startMs: number, endMs: number): string {
  const days = Math.max(0, (endMs - startMs) / (1000 * 60 * 60 * 24));
  if (days < 1) return `${Math.round(days * 24)}h`;
  if (days < 10) return `${days.toFixed(1)} days`;
  return `${Math.round(days)} days`;
}

interface DiffCI {
  diff: number;
  low: number;
  high: number;
}

function absoluteDiffCI(
  treatment: VariantResult,
  control: VariantResult,
): DiffCI | null {
  const pt = treatment.conversionRate;
  const pc = control.conversionRate;
  if (pt === null || pc === null) return null;
  if (treatment.exposures === 0 || control.exposures === 0) return null;
  const diff = pt - pc;
  const se = Math.sqrt(
    (pt * (1 - pt)) / treatment.exposures +
      (pc * (1 - pc)) / control.exposures,
  );
  return { diff, low: diff - Z_ALPHA * se, high: diff + Z_ALPHA * se };
}

export function Verdict({
  results,
  variantKeyOrder,
}: {
  results: ResultsResponse;
  variantKeyOrder: readonly string[];
}) {
  const verdict = selectVerdict(results);
  const totalExposures = results.variants.reduce((a, v) => a + v.exposures, 0);

  return (
    <div className="flex flex-col gap-3">
      <VerdictCard verdict={verdict} variantKeyOrder={variantKeyOrder} />
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        <SupportStat label="Total exposures" value={fmtInt(totalExposures)} />
        <SupportStat
          label="Running"
          value={fmtDays(results.windowStartMs, results.windowEndMs)}
        />
        <SupportStat label="Primary metric" value={results.metricName} mono />
      </div>
    </div>
  );
}

function VerdictCard({
  verdict,
  variantKeyOrder,
}: {
  verdict: VerdictKind;
  variantKeyOrder: readonly string[];
}) {
  if (verdict.kind === 'no_data') {
    return (
      <Shell tone="neutral" icon={Hourglass} badge="Awaiting data">
        <Headline>No exposures recorded yet</Headline>
        <Body>
          Once your code calls{' '}
          <code className="font-mono text-sm">/api/v1/experiments/evaluate</code>
          , the verdict will appear here within a few seconds.
        </Body>
      </Shell>
    );
  }

  if (verdict.kind === 'winner') {
    const v = verdict.variant;
    const c = verdict.control;
    const color = variantColorByKey(variantKeyOrder, v.variantKey);
    const ci = absoluteDiffCI(v, c);
    return (
      <Shell tone="positive" icon={Award} badge="Significant winner">
        <Headline>
          <span style={{ color: color.hex }}>{v.variantKey}</span> is winning
        </Headline>
        <Stats
          relLift={v.lift}
          pValue={v.pValue}
          treatmentRate={v.conversionRate}
          controlRate={c.conversionRate}
          treatmentKey={v.variantKey}
          controlKey={c.variantKey}
          ci={ci}
        />
        <Body>
          The lift is unlikely to be noise. Promoting{' '}
          <span className="font-semibold">{v.variantKey}</span> is the safe
          call.
        </Body>
      </Shell>
    );
  }

  if (verdict.kind === 'harm') {
    const v = verdict.variant;
    const c = verdict.control;
    const ci = absoluteDiffCI(v, c);
    return (
      <Shell tone="negative" icon={AlertOctagon} badge="Stop — treatment is hurting">
        <Headline>
          <span className="font-mono">{v.variantKey}</span> is significantly
          worse than control
        </Headline>
        <Stats
          relLift={v.lift}
          pValue={v.pValue}
          treatmentRate={v.conversionRate}
          controlRate={c.conversionRate}
          treatmentKey={v.variantKey}
          controlKey={c.variantKey}
          ci={ci}
        />
        <Body>
          Continuing the test is costing conversions. Stop or revert this
          variant.
        </Body>
      </Shell>
    );
  }

  return (
    <Shell tone="neutral" icon={HelpCircle} badge="Inconclusive">
      <Headline>No variant has separated from control</Headline>
      {verdict.bestPositive ? (
        <Body>
          Largest observed lift:{' '}
          <span className="font-semibold tabular-nums">
            {fmtRelLift(verdict.bestPositive.lift ?? 0)}
          </span>{' '}
          (<span className="font-mono">{verdict.bestPositive.variantKey}</span>
          ), but the result could still be noise.
        </Body>
      ) : (
        <Body>No treatment is currently outperforming control.</Body>
      )}
      {verdict.mdeAbs !== null ? (
        <Body>
          Detectable lift at 80% power, current sample size:{' '}
          <span className="font-semibold tabular-nums">
            ±{(verdict.mdeAbs * 100).toFixed(2)} pp
          </span>
          . Effects smaller than that need more data to surface.
        </Body>
      ) : null}
      {verdict.requiredNPerArm !== null && verdict.bestPositive ? (
        <Body>
          To confirm the{' '}
          <span className="tabular-nums">
            {fmtRelLift(verdict.bestPositive.lift ?? 0)}
          </span>{' '}
          you're seeing, you'd need roughly{' '}
          <span className="font-semibold tabular-nums">
            {fmtInt(verdict.requiredNPerArm)}
          </span>{' '}
          exposures per arm.
        </Body>
      ) : null}
    </Shell>
  );
}

type Tone = 'positive' | 'negative' | 'neutral';

function Shell({
  tone,
  icon: Icon,
  badge,
  children,
}: {
  tone: Tone;
  icon: LucideIcon;
  badge: string;
  children: ReactNode;
}) {
  const toneClass =
    tone === 'positive'
      ? 'border-transparent bg-brand-gradient text-white shadow-brand-glow'
      : tone === 'negative'
        ? 'border-rose-200 bg-rose-50 text-rose-950'
        : 'border-slate-200 bg-white text-slate-900';
  const badgeClass =
    tone === 'positive'
      ? 'bg-white/15 text-white'
      : tone === 'negative'
        ? 'bg-rose-100 text-rose-900'
        : 'bg-slate-100 text-slate-700';
  const iconClass =
    tone === 'positive'
      ? 'text-white/90'
      : tone === 'negative'
        ? 'text-rose-700'
        : 'text-slate-500';
  return (
    <div className={cn('relative overflow-hidden rounded-xl border p-6', toneClass)}>
      <div className="flex items-center gap-2">
        <Icon aria-hidden className={cn('h-5 w-5', iconClass)} />
        <span
          className={cn(
            'inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wide',
            badgeClass,
          )}
        >
          {badge}
        </span>
      </div>
      <div className="mt-3 flex flex-col gap-2">{children}</div>
    </div>
  );
}

function Headline({ children }: { children: ReactNode }) {
  return <div className="text-2xl font-semibold leading-tight">{children}</div>;
}

function Body({ children }: { children: ReactNode }) {
  return <p className="text-base opacity-90">{children}</p>;
}

function Stats({
  relLift,
  pValue,
  treatmentRate,
  controlRate,
  treatmentKey,
  controlKey,
  ci,
}: {
  relLift: number | null;
  pValue: number | null;
  treatmentRate: number | null;
  controlRate: number | null;
  treatmentKey: string;
  controlKey: string;
  ci: DiffCI | null;
}) {
  return (
    <div className="mt-1 flex flex-wrap items-baseline gap-x-5 gap-y-1">
      {relLift !== null ? (
        <span className="text-3xl font-semibold tabular-nums leading-none">
          {fmtRelLift(relLift)}
        </span>
      ) : null}
      {ci ? (
        <span className="text-sm tabular-nums opacity-80">
          Δ {fmtPP(ci.diff)} · 95% CI [{fmtPP(ci.low)}, {fmtPP(ci.high)}]
        </span>
      ) : null}
      {pValue !== null ? (
        <span className="text-sm tabular-nums opacity-80">
          p = {fmtPValue(pValue)}
        </span>
      ) : null}
      {treatmentRate !== null && controlRate !== null ? (
        <span className="text-sm tabular-nums opacity-80">
          <span className="font-mono">{treatmentKey}</span>{' '}
          {fmtAbsRate(treatmentRate)} vs <span className="font-mono">{controlKey}</span>{' '}
          {fmtAbsRate(controlRate)}
        </span>
      ) : null}
    </div>
  );
}

function SupportStat({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="rounded-lg border border-slate-200 bg-white px-4 py-3">
      <div className="text-xs font-semibold uppercase tracking-wide text-slate-500">
        {label}
      </div>
      <div
        className={cn(
          'mt-0.5 text-lg font-semibold text-slate-900 tabular-nums',
          mono && 'font-mono text-base',
        )}
      >
        {value}
      </div>
    </div>
  );
}
