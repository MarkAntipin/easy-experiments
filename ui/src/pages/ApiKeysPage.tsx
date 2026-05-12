import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Copy, KeyRound, Plus, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import * as ApiKeysAPI from '@/api/apiKeys';
import { ApiError } from '@/api/client';
import type { ApiKeySummary, CreateApiKeyResponse } from '@/api/types';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/Button';
import { Dialog } from '@/components/Dialog';
import { Field, Input } from '@/components/Input';
import { PageLoader } from '@/components/Spinner';
import { ErrorAlert } from '@/components/ErrorAlert';
import { EmptyState } from '@/components/EmptyState';
import { formatRelative, formatTimestamp } from '@/lib/format';

export function ApiKeysPage() {
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState('');
  const [nameError, setNameError] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<CreateApiKeyResponse | null>(null);
  const [confirmRevoke, setConfirmRevoke] = useState<ApiKeySummary | null>(null);

  const queryClient = useQueryClient();

  const listQuery = useQuery({
    queryKey: ['api-keys'],
    queryFn: ApiKeysAPI.listApiKeys,
  });

  const createMutation = useMutation({
    mutationFn: ApiKeysAPI.createApiKey,
    onSuccess: (res) => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      setRevealed(res);
      setCreateOpen(false);
      setName('');
    },
    onError: (err) => {
      toast.error(err instanceof ApiError ? err.message : 'Failed to create key');
    },
  });

  const revokeMutation = useMutation({
    mutationFn: (id: string) => ApiKeysAPI.revokeApiKey(id),
    onSuccess: () => {
      toast.success('API key revoked');
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      setConfirmRevoke(null);
    },
    onError: (err) => {
      toast.error(err instanceof ApiError ? err.message : 'Failed to revoke');
    },
  });

  const submitCreate = () => {
    const trimmed = name.trim();
    if (trimmed.length === 0) {
      setNameError('Name is required');
      return;
    }
    if (trimmed.length > 128) {
      setNameError('Name must be at most 128 characters');
      return;
    }
    setNameError(null);
    createMutation.mutate(trimmed);
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success('Copied to clipboard');
    } catch {
      toast.error('Could not copy');
    }
  };

  return (
    <>
      <PageHeader
        title="API Keys"
        description="Your code uses these keys to ask which variant a user should see."
        actions={
          <Button onClick={() => setCreateOpen(true)}>
            <Plus aria-hidden className="h-5 w-5" />
            New key
          </Button>
        }
      />
      <PageBody>
        {listQuery.isLoading ? (
          <PageLoader />
        ) : listQuery.isError ? (
          <ErrorAlert error={listQuery.error} title="Failed to load keys" />
        ) : listQuery.data && listQuery.data.items.length > 0 ? (
          <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-slate-200 text-base">
                <thead className="bg-slate-50 text-left text-sm uppercase tracking-wide text-slate-500">
                  <tr>
                    <th scope="col" className="px-5 py-3 font-medium">Name</th>
                    <th scope="col" className="px-5 py-3 font-medium">Prefix</th>
                    <th scope="col" className="px-5 py-3 font-medium">Created</th>
                    <th scope="col" className="px-5 py-3 font-medium text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {listQuery.data.items.map((k) => (
                    <tr key={k.apiKeyId}>
                      <td className="px-5 py-3 font-medium text-slate-900">
                        <div className="flex items-center gap-2">
                          <KeyRound aria-hidden className="h-5 w-5 text-slate-400" />
                          {k.name}
                        </div>
                      </td>
                      <td className="px-5 py-3 font-mono text-sm text-slate-700">
                        {k.keyPrefix}…
                      </td>
                      <td className="px-5 py-3 text-slate-500">
                        <span title={formatTimestamp(k.createdAt)}>
                          {formatRelative(k.createdAt)}
                        </span>
                      </td>
                      <td className="px-5 py-3 text-right">
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => setConfirmRevoke(k)}
                        >
                          <Trash2 aria-hidden className="h-4 w-4" />
                          Revoke
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ) : (
          <EmptyState
            title="No API keys yet"
            description="Create a key so your code can ask which variant a user should see."
            action={
              <Button variant="brand" onClick={() => setCreateOpen(true)}>
                <Plus aria-hidden className="h-5 w-5" />
                New key
              </Button>
            }
          />
        )}

      </PageBody>

      <Dialog
        open={createOpen}
        onClose={() => {
          setCreateOpen(false);
          setName('');
          setNameError(null);
        }}
        title="Create API key"
        description="Give this key a descriptive name, for example the service that will use it."
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            submitCreate();
          }}
          className="flex flex-col gap-4"
        >
          <Field label="Name" required error={nameError ?? undefined}>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="production-backend"
              autoFocus
            />
          </Field>
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="secondary"
              onClick={() => setCreateOpen(false)}
            >
              Cancel
            </Button>
            <Button type="submit" loading={createMutation.isPending}>
              Create
            </Button>
          </div>
        </form>
      </Dialog>

      <Dialog
        open={revealed !== null}
        onClose={() => setRevealed(null)}
        title="Copy your new API key"
        description="This is the only time the plaintext key will be shown. Save it somewhere safe."
        className="max-w-2xl"
      >
        {revealed ? (
          <div className="flex flex-col gap-4">
            <div className="rounded-md border border-amber-200 bg-amber-50 px-3.5 py-2.5 text-sm text-amber-800">
              If you lose this key, you'll need to revoke it and create a new one.
            </div>
            <div className="flex items-center gap-2 rounded-md border border-slate-200 bg-slate-50 p-2.5">
              <code className="flex-1 overflow-x-auto whitespace-nowrap font-mono text-sm text-slate-800">
                {revealed.key}
              </code>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => copyToClipboard(revealed.key)}
              >
                <Copy aria-hidden className="h-4 w-4" />
                Copy
              </Button>
            </div>

            <div className="flex justify-end">
              <Button onClick={() => setRevealed(null)}>Done</Button>
            </div>
          </div>
        ) : null}
      </Dialog>

      <Dialog
        open={confirmRevoke !== null}
        onClose={() => setConfirmRevoke(null)}
        title="Revoke API key"
        description={
          confirmRevoke
            ? `"${confirmRevoke.name}" will stop working immediately. This cannot be undone.`
            : ''
        }
      >
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => setConfirmRevoke(null)}>
            Cancel
          </Button>
          <Button
            variant="danger"
            loading={revokeMutation.isPending}
            onClick={() => {
              if (confirmRevoke) revokeMutation.mutate(confirmRevoke.apiKeyId);
            }}
          >
            Revoke
          </Button>
        </div>
      </Dialog>
    </>
  );
}

