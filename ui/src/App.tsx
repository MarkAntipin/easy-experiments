import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { GoogleOAuthProvider } from '@react-oauth/google';
import { Toaster } from 'sonner';
import { ApiError } from '@/api/client';
import { AuthProvider } from '@/auth/AuthContext';
import { ProtectedRoute } from '@/auth/ProtectedRoute';
import { Layout } from '@/components/Layout';
import { LoginPage } from '@/pages/LoginPage';
import { ExperimentsListPage } from '@/pages/ExperimentsListPage';
import { ExperimentCreatePage } from '@/pages/ExperimentCreatePage';
import { ExperimentDetailPage } from '@/pages/ExperimentDetailPage';
import { ExperimentEditPage } from '@/pages/ExperimentEditPage';
import { ExperimentResultsPage } from '@/pages/ExperimentResultsPage';
import { ApiKeysPage } from '@/pages/ApiKeysPage';
import { GuidePage } from '@/pages/GuidePage';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: (failureCount, error) => {
        // Don't retry 4xx errors.
        if (error instanceof ApiError && error.status >= 400 && error.status < 500) {
          return false;
        }
        return failureCount < 2;
      },
      refetchOnWindowFocus: false,
      staleTime: 10_000,
    },
    mutations: {
      retry: false,
    },
  },
});

const GOOGLE_CLIENT_ID = import.meta.env.VITE_GOOGLE_CLIENT_ID ?? '';

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <GoogleOAuthProvider clientId={GOOGLE_CLIENT_ID}>
        <BrowserRouter>
          <AuthProvider>
            <Routes>
              <Route path="/login" element={<LoginPage />} />
              <Route
                element={
                  <ProtectedRoute>
                    <Layout />
                  </ProtectedRoute>
                }
              >
                <Route
                  index
                  element={<Navigate to="/experiments" replace />}
                />
                <Route path="/experiments" element={<ExperimentsListPage />} />
                <Route
                  path="/experiments/new"
                  element={<ExperimentCreatePage />}
                />
                <Route
                  path="/experiments/:id"
                  element={<ExperimentDetailPage />}
                />
                <Route
                  path="/experiments/:id/edit"
                  element={<ExperimentEditPage />}
                />
                <Route
                  path="/experiments/:id/results"
                  element={<ExperimentResultsPage />}
                />
                <Route path="/api-keys" element={<ApiKeysPage />} />
                <Route path="/guide" element={<GuidePage />} />
              </Route>
              <Route path="*" element={<Navigate to="/experiments" replace />} />
            </Routes>
          </AuthProvider>
        </BrowserRouter>
      </GoogleOAuthProvider>
      <Toaster richColors position="bottom-right" />
      {import.meta.env.DEV ? <ReactQueryDevtools initialIsOpen={false} /> : null}
    </QueryClientProvider>
  );
}
