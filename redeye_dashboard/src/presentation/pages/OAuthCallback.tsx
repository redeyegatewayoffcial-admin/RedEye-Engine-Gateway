import { useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

export function OAuthCallback() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { syncOAuthState } = useAuth();

  useEffect(() => {
    const onboardingComplete = searchParams.get('onboarding_complete') === 'true';

    // Persist token AND hydrate React Context state before navigating
    // syncOAuthState now triggers a /refresh call to establish session from cookies
    syncOAuthState().then(() => {
      if (onboardingComplete) {
        navigate('/dashboard', { replace: true });
      } else {
        navigate('/onboarding', { replace: true });
      }
    }).catch(() => {
      navigate('/login', { replace: true });
    });
  }, [searchParams, navigate, syncOAuthState]);

  return (
    <div className="relative min-h-screen bg-slate-950 flex flex-col items-center justify-center overflow-hidden">
      {/* Background neon glow */}
      <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
        <div className="w-[500px] h-[500px] rounded-full bg-cyan-500/15 blur-[120px]" />
      </div>

      <div className="relative z-10 flex flex-col items-center gap-6">
        {/* Brand mark */}
        <div className="h-14 w-14 rounded-2xl bg-gradient-to-br from-cyan-400 to-teal-400
          flex items-center justify-center
          shadow-[0_0_40px_rgba(34,211,238,0.45)]
          animate-pulse">
          <span className="text-xl font-bold tracking-tight text-slate-950">RE</span>
        </div>

        {/* Spinner ring */}
        <div className="relative h-12 w-12">
          <div className="absolute inset-0 rounded-full border-2 border-slate-800" />
          <div className="absolute inset-0 rounded-full border-2 border-transparent border-t-cyan-400 animate-spin" />
        </div>

        <div className="text-center">
          <p className="text-sm font-semibold text-slate-200 tracking-wide">Authenticating securely…</p>
          <p className="text-xs text-slate-500 mt-1">Establishing your RedEye session</p>
        </div>
      </div>
    </div>
  );
}
