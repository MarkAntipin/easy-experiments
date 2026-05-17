import { useMemo } from 'react';
import { Link, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { ArrowLeft, RefreshCcw } from 'lucide-react';
import * as ExperimentsAPI from '@/api/experiments';
import type { ResultsResponse, VariantResult } from '@/api/types';
import { Button } from '@/components/Button';
import { ErrorAlert } from '@/components/ErrorAlert';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { Spinner } from '@/components/Spinner';
import { StatusBadge } from '@/components/Badge';
import { SrmBanner } from '@/components/results/SrmBanner';
import { TimeSeriesChart } from '@/components/results/TimeSeriesChart';
import { VariantsTable } from '@/components/results/VariantsTable';

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;
const NUM = new Intl.NumberFormat();

function fmtDuration(ms: number): string {
  if (ms <= 0) return '0h';
  if (ms < DAY_MS) return `${Math.max(1, Math.round(ms / HOUR_MS))}h`;
  const days = ms / DAY_MS;
  if (days < 10) return `${days.toFixed(1)} days`;
  return `${Math.round(days)} days`;
}

export function ExperimentResultsPage() {
  const { id = '' } = useParams<{ id: string }>();

  const detailQuery = useQuery({
    queryKey: ['experiment', id],
    queryFn: () => ExperimentsAPI.getExperiment(id),
    enabled: Boolean(id),
  });

  const detail = detailQuery.data;

  // Drives the "Running" stat card. Computed client-side so a draft experiment
  // shows 0h rather than "days since creation".
  const durationMs = useMemo(() => {
    if (!detail?.startedAt) return 0;
    const end = detail.stoppedAt ?? Date.now();
    return Math.max(0, end - detail.startedAt);
  }, [detail]);

  const resultsQuery = useQuery<ResultsResponse>({
    queryKey: ['experiment-results', id],
    queryFn: () => ExperimentsAPI.getExperimentResults(id),
    enabled: Boolean(id),
    refetchInterval: 30_000,
    placeholderData: (prev) => prev,
  });

  const results = resultsQuery.data;
  const variantKeyOrder: readonly string[] = useMemo(
    () => detail?.variants.map((v) => v.key) ?? [],
    [detail],
  );
  const sortedVariants: VariantResult[] = useMemo(() => {
    if (!results) return [];
    const order = new Map<string, number>();
    variantKeyOrder.forEach((k, i) => order.set(k, i));
    return [...results.variants].sort((a, b) => {
      if (a.isControl && !b.isControl) return -1;
      if (!a.isControl && b.isControl) return 1;
      return (order.get(a.variantKey) ?? 999) - (order.get(b.variantKey) ?? 999);
    });
  }, [results, variantKeyOrder]);

  const totalExposures = useMemo(
    () => (results ? results.variants.reduce((a, v) => a + v.exposures, 0) : 0),
    [results],
  );

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
        description={<StatusBadge status={detail.status} />}
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
          {resultsQuery.isError ? (
            <ErrorAlert
              error={resultsQuery.error}
              title="Failed to load results"
            />
          ) : null}

          {results ? (
            <>
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                <StatCard
                  label="Total exposures"
                  value={NUM.format(totalExposures)}
                />
                <StatCard label="Running" value={fmtDuration(durationMs)} />
                <StatCard label="Primary metric" value={detail.primaryMetric} mono />
              </div>
              <SrmBanner srm={results.srm} variantKeyOrder={variantKeyOrder} />
              <VariantsTable
                variants={sortedVariants}
                variantKeyOrder={variantKeyOrder}
              />
              <TimeSeriesChart
                buckets={results.timeSeries}
                variantKeyOrder={variantKeyOrder}
                granularity={results.granularity}
              />
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

function StatCard({
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
        className={
          mono
            ? 'mt-0.5 truncate font-mono text-base font-semibold text-slate-900'
            : 'mt-0.5 truncate text-lg font-semibold tabular-nums text-slate-900'
        }
        title={value}
      >
        {value}
      </div>
    </div>
  );
}
