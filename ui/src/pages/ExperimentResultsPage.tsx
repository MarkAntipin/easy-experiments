import { useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { ArrowLeft, RefreshCcw } from 'lucide-react';
import * as ExperimentsAPI from '@/api/experiments';
import type { Granularity, ResultsResponse, VariantResult } from '@/api/types';
import { Button } from '@/components/Button';
import { ErrorAlert } from '@/components/ErrorAlert';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { SegmentedControl } from '@/components/SegmentedControl';
import { Spinner } from '@/components/Spinner';
import { StatusBadge } from '@/components/Badge';
import { ConversionChart } from '@/components/results/ConversionChart';
import {
  DateRangePicker,
  presetToRange,
  type RangePreset,
} from '@/components/results/DateRangePicker';
import { HeroKPI } from '@/components/results/HeroKPI';
import { SrmBanner } from '@/components/results/SrmBanner';
import { TimeSeriesChart } from '@/components/results/TimeSeriesChart';
import { VariantStatCard } from '@/components/results/VariantStatCard';

const GRANULARITY_OPTIONS = [
  { value: 'day' as const, label: 'Day' },
  { value: 'hour' as const, label: 'Hour' },
];

/**
 * Pick the "winner" — the non-control variant with the highest conversion
 * rate AND p < 0.05 AND positive lift. Returns the variant key or null.
 */
function pickWinner(variants: VariantResult[]): string | null {
  const candidates = variants
    .filter(
      (v) =>
        !v.isControl &&
        v.exposures > 0 &&
        v.pValue !== null &&
        v.pValue < 0.05 &&
        (v.lift ?? 0) > 0,
    )
    .sort((a, b) => (b.conversionRate ?? 0) - (a.conversionRate ?? 0));
  return candidates[0]?.variantKey ?? null;
}

export function ExperimentResultsPage() {
  const { id = '' } = useParams<{ id: string }>();
  const [preset, setPreset] = useState<RangePreset>('all');
  const [granularity, setGranularity] = useState<Granularity>('day');

  // The detail query gives us experiment metadata to render alongside (key,
  // status, variant order). We also need it as a fallback if /results errors.
  const detailQuery = useQuery({
    queryKey: ['experiment', id],
    queryFn: () => ExperimentsAPI.getExperiment(id),
    enabled: Boolean(id),
  });

  const range = useMemo(() => presetToRange(preset), [preset]);

  const resultsQuery = useQuery<ResultsResponse>({
    queryKey: ['experiment-results', id, preset, granularity],
    queryFn: () =>
      ExperimentsAPI.getExperimentResults(id, {
        start: range.start,
        end: range.end,
        granularity,
      }),
    enabled: Boolean(id),
    refetchInterval: 30_000,
    placeholderData: (prev) => prev,
  });

  const detail = detailQuery.data;
  const results = resultsQuery.data;
  const variantKeyOrder: readonly string[] = useMemo(
    () => detail?.variants.map((v) => v.key) ?? [],
    [detail],
  );
  const sortedVariants: VariantResult[] = useMemo(() => {
    if (!results) return [];
    // Mirror the chart's row order: control first, then variantKeyOrder.
    const order = new Map<string, number>();
    variantKeyOrder.forEach((k, i) => order.set(k, i));
    return [...results.variants].sort((a, b) => {
      if (a.isControl && !b.isControl) return -1;
      if (!a.isControl && b.isControl) return 1;
      return (order.get(a.variantKey) ?? 999) - (order.get(b.variantKey) ?? 999);
    });
  }, [results, variantKeyOrder]);

  const winner = useMemo(
    () => (results ? pickWinner(results.variants) : null),
    [results],
  );

  // Initial load: nothing to render yet.
  if (detailQuery.isLoading) {
    return (
      <>
        <PageHeader title="Results" />
        <PageBody>
          <div className="flex h-64 items-center justify-center">
            <Spinner className="h-7 w-7" />
          </div>
        </PageBody>
      </>
    );
  }

  if (detailQuery.isError || !detail) {
    return (
      <>
        <PageHeader title="Results" />
        <PageBody>
          <ErrorAlert error={detailQuery.error} title="Failed to load experiment" />
        </PageBody>
      </>
    );
  }

  return (
    <>
      <PageHeader
        title={`${detail.key} · Results`}
        description={
          <span className="flex items-center gap-2">
            <StatusBadge status={detail.status} />
            <span className="font-mono text-sm text-slate-500">
              {detail.experimentId}
            </span>
          </span>
        }
        actions={
          <div className="flex items-center gap-2">
            <Link
              to={`/experiments/${detail.experimentId}`}
              className="inline-flex items-center gap-1.5 text-base font-medium text-slate-600 hover:text-slate-900"
            >
              <ArrowLeft aria-hidden className="h-5 w-5" />
              Back to experiment
            </Link>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => resultsQuery.refetch()}
              loading={resultsQuery.isFetching}
              aria-label="Refresh results"
            >
              <RefreshCcw aria-hidden className="h-4 w-4" />
              Refresh
            </Button>
          </div>
        }
      />

      <PageBody>
        <div className="flex flex-col gap-6">
          {/* Controls row */}
          <div className="flex flex-col gap-3 rounded-lg border border-slate-200 bg-white px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex items-center gap-3">
              <span className="text-sm font-semibold uppercase tracking-wide text-slate-500">
                Window
              </span>
              <DateRangePicker value={preset} onChange={setPreset} />
            </div>
            <div className="flex items-center gap-3">
              <span className="text-sm font-semibold uppercase tracking-wide text-slate-500">
                Granularity
              </span>
              <SegmentedControl<Granularity>
                ariaLabel="Granularity"
                options={GRANULARITY_OPTIONS}
                value={granularity}
                onChange={setGranularity}
                size="sm"
              />
            </div>
          </div>

          {resultsQuery.isError ? (
            <ErrorAlert
              error={resultsQuery.error}
              title="Failed to load results"
            />
          ) : null}

          {results ? (
            <>
              <HeroKPI results={results} />
              <SrmBanner srm={results.srm} variantKeyOrder={variantKeyOrder} />

              {sortedVariants.length === 0 ||
              sortedVariants.every((v) => v.exposures === 0) ? (
                <EmptyResultsCard
                  status={detail.status}
                  hasStarted={detail.startedAt !== null}
                />
              ) : (
                <>
                  <ConversionChart
                    variants={sortedVariants}
                    variantKeyOrder={variantKeyOrder}
                  />
                  <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-3">
                    {sortedVariants.map((v) => (
                      <VariantStatCard
                        key={v.variantKey}
                        variant={v}
                        variantKeyOrder={variantKeyOrder}
                        isWinner={v.variantKey === winner}
                      />
                    ))}
                  </div>
                  <TimeSeriesChart
                    buckets={results.timeSeries}
                    variantKeyOrder={variantKeyOrder}
                    granularity={results.granularity}
                  />
                  <FootnoteCard results={results} />
                </>
              )}
            </>
          ) : resultsQuery.isLoading ? (
            <div className="flex h-64 items-center justify-center rounded-lg border border-slate-200 bg-white">
              <Spinner className="h-7 w-7" />
            </div>
          ) : null}
        </div>
      </PageBody>
    </>
  );
}

