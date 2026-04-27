import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { Loader2, Mail, Github, ArrowLeft } from 'lucide-react';
import { motion } from 'framer-motion';

type AuthState = 'request' | 'verify';

/**
 * Google "G" Logo SVG
 */
const GoogleIcon = () => (
  <svg viewBox="0 0 24 24" className="w-4 h-4" xmlns="http://www.w3.org/2000/svg">
    <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4"/>
    <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-1 .67-2.28 1.07-3.71 1.07-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/>
    <path d="M5.84 14.11c-.22-.66-.35-1.36-.35-2.11s.13-1.45.35-2.11V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l3.66-2.83z" fill="#FBBC05"/>
    <path d="M12 5.38c1.62 0 3.06.56 4.21 1.66l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.83c.87-2.6 3.3-4.51 6.16-4.51z" fill="#EA4335"/>
  </svg>
);

const BTN_3D = "w-full relative flex items-center justify-center gap-3 bg-gradient-to-b from-[var(--surface-bright)] to-[var(--surface-container)] text-[var(--on-surface)] font-geist font-medium border border-[rgba(255,255,255,0.1)] dark:border-[rgba(255,255,255,0.05)] shadow-[inset_0_1px_1px_rgba(255,255,255,0.15)] hover:shadow-[0_0_20px_rgba(34,211,238,0.4)] hover:border-[var(--accent-cyan)] active:translate-y-[2px] active:shadow-none transition-all duration-200 rounded-lg px-6 py-3 disabled:opacity-60 disabled:cursor-not-allowed";

