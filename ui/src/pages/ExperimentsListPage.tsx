import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Plus, Search } from 'lucide-react';
import * as ExperimentsAPI from '@/api/experiments';
import type { ExperimentStatus } from '@/api/types';
import { Button } from '@/components/Button';
import { StatusBadge } from '@/components/Badge';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { ErrorAlert } from '@/components/ErrorAlert';
import { PageLoader } from '@/components/Spinner';
import { SegmentedControl } from '@/components/SegmentedControl';
import { Input } from '@/components/Input';
import { formatRelative } from '@/lib/format';

type Filter = 'all' | Exclude<ExperimentStatus, 'deleted'>;

const FILTERS: Array<{ value: Filter; label: string }> = [
  { value: 'all', label: 'All' },
  { value: 'draft', label: 'Draft' },
  { value: 'running', label: 'Running' },
  { value: 'stopped', label: 'Stopped' },
];

export function ExperimentsListPage() {
  const [filter, setFilter] = useState<Filter>('all');
  const [search, setSearch] = useState('');

  const query = useQuery({
    queryKey: ['experiments', filter],
    queryFn: () => ExperimentsAPI.listExperiments(filter === 'all' ? undefined : filter),
  });

  const filteredItems = useMemo(() => {
    const items = query.data?.items ?? [];
    const needle = search.trim().toLowerCase();
    if (!needle) return items;
    return items.filter(
      (exp) =>
        exp.key.toLowerCase().includes(needle) ||
        (exp.description?.toLowerCase().includes(needle) ?? false),
    );
  }, [query.data, search]);

  const hasItems = (query.data?.items.length ?? 0) > 0;

  return (
    <>
      <PageHeader
        title="Experiments"
        description="Create, run, and manage your A/B experiments."
        actions={
          <Link to="/experiments/new">
            <Button>
              <Plus aria-hidden className="h-5 w-5" />
              New experiment
            </Button>
          </Link>
        }
      />
      <PageBody>
        <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <SegmentedControl
            ariaLabel="Filter experiments by status"
            options={FILTERS}
            value={filter}
            onChange={setFilter}
          />
          <div className="relative sm:w-72">
            <Search
              aria-hidden
              className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400"
            />
            <Input
              type="search"
              aria-label="Search experiments"
              placeholder="Search by key or description"
              className="pl-9"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
        </div>

        {query.isLoading ? (
          <PageLoader />
        ) : query.isError ? (
          <ErrorAlert error={query.error} title="Failed to load experiments" />
        ) : hasItems && filteredItems.length > 0 ? (
          <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-slate-200 text-base">
                <thead className="bg-slate-50">
                  <tr className="text-left text-sm uppercase tracking-wide text-slate-500">
                    <th scope="col" className="px-5 py-3 font-medium">Key</th>
                    <th scope="col" className="px-5 py-3 font-medium">Status</th>
                    <th scope="col" className="px-5 py-3 font-medium">Primary metric</th>
                    <th scope="col" className="px-5 py-3 font-medium">Created</th>
                    <th scope="col" className="px-5 py-3 font-medium">Updated</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {filteredItems.map((exp) => (
                    <tr key={exp.experimentId} className="hover:bg-slate-50">
                      <td className="px-5 py-3">
                        <Link
                          to={`/experiments/${exp.experimentId}`}
                          className="font-medium text-brand-700 hover:underline"
                        >
                          {exp.key}
                        </Link>
                        {exp.description ? (
                          <div className="max-w-md truncate text-sm text-slate-500">
                            {exp.description}
                          </div>
                        ) : null}
                      </td>
                      <td className="px-5 py-3">
                        <StatusBadge status={exp.status} />
                      </td>
                      <td className="px-5 py-3 font-mono text-sm text-slate-700">
                        {exp.primaryMetric}
                      </td>
                      <td className="px-5 py-3 text-slate-500">
                        {formatRelative(exp.createdAt)}
                      </td>
                      <td className="px-5 py-3 text-slate-500">
                        {formatRelative(exp.updatedAt)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ) : hasItems ? (
          <div className="rounded-lg border border-dashed border-slate-300 bg-white px-8 py-12 text-center">
            <h3 className="text-base font-semibold text-slate-900">
              No experiments match your search
            </h3>
            <p className="mx-auto mt-1.5 max-w-md text-base text-slate-500">
              Try a different keyword or clear the search to see all
              experiments.
            </p>
          </div>
        ) : filter !== 'all' ? (
          <div className="rounded-lg border border-dashed border-slate-300 bg-white px-8 py-12 text-center">
            <h3 className="text-base font-semibold text-slate-900">
              No {filter} experiments
            </h3>
          </div>
        ) : (
          <div className="rounded-lg border border-dashed border-slate-300 bg-white px-8 py-12 text-center">
            <h3 className="text-base font-semibold text-slate-900">
              No experiments yet
            </h3>
            <div className="mt-5">
              <Link to="/experiments/new">
                <Button variant="brand">
                  <Plus aria-hidden className="h-5 w-5" />
                  Create your first experiment
                </Button>
              </Link>
            </div>
          </div>
        )}
      </PageBody>
    </>
  );
}
