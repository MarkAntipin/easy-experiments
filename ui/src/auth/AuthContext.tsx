import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import {
  getStoredToken,
  setStoredToken,
  setUnauthorizedHandler,
} from '@/api/client';
import * as AuthAPI from '@/api/auth';
import type { AuthCompany, AuthUser } from '@/api/types';

interface Session {
  token: string;
  user: AuthUser;
  company: AuthCompany;
}

const SESSION_STORAGE_KEY = 'ee.auth.session';

interface AuthContextValue {
  session: Session | null;
  isAuthenticated: boolean;
  isInitialized: boolean;
  loginWithGoogle: (idToken: string) => Promise<void>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

function loadPersistedSession(): Session | null {
  const token = getStoredToken();
  const raw = localStorage.getItem(SESSION_STORAGE_KEY);
  if (!token || !raw) return null;
  try {
    const parsed = JSON.parse(raw) as Omit<Session, 'token'>;
    if (!parsed?.user || !parsed?.company) return null;
    return { token, user: parsed.user, company: parsed.company };
  } catch {
    return null;
  }
}

function persistSession(session: Session | null): void {
  if (session === null) {
    localStorage.removeItem(SESSION_STORAGE_KEY);
    setStoredToken(null);
    return;
  }
  setStoredToken(session.token);
  localStorage.setItem(
    SESSION_STORAGE_KEY,
    JSON.stringify({ user: session.user, company: session.company }),
  );
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<Session | null>(() => loadPersistedSession());
  const [isInitialized, setInitialized] = useState(false);

  useEffect(() => {
    setInitialized(true);
  }, []);

  const logout = useCallback(() => {
    persistSession(null);
    setSession(null);
  }, []);

  useEffect(() => {
    setUnauthorizedHandler(() => {
      persistSession(null);
      setSession(null);
    });
    return () => setUnauthorizedHandler(null);
  }, []);

  const loginWithGoogle = useCallback(async (idToken: string) => {
    const res = await AuthAPI.googleLogin(idToken);
    const next: Session = { token: res.token, user: res.user, company: res.company };
    persistSession(next);
    setSession(next);
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({
      session,
      isAuthenticated: session !== null,
      isInitialized,
      loginWithGoogle,
      logout,
    }),
    [session, isInitialized, loginWithGoogle, logout],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used inside AuthProvider');
  return ctx;
}