const TACTICAL_INPUT = "w-full bg-transparent border-0 border-b border-[var(--surface-bright)] focus:border-[var(--accent-cyan)] focus:ring-0 rounded-none px-0 py-2.5 text-[var(--on-surface)] placeholder:text-[var(--on-surface-muted)] transition-all font-geist";

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
    <div className="relative min-h-screen bg-[var(--bg-canvas)] flex items-center justify-center px-4 overflow-hidden">
      
      {/* ── Ambient Mesh Background (Obsidian Style) ──────────────── */}
      <div className="fixed inset-0 overflow-hidden pointer-events-none z-0">
        <div className="absolute top-[10%] left-[10%] w-[40%] h-[40%] bg-cyan-500/10 blur-[120px] rounded-full" />
        <div className="absolute bottom-[10%] right-[10%] w-[30%] h-[30%] bg-amber-500/5 blur-[100px] rounded-full" />
      </div>

      <div className="relative z-10 w-full max-w-sm">
        {/* Brand Header */}
        <div className="flex flex-col items-center justify-center mb-10">
          <motion.div 
            initial={{ scale: 0.8, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            className="h-12 w-12 rounded-2xl bg-cyan-500 flex items-center justify-center shadow-[0_0_30px_rgba(34,211,238,0.4)] mb-5"
          >
            <span className="text-lg font-black tracking-tight text-[#050505]">RE</span>
          </motion.div>
          <h1 className="text-xl font-bold text-[var(--on-surface)] mb-1 font-geist">RedEye Command</h1>
          <p className="text-sm text-[var(--on-surface-muted)] font-geist">Strategic Intelligence Gateway</p>
        </div>

        {/* Card: Liquid Glass */}
        <div className="backdrop-blur-[40px] saturate-[200%] bg-[var(--surface-container)] border border-white/5 rounded-2xl p-8 shadow-2xl">
          
          {error && (
            <motion.p 
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              className="mb-6 text-xs text-rose-400 bg-rose-500/10 border border-rose-500/20 rounded-lg px-3 py-2 flex items-center font-geist"
            >
              {error}
            </motion.p>
          )}

          {authState === 'request' ? (
            <div className="space-y-6">
              {/* OAuth Section */}
              <div className="space-y-3">
                <button
                  type="button"
                  onClick={() => handleOAuth('github')}
                  disabled={loading}
                  className="w-full relative flex items-center justify-center gap-3 rounded-xl bg-[var(--surface-bright)] border border-white/5 px-4 py-2.5 text-sm font-medium text-[var(--on-surface)] hover:border-[var(--accent-cyan)] transition-all duration-300 disabled:opacity-60 group shadow-sm"
                >
                  <Github className="w-4 h-4 text-[var(--on-surface-muted)] group-hover:text-[var(--accent-cyan)] transition-colors" />
                  Continue with GitHub
                </button>
                <button
                  type="button"
                  onClick={() => handleOAuth('google')}
                  disabled={loading}
                  className="w-full relative flex items-center justify-center gap-3 rounded-xl bg-[var(--surface-bright)] border border-white/5 px-4 py-2.5 text-sm font-medium text-[var(--on-surface)] hover:border-[var(--accent-cyan)] transition-all duration-300 disabled:opacity-60 group shadow-sm"
                >
                  <GoogleIcon />
                  Continue with Google
                </button>
              </div>

              {/* Divider */}
              <div className="relative flex items-center">
                <div className="flex-grow border-t border-white/5"></div>
                <span className="flex-shrink-0 mx-4 text-[10px] text-[var(--on-surface-muted)] font-bold uppercase tracking-[0.2em] font-geist">
                  Protocol Access
                </span>
                <div className="flex-grow border-t border-white/5"></div>
              </div>

              {/* Magic Link Form */}
              <form onSubmit={handleRequestOtp} className="space-y-6">
                <div className="relative group">
                  <div className="absolute top-[-10px] left-0 text-[10px] font-bold uppercase tracking-widest text-[var(--on-surface-muted)] group-focus-within:text-[var(--accent-cyan)] transition-colors">
                    Deployment Email
                  </div>
                  <input
                    type="email"
                    required
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    placeholder="operator@company.com"
                    className={TACTICAL_INPUT}
                  />
                </div>

                <button
                  type="submit"
                  disabled={loading || !email}
                  className={BTN_3D}
                >
                  {loading && <Loader2 className="w-4 h-4 animate-spin text-[var(--accent-cyan)]" />}
                  Issue Access Token
                </button>
              </form>
            </div>
          ) : (
            <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
              <div className="text-center mb-6">
                <div className="inline-flex h-12 w-12 items-center justify-center rounded-2xl bg-cyan-500/10 border border-cyan-500/20 mb-4">
                  <Mail className="h-6 w-6 text-[var(--accent-cyan)]" />
                </div>
                <h2 className="text-lg font-bold text-[var(--on-surface)] font-geist">Transmission Sent</h2>
                <p className="text-sm text-[var(--on-surface-muted)] mt-1 font-geist">
                  Magic code dispatched to <span className="text-[var(--accent-cyan)] font-medium">{email}</span>
                </p>
              </div>

              <form onSubmit={handleVerifyOtp} className="space-y-6">
                <div className="relative group">
                  <div className="absolute top-[-10px] left-0 text-[10px] font-bold uppercase tracking-widest text-[var(--on-surface-muted)] group-focus-within:text-[var(--accent-cyan)] transition-colors">
                    6-Digit Verification
                  </div>
                  <input
                    type="text"
                    required
                    maxLength={6}
                    value={otp}
                    onChange={(e) => setOtp(e.target.value.replace(/\D/g, ''))}
                    placeholder="000000"
                    className={`${TACTICAL_INPUT} text-center text-2xl tracking-[0.5em] font-jetbrains`}
                  />
                </div>

                <button
                  type="submit"
                  disabled={loading || otp.length !== 6}
                  className={BTN_3D}
                >
                  {loading && <Loader2 className="w-4 h-4 animate-spin text-[var(--accent-cyan)]" />}
                  Verify Protocol
                </button>
              </form>

              <div className="text-center pt-2">
                <button
                  type="button"
                  onClick={() => { setAuthState('request'); setOtp(''); setError(null); }}
                  className="inline-flex items-center gap-2 text-[10px] font-bold uppercase tracking-widest text-[var(--on-surface-muted)] hover:text-[var(--accent-cyan)] transition-all"
                >
                  <ArrowLeft className="w-3 h-3" />
                  New Deployment
                </button>
              </div>
            </div>
          )}
        </div>

        <p className="mt-8 text-center text-[10px] text-[var(--on-surface-muted)] uppercase tracking-[0.2em] font-medium leading-relaxed">
          Secure Neural Gateway — v2.4.0<br/>
          By continuing you agree to the RedEye Protocols.
        </p>
      </div>
    </div>
  );
}
