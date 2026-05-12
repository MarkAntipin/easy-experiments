import { useState } from 'react';
import type { ReactNode } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { ArrowLeft, BarChart3, Pencil, Play, Square, Trash2 } from 'lucide-react';
import * as ExperimentsAPI from '@/api/experiments';
import { ApiError } from '@/api/client';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/Button';
import { StatusBadge } from '@/components/Badge';
import { ErrorAlert } from '@/components/ErrorAlert';
import { PageLoader } from '@/components/Spinner';
import { Dialog } from '@/components/Dialog';
import { operatorLabels } from '@/forms/experimentSchema';
import { formatTimestamp } from '@/lib/format';

export function ExperimentDetailPage() {
  const { id = '' } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [confirmStart, setConfirmStart] = useState(false);
  const [confirmStop, setConfirmStop] = useState(false);

  const query = useQuery({
    queryKey: ['experiment', id],
    queryFn: () => ExperimentsAPI.getExperiment(id),
    enabled: Boolean(id),
  });

  const onSuccess = (msg: string) => {
    toast.success(msg);
    queryClient.invalidateQueries({ queryKey: ['experiments'] });
    queryClient.invalidateQueries({ queryKey: ['experiment', id] });
  };
  const onError = (err: unknown) => {
    toast.error(err instanceof ApiError ? err.message : 'Action failed');
  };

  const startMutation = useMutation({
    mutationFn: () => ExperimentsAPI.startExperiment(id),
    onSuccess: () => {
      onSuccess('Experiment started');
      setConfirmStart(false);
    },
    onError,
  });
  const stopMutation = useMutation({
    mutationFn: () => ExperimentsAPI.stopExperiment(id),
    onSuccess: () => {
      onSuccess('Experiment stopped');
      setConfirmStop(false);
    },
    onError,
  });
  const deleteMutation = useMutation({
    mutationFn: () => ExperimentsAPI.deleteExperiment(id),
    onSuccess: () => {
      toast.success('Experiment deleted');
      queryClient.invalidateQueries({ queryKey: ['experiments'] });
      navigate('/experiments');
    },
    onError,
  });

  if (query.isLoading) return <PageLoader />;
  if (query.isError) {
    return (
      <>
        <PageHeader title="Experiment" />
        <PageBody>
          <ErrorAlert error={query.error} title="Failed to load experiment" />
        </PageBody>
      </>
    );
  }
  const exp = query.data;
  if (!exp) return null;

  return (
    <>
      <PageHeader
        title={exp.key}
        description={
          <span className="flex items-center gap-2">
            <StatusBadge status={exp.status} />
            <span className="font-mono text-sm text-slate-500">{exp.experimentId}</span>
          </span>
        }
        actions={
          <div className="flex items-center gap-2">
            <Link
              to="/experiments"
              className="inline-flex items-center gap-1.5 text-base font-medium text-slate-600 hover:text-slate-900"
            >
              <ArrowLeft aria-hidden className="h-5 w-5" />
              Back
            </Link>
            {exp.status === 'running' || exp.status === 'stopped' ? (
              <Link to={`/experiments/${exp.experimentId}/results`}>
                <Button variant="brand">
                  <BarChart3 aria-hidden className="h-5 w-5" />
                  Results
                </Button>
              </Link>
            ) : null}
            <Link to={`/experiments/${exp.experimentId}/edit`}>
              <Button variant="secondary">
                <Pencil aria-hidden className="h-5 w-5" />
                Edit
              </Button>
            </Link>
            {exp.status === 'draft' ? (
              <Button onClick={() => setConfirmStart(true)}>
                <Play aria-hidden className="h-5 w-5" />
                Start
              </Button>
            ) : null}
            {exp.status === 'running' ? (
              <Button
                variant="secondary"
                onClick={() => setConfirmStop(true)}
              >
                <Square aria-hidden className="h-5 w-5" />
                Stop
              </Button>
            ) : null}
            <Button variant="danger" onClick={() => setConfirmDelete(true)}>
              <Trash2 aria-hidden className="h-5 w-5" />
              Delete
            </Button>
          </div>
        }
      />
      <PageBody>
        <div className="flex flex-col gap-6">
          <section className="grid gap-5 rounded-lg border border-slate-200 bg-white p-6 sm:grid-cols-3">
            <Meta label="Primary metric" value={exp.primaryMetric} mono />
            <Meta label="Created" value={formatTimestamp(exp.createdAt)} />
            <Meta label="Updated" value={formatTimestamp(exp.updatedAt)} />
            <Meta label="Started" value={formatTimestamp(exp.startedAt)} />
            <Meta label="Stopped" value={formatTimestamp(exp.stoppedAt)} />
            <Meta label="Description" value={exp.description ?? '—'} />
          </section>

          <section className="rounded-lg border border-slate-200 bg-white p-6">
            <h2 className="mb-4 text-base font-semibold text-slate-900">Variants</h2>
            <div className="overflow-hidden rounded border border-slate-200">
              <div className="overflow-x-auto">
                <table className="min-w-full divide-y divide-slate-200 text-base">
                  <thead className="bg-slate-50 text-left text-sm uppercase tracking-wide text-slate-500">
                    <tr>
                      <th scope="col" className="px-5 py-3 font-medium">Key</th>
                      <th scope="col" className="px-5 py-3 font-medium">Control</th>
                      <th scope="col" className="px-5 py-3 font-medium">Config</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-100">
                    {exp.variants.map((v) => (
                      <tr key={v.key}>
                        <td className="px-5 py-3 font-mono text-sm text-slate-800">
                          {v.key}
                        </td>
                        <td className="px-5 py-3">
                          {v.isControl ? (
                            <span className="rounded-full bg-brand-50 px-2.5 py-0.5 text-sm font-medium text-brand-700 ring-1 ring-inset ring-brand-200">
                              control
                            </span>
                          ) : (
                            <span className="text-sm text-slate-400">—</span>
                          )}
                        </td>
                        <td className="px-5 py-3">
                          <pre className="max-w-md overflow-x-auto rounded bg-ink-50 px-2.5 py-1.5 font-mono text-sm text-ink-700">
                            {JSON.stringify(v.config ?? {}, null, 2)}
                          </pre>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </section>

          <section className="rounded-lg border border-slate-200 bg-white p-6">
            <h2 className="mb-4 text-base font-semibold text-slate-900">Segments</h2>
            <div className="flex flex-col gap-3">
              {exp.segments
                .slice()
                .sort((a, b) => a.priority - b.priority)
                .map((seg) => (
                  <div
                    key={seg.priority}
                    className="flex flex-col gap-3 rounded-md border border-slate-200 p-4"
                  >
                    <div className="flex items-center gap-3 text-base">
                      <span className="rounded bg-ink-100 px-2.5 py-0.5 font-mono text-sm text-ink-700">
                        priority {seg.priority}
                      </span>
                      <span className="text-slate-600">
                        rollout {seg.rolloutPercent}%
                      </span>
                    </div>
                    {seg.constraints.length > 0 ? (
                      <SegmentBlock title="Constraints">
                        <ul className="flex flex-col gap-1">
                          {seg.constraints.map((c, i) => (
                            <li key={i} className="font-mono text-slate-700">
                              {c.property}{' '}
                              <span className="text-slate-500">
                                {operatorLabels[c.operator]}
                              </span>{' '}
                              {JSON.stringify(c.value)}
                            </li>
                          ))}
                        </ul>
                      </SegmentBlock>
                    ) : null}
                    <SegmentBlock title="Distributions">
                      <ul className="flex flex-wrap gap-2">
                        {seg.distributions.map((d) => (
                          <li
                            key={d.variantKey}
                            className="rounded bg-ink-100 px-2.5 py-0.5 font-mono text-ink-700"
                          >
                            {d.variantKey}: {d.percent}%
                          </li>
                        ))}
                      </ul>
                    </SegmentBlock>
                  </div>
                ))}
            </div>
          </section>
        </div>
      </PageBody>

      <Dialog
        open={confirmStart}
        onClose={() => setConfirmStart(false)}
        title="Start experiment?"
        description="Once started, real users will begin seeing the variants. You can stop the experiment at any time, but you can't return it to draft."
      >
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => setConfirmStart(false)}>
            Cancel
          </Button>
          <Button
            loading={startMutation.isPending}
            onClick={() => startMutation.mutate()}
          >
            <Play aria-hidden className="h-5 w-5" />
            Start now
          </Button>
        </div>
      </Dialog>

      <Dialog
        open={confirmStop}
        onClose={() => setConfirmStop(false)}
        title="Stop experiment?"
        description="Users will stop being assigned variants. The experiment can't be restarted; you'd need to create a new one."
      >
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => setConfirmStop(false)}>
            Cancel
          </Button>
          <Button
            variant="danger"
            loading={stopMutation.isPending}
            onClick={() => stopMutation.mutate()}
          >
            <Square aria-hidden className="h-5 w-5" />
            Stop experiment
          </Button>
        </div>
      </Dialog>

      <Dialog
        open={confirmDelete}
        onClose={() => setConfirmDelete(false)}
        title="Delete experiment?"
        description="The experiment will be hidden and stop returning variants to your code. This can't be undone."
      >
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => setConfirmDelete(false)}>
            Cancel
          </Button>
          <Button
            variant="danger"
            loading={deleteMutation.isPending}
            onClick={() => deleteMutation.mutate()}
          >
            Delete
          </Button>
        </div>
      </Dialog>
    </>
  );
}

function Meta({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <div className="text-sm font-medium uppercase tracking-wide text-slate-500">
        {label}
      </div>
      <div
        className={`mt-1 text-base text-slate-900 ${mono ? 'font-mono' : ''}`}
      >
        {value}
      </div>
    </div>
  );
}

function SegmentBlock({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1 text-sm">
      <div className="font-semibold text-slate-500">{title}</div>
      {children}
    </div>
  );
}
