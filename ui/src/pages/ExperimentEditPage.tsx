import { Link, useNavigate, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft } from 'lucide-react';
import { toast } from 'sonner';
import * as ExperimentsAPI from '@/api/experiments';
import { ApiError } from '@/api/client';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { PageLoader } from '@/components/Spinner';
import { ErrorAlert } from '@/components/ErrorAlert';
import { ExperimentForm, type ExperimentFormLockMode } from '@/forms/ExperimentForm';
import type { CreateExperimentRequest, UpdateExperimentRequest } from '@/api/types';

function lockModeForStatus(status: string): ExperimentFormLockMode {
  if (status === 'draft') return 'unlocked';
  if (status === 'running') return 'rampUpOnly';
  return 'fullyLocked';
}

function headerCopyForStatus(status: string): string {
  if (status === 'draft') return 'Draft experiment. All fields editable.';
  if (status === 'running') {
    return 'Running. Description and rollout % are editable. Variants, primary metric, and segment shape are locked.';
  }
  return 'Stopped. Only description is editable. Create a new experiment to test something else.';
}

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
          'Someone else updated this experiment. We loaded the latest version. Please review and resubmit.',
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

  const lockMode = lockModeForStatus(exp.status);

  const onSubmit = (full: CreateExperimentRequest) => {
    let payload: UpdateExperimentRequest;
    if (lockMode === 'unlocked') {
      payload = {
        description: full.description ?? null,
        primaryMetric: full.primaryMetric,
        variants: full.variants,
        segments: full.segments,
      };
    } else if (lockMode === 'rampUpOnly') {
      // Description and segments (rollout-up). The backend validates that
      // segments differ only by non-decreasing rolloutPercent and no-ops if
      // values are unchanged.
      payload = {
        description: full.description ?? null,
        segments: full.segments,
      };
    } else {
      payload = { description: full.description ?? null };
    }
    mutation.mutate(payload);
  };

  return (
    <>
      <PageHeader
        title={`Edit: ${exp.key}`}
        description={headerCopyForStatus(exp.status)}
        actions={
          <Link
            to={`/experiments/${id}`}
            className="inline-flex items-center gap-1.5 text-base font-medium text-slate-600 hover:text-slate-900"
          >
            <ArrowLeft aria-hidden className="h-5 w-5" />
            Back
          </Link>
        }
      />
      <PageBody>
        <ExperimentForm
          mode="edit"
          initial={exp}
          lockMode={lockMode}
          submitLabel="Save changes"
          submitting={mutation.isPending}
          onSubmit={onSubmit}
        />
      </PageBody>
    </>
  );
}
