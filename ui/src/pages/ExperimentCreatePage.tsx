import { useNavigate, Link } from 'react-router-dom';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft } from 'lucide-react';
import { toast } from 'sonner';
import * as ExperimentsAPI from '@/api/experiments';
import { ApiError } from '@/api/client';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { ExperimentForm } from '@/forms/ExperimentForm';

export function ExperimentCreatePage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ExperimentsAPI.createExperiment,
    onSuccess: (res) => {
      toast.success('Experiment created');
      queryClient.invalidateQueries({ queryKey: ['experiments'] });
      navigate(`/experiments/${res.experimentId}`);
    },
    onError: (err) => {
      const msg = err instanceof ApiError ? err.message : 'Failed to create';
      toast.error(msg);
    },
  });

  return (
    <>
      <PageHeader
        title="New experiment"
        description="Define variants and targeting segments."
        actions={
          <Link
            to="/experiments"
            className="inline-flex items-center gap-1.5 text-base font-medium text-slate-600 hover:text-slate-900"
          >
            <ArrowLeft aria-hidden className="h-5 w-5" />
            Back
          </Link>
        }
      />
      <PageBody>
        <ExperimentForm
          mode="create"
          submitLabel="Create experiment"
          submitting={mutation.isPending}
          onSubmit={(payload) => mutation.mutate(payload)}
        />
      </PageBody>
    </>
  );
}
