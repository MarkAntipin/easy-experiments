import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Plus } from 'lucide-react';
import * as ExperimentsAPI from '@/api/experiments';
import type { ExperimentStatus } from '@/api/types';
import { Button } from '@/components/Button';
import { StatusBadge } from '@/components/Badge';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { ErrorAlert } from '@/components/ErrorAlert';
import { PageLoader } from '@/components/Spinner';
import { ConceptPanel } from '@/components/teaching';
import { formatRelative } from '@/lib/format';
import { cn } from '@/lib/cn';

type Filter = 'all' | Exclude<ExperimentStatus, 'deleted'>;

const FILTERS: Array<{ value: Filter; label: string }> = [
  { value: 'all', label: 'All' },
  { value: 'draft', label: 'Draft' },
  { value: 'running', label: 'Running' },
  { value: 'stopped', label: 'Stopped' },
];

export function ExperimentsListPage() {
  const [filter, setFilter] = useState<Filter>('all');

  const query = useQuery({
    queryKey: ['experiments', filter],
    queryFn: () => ExperimentsAPI.listExperiments(filter === 'all' ? undefined : filter),
  });

  return (
    <>
      <PageHeader
        title="Experiments"
        description="Create, run, and manage your A/B experiments."
        actions={
          <Link to="/experiments/new">
            <Button>
              <Plus className="h-4 w-4" />
              New experiment
            </Button>
          </Link>
        }
      />
      <PageBody>
        <div className="mb-4 inline-flex rounded-md bg-slate-100 p-1">
          {FILTERS.map((f) => (
            <button
              key={f.value}
              type="button"
              onClick={() => setFilter(f.value)}
              className={cn(
                'rounded px-3 py-1 text-sm font-medium transition',
                filter === f.value
                  ? 'bg-white text-slate-900 shadow-sm'
                  : 'text-slate-500 hover:text-slate-800',
              )}
            >
              {f.label}
            </button>
          ))}
        </div>

        {query.isLoading ? (
          <PageLoader />
        ) : query.isError ? (
          <ErrorAlert error={query.error} title="Failed to load experiments" />
        ) : query.data && query.data.length > 0 ? (
          <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
            <table className="min-w-full divide-y divide-slate-200 text-sm">
              <thead className="bg-slate-50">
                <tr className="text-left text-xs uppercase tracking-wide text-slate-500">
                  <th className="px-4 py-2 font-medium">Key</th>
                  <th className="px-4 py-2 font-medium">Status</th>
                  <th className="px-4 py-2 font-medium">Primary metric</th>
                  <th className="px-4 py-2 font-medium">Created</th>
                  <th className="px-4 py-2 font-medium">Updated</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {query.data.map((exp) => (
                  <tr key={exp.experimentId} className="hover:bg-slate-50">
                    <td className="px-4 py-2.5">
                      <Link
                        to={`/experiments/${exp.experimentId}`}
                        className="font-medium text-brand-700 hover:underline"
                      >
                        {exp.key}
                      </Link>
                      {exp.description ? (
                        <div className="max-w-md truncate text-xs text-slate-500">
                          {exp.description}
                        </div>
                      ) : null}
                    </td>
                    <td className="px-4 py-2.5">
                      <StatusBadge status={exp.status} />
                    </td>
                    <td className="px-4 py-2.5 font-mono text-xs text-slate-700">
                      {exp.primaryMetric}
                    </td>
                    <td className="px-4 py-2.5 text-slate-500">
                      {formatRelative(exp.createdAt)}
                    </td>
                    <td className="px-4 py-2.5 text-slate-500">
                      {formatRelative(exp.updatedAt)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="flex flex-col gap-5">
            <ConceptPanel />
            <div className="rounded-lg border border-dashed border-slate-300 bg-white px-6 py-8 text-center">
              <h3 className="text-sm font-semibold text-slate-900">
                Ready to try one?
              </h3>
              <p className="mx-auto mt-1 max-w-md text-sm text-slate-500">
                Pick a feature you&rsquo;d like to test, define a variant or
                two, and start measuring. You can stop the experiment at any
                time.
              </p>
              <div className="mt-4">
                <Link to="/experiments/new">
                  <Button>
                    <Plus className="h-4 w-4" />
                    Create your first experiment
                  </Button>
                </Link>
              </div>
            </div>
          </div>
        )}
      </PageBody>
    </>
  );
}
