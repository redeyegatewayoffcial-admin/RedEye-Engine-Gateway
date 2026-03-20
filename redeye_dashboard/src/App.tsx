// App.tsx — Root Router
// <AuthProvider> + <BrowserRouter> wrapping all routes.
// Protected routes redirect to /login when not authenticated.

import { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { AuthProvider, useAuth } from './presentation/context/AuthContext';

// Pages
import { LandingPage }      from './presentation/pages/LandingPage';
import { AuthPage }         from './presentation/pages/AuthPage';
import { OnboardingWizard } from './presentation/pages/OnboardingWizard';

// Layout
import { DashboardLayout }  from './presentation/layouts/DashboardLayout';

// Dashboard sub-views (mounted as children of DashboardLayout)
import { DashboardView }    from './presentation/dashboard/DashboardView';
import { ApiKeysView }      from './presentation/dashboard/ApiKeysView';
import { ComplianceView }   from './presentation/dashboard/ComplianceView';
import { TracesView }       from './presentation/dashboard/TracesView';
import { CacheView }        from './presentation/dashboard/CacheView';
import { SettingsView }     from './presentation/dashboard/SettingsView';

// -----------------------------------------------------------------------
// Metric types & live-fetch logic (formerly in App.tsx monolith)
// -----------------------------------------------------------------------
interface Metrics {
  total_requests: string;
  avg_latency_ms: number;
  total_tokens: string;
  rate_limited_requests: string;
}

function DashboardIndex() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [chartData, setChartData] = useState<{ time: string; requests: number; latency: number }[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;

    const fetchMetrics = async () => {
      try {
        const res = await fetch('http://localhost:8080/v1/admin/metrics');
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data: Metrics = await res.json();
        if (!alive) return;
        setMetrics(data);
        setError(null);
        const now = new Date().toLocaleTimeString('en-US', {
          hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit',
        });
        setChartData((prev) =>
          [...prev, {
            time: now,
            requests: parseInt(data.total_requests) || Math.floor(Math.random() * 10),
            latency: Math.round(data.avg_latency_ms),
          }].slice(-10),
        );
      } catch (err: unknown) {
        if (alive) setError(err instanceof Error ? err.message : 'Unknown error');
      }
    };

    fetchMetrics();
    const id = setInterval(fetchMetrics, 3000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  const calculateSavedCost = () => {
    if (!metrics) return '0.00';
    return (parseInt(metrics.rate_limited_requests) * 0.005).toFixed(2);
  };

  return (
    <DashboardView
      metrics={metrics}
      chartData={chartData}
      error={error}
      calculateSavedCost={calculateSavedCost}
    />
  );
}

// -----------------------------------------------------------------------
// Route Guards
// -----------------------------------------------------------------------
function RequireAuth({ children }: { children: React.ReactNode }) {
  const { isAuthenticated } = useAuth();
  const location = useLocation();
  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }
  return <>{children}</>;
}

function RedirectIfAuth({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, user } = useAuth();
  if (isAuthenticated) {
    return <Navigate to={user?.onboardingComplete ? '/dashboard' : '/onboarding'} replace />;
  }
  return <>{children}</>;
}

// -----------------------------------------------------------------------
// App
// -----------------------------------------------------------------------
export default function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <Routes>
          {/* Public */}
          <Route path="/" element={<LandingPage />} />

          <Route
            path="/login"
            element={
              <RedirectIfAuth>
                <AuthPage />
              </RedirectIfAuth>
            }
          />

          {/* Protected — Onboarding */}
          <Route
            path="/onboarding"
            element={
              <RequireAuth>
                <OnboardingWizard />
              </RequireAuth>
            }
          />

          {/* Protected — Dashboard */}
          <Route
            path="/dashboard"
            element={
              <RequireAuth>
                <DashboardLayout />
              </RequireAuth>
            }
          >
            <Route index element={<DashboardIndex />} />
            <Route path="api-keys"   element={<ApiKeysView />} />
            <Route path="compliance" element={<ComplianceView />} />
            <Route path="traces"     element={<TracesView />} />
            <Route path="cache"      element={<CacheView />} />
            <Route path="settings"   element={<SettingsView />} />
          </Route>

          {/* Catch-all */}
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  );
}