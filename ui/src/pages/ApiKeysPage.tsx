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

const EVALUATE_PATH = '/api/v1/experiments/evaluate';

function evaluateUrl(): string {
  const base = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/$/, '');
  return base ? `${base}${EVALUATE_PATH}` : `https://your-api-host${EVALUATE_PATH}`;
}

function curlSnippet(key: string): string {
  return `curl -X POST '${evaluateUrl()}' \\
  -H 'X-Api-Key: ${key}' \\
  -H 'Content-Type: application/json' \\
  -d '{
    "experimentKey": "homepage_cta",
    "entityId": "user-123",
    "properties": { "country": "US" }
  }'`;
}

function jsSnippet(key: string): string {
  return `const res = await fetch('${evaluateUrl()}', {
  method: 'POST',
  headers: {
    'X-Api-Key': '${key}',
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    experimentKey: 'homepage_cta',
    entityId: 'user-123',
    properties: { country: 'US' },
  }),
});
const { variantKey, config } = await res.json();`;
}

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
            <Plus className="h-4 w-4" />
            New key
          </Button>
        }
      />
      <PageBody>
        {listQuery.isLoading ? (
          <PageLoader />
        ) : listQuery.isError ? (
          <ErrorAlert error={listQuery.error} title="Failed to load keys" />
        ) : listQuery.data && listQuery.data.length > 0 ? (
          <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
            <table className="min-w-full divide-y divide-slate-200 text-sm">
              <thead className="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-2 font-medium">Name</th>
                  <th className="px-4 py-2 font-medium">Prefix</th>
                  <th className="px-4 py-2 font-medium">Created</th>
                  <th className="px-4 py-2 font-medium text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {listQuery.data.map((k) => (
                  <tr key={k.apiKeyId}>
                    <td className="px-4 py-2.5 font-medium text-slate-900">
                      <div className="flex items-center gap-2">
                        <KeyRound className="h-4 w-4 text-slate-400" />
                        {k.name}
                      </div>
                    </td>
                    <td className="px-4 py-2.5 font-mono text-xs text-slate-700">
                      {k.keyPrefix}…
                    </td>
                    <td className="px-4 py-2.5 text-slate-500">
                      <span title={formatTimestamp(k.createdAt)}>
                        {formatRelative(k.createdAt)}
                      </span>
                    </td>
                    <td className="px-4 py-2.5 text-right">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => setConfirmRevoke(k)}
                      >
                        <Trash2 className="h-4 w-4" />
                        Revoke
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyState
            title="No API keys yet"
            description="Create a key so your code can ask which variant a user should see."
            action={
              <Button onClick={() => setCreateOpen(true)}>
                <Plus className="h-4 w-4" />
                New key
              </Button>
            }
          />
        )}

        {listQuery.data && listQuery.data.length > 0 ? (
          <div className="mt-6">
            <UsageSnippets onCopy={copyToClipboard} />
          </div>
        ) : null}
      </PageBody>

      <Dialog
        open={createOpen}
        onClose={() => {
          setCreateOpen(false);
          setName('');
          setNameError(null);
        }}
        title="Create API key"
        description="Give this key a descriptive name — for example the service that will use it."
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
            <div className="rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800">
              If you lose this key, you'll need to revoke it and create a new one.
            </div>
            <div className="flex items-center gap-2 rounded-md border border-slate-200 bg-slate-50 p-2">
              <code className="flex-1 overflow-x-auto whitespace-nowrap font-mono text-xs text-slate-800">
                {revealed.key}
              </code>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => copyToClipboard(revealed.key)}
              >
                <Copy className="h-4 w-4" />
                Copy
              </Button>
            </div>

            <UsageSnippets apiKey={revealed.key} onCopy={copyToClipboard} />

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

function UsageSnippets({
  apiKey,
  onCopy,
}: {
  apiKey?: string;
  onCopy: (text: string) => void;
}) {
  const [tab, setTab] = useState<'curl' | 'js'>('curl');
  const placeholder = apiKey ?? 'YOUR_API_KEY';
  const code = tab === 'curl' ? curlSnippet(placeholder) : jsSnippet(placeholder);

  return (
    <div className="rounded-lg border border-slate-200 bg-white">
      <div className="flex items-center justify-between border-b border-slate-200 px-4 py-2.5">
        <div>
          <h3 className="text-sm font-semibold text-slate-900">
            How to use this key
          </h3>
          <p className="text-xs text-slate-500">
            Send the key in the{' '}
            <code className="rounded bg-slate-100 px-1 font-mono text-[11px]">
              X-Api-Key
            </code>{' '}
            header on every evaluate call.
          </p>
        </div>
        <div className="inline-flex rounded-md bg-slate-100 p-0.5">
          <button
            type="button"
            onClick={() => setTab('curl')}
            className={
              tab === 'curl'
                ? 'rounded px-2.5 py-1 text-xs font-medium bg-white text-slate-900 shadow-sm'
                : 'rounded px-2.5 py-1 text-xs font-medium text-slate-500 hover:text-slate-800'
            }
          >
            curl
          </button>
          <button
            type="button"
            onClick={() => setTab('js')}
            className={
              tab === 'js'
                ? 'rounded px-2.5 py-1 text-xs font-medium bg-white text-slate-900 shadow-sm'
                : 'rounded px-2.5 py-1 text-xs font-medium text-slate-500 hover:text-slate-800'
            }
          >
            JavaScript
          </button>
        </div>
      </div>
      <div className="relative">
        <pre className="max-h-72 overflow-auto bg-slate-900 px-4 py-3 font-mono text-xs leading-relaxed text-slate-100">
          {code}
        </pre>
        <Button
          size="sm"
          variant="secondary"
          className="absolute right-2 top-2"
          onClick={() => onCopy(code)}
        >
          <Copy className="h-3.5 w-3.5" />
          Copy
        </Button>
      </div>
      <div className="border-t border-slate-200 px-4 py-2 text-[11px] text-slate-500">
        Replace{' '}
        <code className="rounded bg-slate-100 px-1 font-mono">homepage_cta</code>{' '}
        with your experiment key, and{' '}
        <code className="rounded bg-slate-100 px-1 font-mono">user-123</code>{' '}
        with a stable per-user identifier so the same user always sees the same
        variant.
      </div>
    </div>
  );
}
