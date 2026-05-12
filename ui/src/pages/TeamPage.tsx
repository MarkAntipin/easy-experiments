import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Plus, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import * as UsersAPI from '@/api/users';
import { ApiError } from '@/api/client';
import type { UserSummary } from '@/api/types';
import { useAuth } from '@/auth/AuthContext';
import { PageBody, PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/Button';
import { Dialog } from '@/components/Dialog';
import { Field, Input } from '@/components/Input';
import { PageLoader } from '@/components/Spinner';
import { ErrorAlert } from '@/components/ErrorAlert';
import { EmptyState } from '@/components/EmptyState';
import { formatRelative, formatTimestamp } from '@/lib/format';

export function TeamPage() {
  const { session } = useAuth();
  const currentUserId = session?.user.userId ?? '';
  const isAdmin = session?.user.role === 'admin';

  const [inviteOpen, setInviteOpen] = useState(false);
  const [email, setEmail] = useState('');
  const [emailError, setEmailError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<UserSummary | null>(null);

  const queryClient = useQueryClient();

  const listQuery = useQuery({
    queryKey: ['users'],
    queryFn: UsersAPI.listUsers,
  });

  const inviteMutation = useMutation({
    mutationFn: UsersAPI.inviteUser,
    onSuccess: (invited) => {
      queryClient.invalidateQueries({ queryKey: ['users'] });
      setInviteOpen(false);
      setEmail('');
      toast.success(`Invited ${invited.email}. Ask them to sign in with Google.`);
    },
    onError: (err) => {
      toast.error(err instanceof ApiError ? err.message : 'Failed to invite');
    },
  });

  const removeMutation = useMutation({
    mutationFn: (id: string) => UsersAPI.removeUser(id),
    onSuccess: () => {
      toast.success('Member removed');
      queryClient.invalidateQueries({ queryKey: ['users'] });
      setConfirmRemove(null);
    },
    onError: (err) => {
      toast.error(err instanceof ApiError ? err.message : 'Failed to remove');
    },
  });

  const submitInvite = () => {
    const trimmed = email.trim();
    if (trimmed.length === 0) {
      setEmailError('Email is required');
      return;
    }
    // Cheap structural check. Server is the source of truth, this just keeps
    // obvious typos out of the round-trip.
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(trimmed)) {
      setEmailError('That doesn’t look like a valid email');
      return;
    }
    setEmailError(null);
    inviteMutation.mutate(trimmed);
  };

  return (
    <>
      <PageHeader
        title="Team"
        description={
          isAdmin
            ? 'Invite teammates by email. They’ll join when they sign in with Google.'
            : 'Your teammates. Only admins can invite or remove members.'
        }
        actions={
          isAdmin ? (
            <Button onClick={() => setInviteOpen(true)}>
              <Plus aria-hidden className="h-5 w-5" />
              Invite member
            </Button>
          ) : null
        }
      />
      <PageBody>
        {listQuery.isLoading ? (
          <PageLoader />
        ) : listQuery.isError ? (
          <ErrorAlert error={listQuery.error} title="Failed to load team" />
        ) : listQuery.data && listQuery.data.items.length > 0 ? (
          <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-slate-200 text-base">
                <thead className="bg-slate-50 text-left text-sm uppercase tracking-wide text-slate-500">
                  <tr>
                    <th scope="col" className="px-5 py-3 font-medium">Member</th>
                    <th scope="col" className="px-5 py-3 font-medium">Role</th>
                    <th scope="col" className="px-5 py-3 font-medium">Status</th>
                    <th scope="col" className="px-5 py-3 font-medium">Added</th>
                    <th scope="col" className="px-5 py-3 font-medium text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {listQuery.data.items.map((u) => {
                    const isPending = u.status === 'pending';
                    const isSelf = u.userId === currentUserId;
                    const displayName = u.name ?? u.email;
                    return (
                      <tr key={u.userId} className={isPending ? 'bg-slate-50/40' : ''}>
                        <td className="px-5 py-3">
                          <div className="flex items-center gap-3">
                            {u.pictureUrl ? (
                              <img
                                src={u.pictureUrl}
                                alt=""
                                className="h-9 w-9 rounded-full ring-2 ring-slate-100"
                                referrerPolicy="no-referrer"
                              />
                            ) : (
                              <div
                                aria-hidden
                                className={
                                  isPending
                                    ? 'grid h-9 w-9 place-items-center rounded-full bg-slate-200 text-sm font-semibold text-slate-500'
                                    : 'grid h-9 w-9 place-items-center rounded-full bg-brand-gradient text-sm font-semibold text-white'
                                }
                              >
                                {u.email.slice(0, 1).toUpperCase()}
                              </div>
                            )}
                            <div className="min-w-0 leading-tight">
                              <div className={isPending ? 'truncate font-medium text-slate-500' : 'truncate font-medium text-slate-900'}>
                                {displayName}
                                {isSelf ? (
                                  <span className="ml-2 text-sm font-normal text-slate-400">(you)</span>
                                ) : null}
                              </div>
                              {u.name ? (
                                <div className="truncate text-sm text-slate-500">{u.email}</div>
                              ) : null}
                            </div>
                          </div>
                        </td>
                        <td className="px-5 py-3">
                          {u.role === 'admin' ? (
                            <span className="inline-flex items-center rounded-full bg-indigo-50 px-2.5 py-0.5 text-sm font-medium text-indigo-700 ring-1 ring-inset ring-indigo-200">
                              Admin
                            </span>
                          ) : (
                            <span className="inline-flex items-center rounded-full bg-slate-100 px-2.5 py-0.5 text-sm font-medium text-slate-700 ring-1 ring-inset ring-slate-200">
                              Member
                            </span>
                          )}
                        </td>
                        <td className="px-5 py-3">
                          {isPending ? (
                            <span className="inline-flex items-center rounded-full bg-amber-50 px-2.5 py-0.5 text-sm font-medium text-amber-800 ring-1 ring-inset ring-amber-200">
                              Pending
                            </span>
                          ) : (
                            <span className="inline-flex items-center rounded-full bg-emerald-50 px-2.5 py-0.5 text-sm font-medium text-emerald-700 ring-1 ring-inset ring-emerald-200">
                              Active
                            </span>
                          )}
                        </td>
                        <td className="px-5 py-3 text-slate-500">
                          <span title={formatTimestamp(u.createdAt)}>
                            {formatRelative(u.createdAt)}
                          </span>
                        </td>
                        <td className="px-5 py-3 text-right">
                          {isSelf || !isAdmin ? null : (
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => setConfirmRemove(u)}
                            >
                              <Trash2 aria-hidden className="h-4 w-4" />
                              Remove
                            </Button>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        ) : (
          <EmptyState
            title="No teammates yet"
            description={
              isAdmin
                ? 'Invite someone by email; they’ll show up here once you add them.'
                : 'Ask an admin to invite teammates.'
            }
            action={
              isAdmin ? (
                <Button variant="brand" onClick={() => setInviteOpen(true)}>
                  <Plus aria-hidden className="h-5 w-5" />
                  Invite member
                </Button>
              ) : undefined
            }
          />
        )}
      </PageBody>

      <Dialog
        open={inviteOpen}
        onClose={() => {
          setInviteOpen(false);
          setEmail('');
          setEmailError(null);
        }}
        title="Invite member"
        description="They join your workspace the next time they sign in with Google using this email."
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            submitInvite();
          }}
          className="flex flex-col gap-4"
        >
          <Field label="Email" required error={emailError ?? undefined}>
            <Input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="teammate@company.com"
              autoFocus
            />
          </Field>
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="secondary"
              onClick={() => setInviteOpen(false)}
            >
              Cancel
            </Button>
            <Button type="submit" loading={inviteMutation.isPending}>
              Invite
            </Button>
          </div>
        </form>
      </Dialog>

      <Dialog
        open={confirmRemove !== null}
        onClose={() => setConfirmRemove(null)}
        title="Remove member"
        description={
          confirmRemove
            ? `${confirmRemove.name ?? confirmRemove.email} will lose access immediately.`
            : ''
        }
      >
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => setConfirmRemove(null)}>
            Cancel
          </Button>
          <Button
            variant="danger"
            loading={removeMutation.isPending}
            onClick={() => {
              if (confirmRemove) removeMutation.mutate(confirmRemove.userId);
            }}
          >
            Remove
          </Button>
        </div>
      </Dialog>
    </>
  );
}
