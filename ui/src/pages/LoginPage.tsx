import { useState } from 'react';
import { GoogleLogin } from '@react-oauth/google';
import { Navigate, useLocation } from 'react-router-dom';
import { toast } from 'sonner';
import { useAuth } from '@/auth/AuthContext';
import { ApiError } from '@/api/client';
import { Button } from '@/components/Button';
import { Field, Input } from '@/components/Input';
import { Logo } from '@/components/Logo';
import { isGoogleAuthEnabled } from '@/lib/runtimeConfig';

export function LoginPage() {
  const { isAuthenticated, loginWithGoogle, loginWithPassword } = useAuth();
  const location = useLocation();
  const [submitting, setSubmitting] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');

  if (isAuthenticated) {
    const from =
      (location.state as { from?: { pathname: string } } | null)?.from?.pathname ??
      '/experiments';
    return <Navigate to={from} replace />;
  }

  const googleEnabled = isGoogleAuthEnabled();

  const submitPassword = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email.trim() || !password) {
      setErr('Email and password are required.');
      return;
    }
    setErr(null);
    setSubmitting(true);
    try {
      await loginWithPassword(email.trim(), password);
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : 'Sign-in failed.';
      setErr(msg);
      toast.error(msg);
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
      <div
        aria-hidden
        className="pointer-events-none absolute bottom-0 right-0 h-[420px] w-[420px] translate-x-1/3 translate-y-1/3 rounded-full bg-accent-500 opacity-[0.06] blur-3xl"
      />

      <div className="relative z-10 w-full max-w-sm rounded-2xl border border-slate-200 bg-white p-8 shadow-brand-glow">
        <div className="mb-6 flex flex-col items-center text-center">
          <Logo size={128} className="h-32 w-32" />
          <p className="mt-1 text-sm uppercase tracking-[0.18em] text-ink-400">
            Admin panel
          </p>
        </div>

        <h2 className="mb-4 text-center text-base font-medium text-ink-700">
          Sign in to continue
        </h2>

        {googleEnabled ? (
          <div className="flex justify-center">
            <GoogleLogin
              onSuccess={async (credential) => {
                if (!credential.credential) {
                  setErr('No credential returned from Google.');
                  return;
                }
                setErr(null);
                setSubmitting(true);
                try {
                  await loginWithGoogle(credential.credential);
                } catch (e) {
                  const msg = e instanceof ApiError ? e.message : 'Sign-in failed.';
                  setErr(msg);
                  toast.error(msg);
                } finally {
                  setSubmitting(false);
                }
              }}
              onError={() => setErr('Google sign-in was cancelled or failed.')}
              useOneTap={false}
            />
          </div>
        ) : (
          <form onSubmit={submitPassword} className="flex flex-col gap-4">
            <Field label="Email" required>
              <Input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                autoComplete="email"
                placeholder="you@company.com"
              />
            </Field>
            <Field label="Password" required>
              <Input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="current-password"
                placeholder="••••••••"
              />
            </Field>
            <Button type="submit" loading={submitting}>
              Sign in
            </Button>
          </form>
        )}

        {err ? <p className="mt-3 text-center text-sm text-red-600">{err}</p> : null}
      </div>
    </div>
  );
}
