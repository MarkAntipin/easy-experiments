import { useState } from 'react';
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import { toast } from 'sonner';
import { ApiError } from '@/api/client';
import { useAuth } from '@/auth/AuthContext';
import { Button } from '@/components/Button';
import { Field, Input } from '@/components/Input';
import { Logo } from '@/components/Logo';

const MIN_PASSWORD_LEN = 8;

export function AcceptInvitePage() {
  const { isAuthenticated, acceptInvite } = useAuth();
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const token = params.get('token') ?? '';

  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  if (isAuthenticated) {
    return <Navigate to="/experiments" replace />;
  }

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!token) {
      setErr('This invite link is missing its token. Ask your admin to resend it.');
      return;
    }
    if (password.length < MIN_PASSWORD_LEN) {
      setErr(`Password must be at least ${MIN_PASSWORD_LEN} characters.`);
      return;
    }
    if (password !== confirm) {
      setErr('Passwords do not match.');
      return;
    }
    setErr(null);
    setSubmitting(true);
    try {
      await acceptInvite(token, password);
      toast.success('Welcome aboard!');
      navigate('/experiments', { replace: true });
    } catch (e) {
      const msg =
        e instanceof ApiError
          ? e.message
          : 'Could not accept the invite. The link may be expired.';
      setErr(msg);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="relative grid min-h-screen place-items-center overflow-hidden bg-slate-50 px-4">
      <div
        aria-hidden
        className="pointer-events-none absolute -top-40 left-1/2 h-[640px] w-[640px] -translate-x-1/2 rounded-full bg-brand-gradient opacity-[0.08] blur-3xl"
      />
      <div className="relative z-10 w-full max-w-sm rounded-2xl border border-slate-200 bg-white p-8 shadow-brand-glow">
        <div className="mb-6 flex flex-col items-center text-center">
          <Logo size={128} className="h-32 w-32" />
          <p className="mt-1 text-sm uppercase tracking-[0.18em] text-ink-400">
            Set up your account
          </p>
        </div>

        <h2 className="mb-4 text-center text-base font-medium text-ink-700">
          Choose a password to finish joining
        </h2>

        <form onSubmit={submit} className="flex flex-col gap-4">
          <Field label="New password" required hint={`At least ${MIN_PASSWORD_LEN} characters.`}>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="new-password"
              placeholder="••••••••"
              autoFocus
            />
          </Field>
          <Field label="Confirm password" required>
            <Input
              type="password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              autoComplete="new-password"
              placeholder="••••••••"
            />
          </Field>
          <Button type="submit" loading={submitting} disabled={!token}>
            Activate account
          </Button>
        </form>

        {err ? <p className="mt-3 text-center text-sm text-red-600">{err}</p> : null}
      </div>
    </div>
  );
}
