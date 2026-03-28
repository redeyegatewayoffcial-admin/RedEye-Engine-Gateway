import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { Loader2, Mail, Github, Chrome, ArrowLeft } from 'lucide-react';

type AuthState = 'request' | 'verify';

export function AuthPage() {
  const navigate = useNavigate();
  const { requestMagicLink, verifyMagicLink, ssoRedirect } = useAuth();

  const [authState, setAuthState] = useState<AuthState>('request');
  const [email, setEmail] = useState('');
  const [otp, setOtp] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleOAuth(provider: 'google' | 'github') {
    setError(null);
    setLoading(true);
    try {
      await ssoRedirect(provider);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Something went wrong.');
      setLoading(false);
    }
  }

  async function handleRequestOtp(e: FormEvent) {
    e.preventDefault();
    setError(null);
    if (!email) return;

    setLoading(true);
    try {
      await requestMagicLink(email);
      setAuthState('verify');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Could not send code.');
    } finally {
      setLoading(false);
    }
  }

  async function handleVerifyOtp(e: FormEvent) {
    e.preventDefault();
    setError(null);
    if (otp.length !== 6) {
      setError('Please enter a valid 6-digit code.');
      return;
    }

    setLoading(true);
    try {
      const user = await verifyMagicLink(email, otp);
      if (user.onboardingComplete === true) {
        navigate('/dashboard');
      } else {
        navigate('/onboarding');
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Invalid or expired magic code.');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="relative min-h-screen bg-slate-950 flex items-center justify-center px-4 overflow-hidden">
      
      {/* Background Neon Glows */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-cyan-500/20 rounded-full blur-[120px] pointer-events-none" />
      <div className="absolute top-1/2 left-1/3 -translate-x-1/2 -translate-y-1/2 w-[400px] h-[400px] bg-teal-500/10 rounded-full blur-[120px] pointer-events-none" />

      <div className="relative z-10 w-full max-w-sm">
        {/* Brand Header */}
        <div className="flex flex-col items-center justify-center mb-10">
          <div className="h-12 w-12 rounded-2xl bg-gradient-to-br from-cyan-400 to-teal-400 flex items-center justify-center shadow-[0_0_30px_rgba(34,211,238,0.4)] mb-5">
            <span className="text-lg font-bold tracking-tight text-slate-950">RE</span>
          </div>
          <h1 className="text-xl font-semibold text-slate-100 mb-1">Welcome to RedEye</h1>
          <p className="text-sm text-slate-400">Sign in to your account to continue</p>
        </div>

        {/* Card */}
        <div className="glass-panel p-8">
          
          {error && (
            <p className="mb-6 text-xs text-rose-400 bg-rose-500/10 border border-rose-500/20 rounded-lg px-3 py-2 flex items-center">
              {error}
            </p>
          )}

          {authState === 'request' ? (
            <div className="space-y-6">
              {/* OAuth Section */}
              <div className="space-y-3">
                <button
                  type="button"
                  onClick={() => handleOAuth('github')}
                  disabled={loading}
                  className="w-full relative flex items-center justify-center gap-3 rounded-xl bg-slate-900 border border-slate-700 px-4 py-2.5 text-sm font-medium text-slate-200 hover:border-cyan-400/50 hover:bg-slate-800 hover:shadow-[0_0_15px_rgba(34,211,238,0.15)] transition-all duration-300 disabled:opacity-60 disabled:cursor-not-allowed group"
                >
                  <Github className="w-4 h-4 text-slate-400 group-hover:text-cyan-400 transition-colors" />
                  Continue with GitHub
                </button>
                <button
                  type="button"
                  onClick={() => handleOAuth('google')}
                  disabled={loading}
                  className="w-full relative flex items-center justify-center gap-3 rounded-xl bg-slate-900 border border-slate-700 px-4 py-2.5 text-sm font-medium text-slate-200 hover:border-cyan-400/50 hover:bg-slate-800 hover:shadow-[0_0_15px_rgba(34,211,238,0.15)] transition-all duration-300 disabled:opacity-60 disabled:cursor-not-allowed group"
                >
                  <Chrome className="w-4 h-4 text-slate-400 group-hover:text-cyan-400 transition-colors" />
                  Continue with Google
                </button>
              </div>

              {/* Divider */}
              <div className="relative flex items-center">
                <div className="flex-grow border-t border-slate-800"></div>
                <span className="flex-shrink-0 mx-4 text-xs text-slate-500 font-medium uppercase tracking-wider">
                  Or continue with Email
                </span>
                <div className="flex-grow border-t border-slate-800"></div>
              </div>

              {/* Magic Link Form */}
              <form onSubmit={handleRequestOtp} className="space-y-4">
                <div className="relative">
                  <div className="absolute inset-y-0 left-0 pl-3.5 flex items-center pointer-events-none">
                    <Mail className="h-4 w-4 text-slate-500" />
                  </div>
                  <input
                    type="email"
                    required
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    placeholder="you@company.com"
                    className="premium-input !pl-11 w-full"
                  />
                </div>

                <button
                  type="submit"
                  disabled={loading || !email}
                  className="w-full inline-flex items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 disabled:from-slate-700 disabled:to-slate-800 disabled:text-slate-500 px-4 py-3 text-sm font-semibold text-slate-950 shadow-[0_0_20px_rgba(34,211,238,0.25)] transition-all duration-200"
                >
                  {loading && <Loader2 className="w-4 h-4 animate-spin" />}
                  Send Magic Code
                </button>
              </form>
            </div>
          ) : (
            <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
              <div className="text-center mb-6">
                <div className="inline-flex h-12 w-12 items-center justify-center rounded-full bg-cyan-500/10 mb-4">
                  <Mail className="h-6 w-6 text-cyan-400" />
                </div>
                <h2 className="text-lg font-medium text-slate-200">Check your email</h2>
                <p className="text-sm text-slate-400 mt-1">
                  We sent a 6-digit code to <span className="text-cyan-400">{email}</span>
                </p>
              </div>

              <form onSubmit={handleVerifyOtp} className="space-y-6">
                <div>
                  <input
                    type="text"
                    required
                    maxLength={6}
                    value={otp}
                    onChange={(e) => setOtp(e.target.value.replace(/\D/g, ''))}
                    placeholder="000000"
                    className="premium-input text-center text-2xl tracking-[0.5em] font-mono py-4"
                  />
                </div>

                <button
                  type="submit"
                  disabled={loading || otp.length !== 6}
                  className="w-full inline-flex items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 disabled:from-slate-700 disabled:to-slate-800 disabled:text-slate-500 px-4 py-3 text-sm font-semibold text-slate-950 shadow-[0_0_20px_rgba(34,211,238,0.25)] transition-all duration-200"
                >
                  {loading && <Loader2 className="w-4 h-4 animate-spin" />}
                  Verify & Login
                </button>
              </form>

              <div className="text-center pt-2">
                <button
                  type="button"
                  onClick={() => { setAuthState('request'); setOtp(''); setError(null); }}
                  className="inline-flex items-center gap-2 text-xs font-medium text-slate-400 hover:text-cyan-400 transition-colors"
                >
                  <ArrowLeft className="w-3 h-3" />
                  Use a different email
                </button>
              </div>
            </div>
          )}
        </div>

        <p className="mt-8 text-center text-xs text-slate-600">
          By continuing you agree to the RedEye Terms of Service & Privacy Policy.
        </p>
      </div>
    </div>
  );
}
