import { Link, useNavigate, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft } from 'lucide-react';
import { toast } from 'sonner';
import * as ExperimentsAPI from '@/api/experiments';
import { ApiError } from '@/api/client';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { PageLoader } from '@/components/Spinner';
import { ErrorAlert } from '@/components/ErrorAlert';
import { ExperimentForm } from '@/forms/ExperimentForm';
import type { CreateExperimentRequest, UpdateExperimentRequest } from '@/api/types';

export function ExperimentEditPage() {
  const { id = '' } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['experiment', id],
    queryFn: () => ExperimentsAPI.getExperiment(id),
    enabled: Boolean(id),
  });

  const mutation = useMutation({
    mutationFn: async (payload: UpdateExperimentRequest) => {
      return ExperimentsAPI.updateExperiment(id, payload, query.data?.updatedAt);
    },
    onSuccess: () => {
      toast.success('Experiment updated');
      queryClient.invalidateQueries({ queryKey: ['experiments'] });
      queryClient.invalidateQueries({ queryKey: ['experiment', id] });
      navigate(`/experiments/${id}`);
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) {
        queryClient.invalidateQueries({ queryKey: ['experiment', id] });
        toast.error(
          'Someone else updated this experiment. We loaded the latest version — please review and resubmit.',
        );
        return;
      }
      toast.error(err instanceof ApiError ? err.message : 'Update failed');
    },
  });

  if (query.isLoading) return <PageLoader />;
  if (query.isError) {
    return (
      <>
        <PageHeader title="Edit experiment" />
        <PageBody>
          <ErrorAlert error={query.error} title="Failed to load experiment" />
        </PageBody>
      </>
    );
  }
  const exp = query.data;
  if (!exp) return null;

  const locked = exp.status !== 'draft';

  const onSubmit = (full: CreateExperimentRequest) => {
    const payload: UpdateExperimentRequest = locked
      ? {
          description: full.description ?? null,
          primaryMetric: full.primaryMetric,
        }
      : {
          description: full.description ?? null,
          primaryMetric: full.primaryMetric,
          variants: full.variants,
          segments: full.segments,
        };
    mutation.mutate(payload);
  };

  return (
    <>
      <PageHeader
        title={`Edit: ${exp.key}`}
        description={
          locked
            ? 'Variants and segments are read-only once the experiment has started. You can still edit description and primary metric.'
            : 'Draft experiment — all fields editable.'
        }
        actions={
          <Link
            to={`/experiments/${id}`}
            className="inline-flex items-center gap-1 text-sm font-medium text-slate-600 hover:text-slate-900"
          >
            <ArrowLeft className="h-4 w-4" />
            Back
          </Link>
        }
      />
      <PageBody>
        <ExperimentForm
          mode="edit"
          initial={exp}
          locked={locked}
          submitLabel="Save changes"
          submitting={mutation.isPending}
          onSubmit={onSubmit}
        />
      </PageBody>
    </>
  );
}
