// App.tsx — Root Router
// <AuthProvider> + <BrowserRouter> wrapping all routes.
// Protected routes redirect to /login when not authenticated.

import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { AuthProvider, useAuth } from './presentation/context/AuthContext';
import { IncidentProvider } from './presentation/context/IncidentContext';
import { authService } from './data/services/authService';

// Pages
import { LandingPage }      from './presentation/pages/LandingPage';
import { AuthPage }         from './presentation/pages/AuthPage';
import { OAuthCallback }    from './presentation/pages/OAuthCallback';
import { OnboardingWizard } from './presentation/pages/OnboardingWizard';

// Layout
import { DashboardLayout }  from './presentation/layouts/DashboardLayout';

// Dashboard sub-views (mounted as children of DashboardLayout)
import { DashboardView }    from './presentation/dashboard/DashboardView';
import { ApiKeysView }      from './presentation/dashboard/ApiKeysView';
import { BillingView }      from './presentation/dashboard/BillingView';
import { ComplianceView }   from './presentation/dashboard/ComplianceView';
import { SecurityView }     from './presentation/dashboard/SecurityView';
import { TracesView }       from './presentation/dashboard/TracesView';
import { CacheView }        from './presentation/dashboard/CacheView';
import { SettingsView }     from './presentation/dashboard/SettingsView';
import { ProfileView }      from './presentation/dashboard/ProfileView';


// -----------------------------------------------------------------------
// Metric types & live-fetch logic (formerly in App.tsx monolith)
// -----------------------------------------------------------------------
function DashboardIndex() {
  return <DashboardView />;
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
    <AuthProvider authUseCase={authService}>
      <IncidentProvider>
        <BrowserRouter>
        <Routes>
          {/* Public */}
          <Route path="/" element={<LandingPage />} />
          <Route path="/oauth/callback" element={<OAuthCallback />} />

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
            <Route path="billing"    element={<BillingView />} />
            <Route path="compliance" element={<ComplianceView />} />
            <Route path="security"   element={<SecurityView />} />
            <Route path="traces"     element={<TracesView />} />
            <Route path="cache"      element={<CacheView />} />
            <Route path="settings"   element={<SettingsView />} />
            <Route path="profile"    element={<ProfileView />} />
          </Route>


          {/* Catch-all */}
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </BrowserRouter>
      </IncidentProvider>
    </AuthProvider>
  );
}