function EmptyResultsCard({
  status,
  hasStarted,
}: {
  status: string;
  hasStarted: boolean;
}) {
  return (
    <div className="rounded-lg border border-dashed border-slate-300 bg-white px-8 py-14 text-center">
      <h3 className="text-lg font-semibold text-slate-900">
        No exposures yet
      </h3>
      <p className="mx-auto mt-2 max-w-md text-base text-slate-500">
        {!hasStarted
          ? `This experiment is in '${status}' status. Start it and call /evaluate from your code to begin recording exposures.`
          : 'Once your code calls POST /api/v1/experiments/evaluate for this experiment, results will appear here within a few seconds.'}
      </p>
    </div>
  );
}

function FootnoteCard({ results }: { results: ResultsResponse }) {
  const fmt = (ms: number) =>
    new Date(ms).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  return (
    <div className="rounded-lg border border-slate-200 bg-slate-50/60 px-5 py-4 text-sm text-slate-600">
      <div className="font-semibold text-slate-700">How these numbers are computed</div>
      <ul className="mt-1.5 list-disc space-y-1 pl-5">
        <li>
          Window: <span className="tabular-nums">{fmt(results.windowStartMs)}</span>{' '}
          → <span className="tabular-nums">{fmt(results.windowEndMs)}</span>
        </li>
        <li>
          A conversion is attributed to a variant only if it happened{' '}
          <em>after</em> the user's first exposure to that variant.
        </li>
        <li>
          Confidence intervals are 95% Wilson score intervals; lift is computed
          relative to the control variant.
        </li>
        <li>
          Significance is a two-sided two-proportion z-test with a pooled
          standard error.
        </li>
      </ul>
    </div>
  );
}
