// Presentation Page — OnboardingWizard
// Theme: "The Obsidian Command" — Liquid Glass, Dynamic Theme, Tactical Gauges.
// On finish → navigate /dashboard

import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import {
  Loader2,
  ChevronRight,
  KeyRound,
  Building2,
  Check,
  Copy,
  ShieldAlert,
  Terminal,
  User,
  Users,
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

type Step = 1 | 2 | 3 | 4;
type AccountType = 'individual' | 'team';

/**
 * Segmented Tactical Gauge (RPM Style)
 */
function TacticalGauge({ current, total }: { current: number; total: number }) {
  return (
    <div className="flex gap-1 h-1 w-full max-w-[240px]">
      {Array.from({ length: total }).map((_, i) => (
        <div
          key={i}
          className={`h-full flex-1 rounded-full transition-all duration-700 ${
            i < current 
              ? 'bg-[var(--accent-cyan)] shadow-[0_0_8px_var(--accent-cyan)]' 
              : 'bg-[var(--surface-bright)] opacity-20'
          }`}
        />
      ))}
    </div>
  );
}

const BTN_3D = "w-full inline-flex items-center justify-center gap-2 bg-gradient-to-b from-[var(--surface-bright)] to-[var(--surface-container)] text-[var(--on-surface)] font-geist font-medium border border-[rgba(255,255,255,0.1)] dark:border-[rgba(255,255,255,0.05)] shadow-[inset_0_1px_1px_rgba(255,255,255,0.15)] hover:shadow-[0_0_20px_rgba(34,211,238,0.4)] hover:border-[var(--accent-cyan)] active:translate-y-[2px] active:shadow-none transition-all duration-200 rounded-lg px-6 py-3 disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer";

const BENTO_GLASS = "backdrop-blur-[40px] saturate-[200%] bg-[var(--surface-container)] border border-white/5 rounded-[2rem] p-8 shadow-2xl relative overflow-hidden";

export function OnboardingWizard() {
  const navigate = useNavigate();
  const { completeOnboarding } = useAuth();

  const [step, setStep] = useState<Step>(1);
  const [workspaceName, setWorkspaceName] = useState('');
  const [accountType, setAccountType] = useState<AccountType>('individual');
  const [provider, setProvider] = useState('openai');
  const [apiKey, setApiKey] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [generatedRedEyeKey, setGeneratedRedEyeKey] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  function handleStep1(e: FormEvent) {
    e.preventDefault();
    if (!workspaceName.trim()) return;
    setStep(2);
  }

  function handleStep2(selection: AccountType) {
    setAccountType(selection);
    setStep(3);
  }

  async function handleFinish(e: FormEvent) {
    e.preventDefault();
    if (!apiKey.trim()) return;
    setError(null);
    setLoading(true);
    try {
      const user = await completeOnboarding(workspaceName.trim(), provider, apiKey.trim(), accountType);
      if (user.redeyeApiKey) {
        setGeneratedRedEyeKey(user.redeyeApiKey);
        setStep(4);
      } else {
        navigate('/dashboard');
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Something went wrong.');
    } finally {
      setLoading(false);
    }
  }

  const providerLabels: Record<string, string> = {
    openai: 'OpenAI Protocol',
    gemini: 'Google Gemini Core',
    groq: 'Groq LPU Array',
    anthropic: 'Anthropic Claude',
  };

  return (
    <div className="relative min-h-screen bg-[var(--bg-canvas)] flex items-center justify-center px-4 overflow-hidden font-geist">

      {/* ── Ambient Mesh Background ────────────────────────────── */}
      <div className="fixed inset-0 overflow-hidden pointer-events-none z-0">
        <div className="absolute top-[-10%] left-[-10%] w-[50%] h-[50%] bg-cyan-500/10 blur-[140px] rounded-full" />
        <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-amber-500/5 blur-[120px] rounded-full" />
      </div>

      <div className="relative w-full max-w-lg z-10">

        {/* ── HUD Header ────────────────────────────────────────── */}
        <div className="flex items-center justify-between mb-10 px-2">
          {/* Logo */}
          <div className="flex items-center gap-3">
            <div className="h-9 w-9 rounded-xl bg-cyan-500 flex items-center justify-center shadow-[0_0_20px_rgba(34,211,238,0.45)]">
              <span className="text-[11px] font-black text-[#050505]">RE</span>
            </div>
            <div>
              <p className="text-[9px] uppercase tracking-[0.25em] font-black text-[var(--on-surface-muted)]">RedEye</p>
              <p className="text-xs font-bold tracking-tight leading-none text-[var(--on-surface)]">Deployment Wizard</p>
            </div>
          </div>

          {/* Tactical Progress */}
          <div className="flex flex-col items-end gap-2">
            <span className="text-[10px] font-bold text-[var(--accent-cyan)] uppercase tracking-widest">
              Stage 0{step} / 04
            </span>
            <TacticalGauge current={step} total={4} />
          </div>
        </div>

        {/* ── Wizard Bento Container ────────────────────────────── */}
        <AnimatePresence mode="wait">
          <motion.div
            key={step}
            initial={{ opacity: 0, scale: 0.98, y: 10 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 1.02, y: -10 }}
            transition={{ duration: 0.4, ease: [0.23, 1, 0.32, 1] }}
            className={BENTO_GLASS}
          >
            {/* Background scanner line animation */}
            <div className="absolute top-0 left-0 w-full h-[1px] bg-gradient-to-r from-transparent via-[var(--accent-cyan)]/20 to-transparent animate-scan z-0" />

            {/* ══════════════════════════════════════════════════════════
                STEP 1 — Workspace Name
            ══════════════════════════════════════════════════════════ */}
            {step === 1 && (
              <div className="relative z-10">
                <div className="flex items-center justify-center w-12 h-12 rounded-2xl bg-cyan-500/10 border border-cyan-500/20 mb-6">
                  <Building2 className="w-6 h-6 text-[var(--accent-cyan)]" />
                </div>
                <h1 className="text-2xl font-bold text-[var(--on-surface)] mb-2 tracking-tight">Identity Designation</h1>
                <p className="text-sm text-[var(--on-surface-muted)] mb-8 leading-relaxed font-medium">
                  Establish a unique identifier for your operational workspace. This will be broadcasted in telemetry logs.
                </p>
                <form onSubmit={handleStep1} className="space-y-6">
                  <div className="relative group">
                    <div className="absolute top-[-10px] left-0 text-[10px] font-bold uppercase tracking-widest text-[var(--on-surface-muted)] group-focus-within:text-[var(--accent-cyan)] transition-colors">
                      Workspace Label
                    </div>
                    <input
                      type="text"
                      required
                      autoFocus
                      value={workspaceName}
                      onChange={(e) => setWorkspaceName(e.target.value)}
                      placeholder="e.g. OMEGA_COMMAND_CENTER"
                      className="w-full bg-transparent border-0 border-b border-[var(--surface-bright)] focus:border-[var(--accent-cyan)] focus:ring-0 rounded-none px-0 py-3 text-[var(--on-surface)] placeholder:text-[var(--on-surface-muted)]/30 transition-all font-jetbrains uppercase tracking-wider"
                    />
                  </div>
                  <button type="submit" className={BTN_3D}>
                    Proceed to Context <ChevronRight className="w-4 h-4" />
                  </button>
                </form>
              </div>
            )}

            {/* ══════════════════════════════════════════════════════════
                STEP 2 — Account Type Selection
            ══════════════════════════════════════════════════════════ */}
            {step === 2 && (
              <div className="relative z-10">
                <div className="flex items-center justify-center w-12 h-12 rounded-2xl bg-cyan-500/10 border border-cyan-500/20 mb-6">
                  <Users className="w-6 h-6 text-[var(--accent-cyan)]" />
                </div>
                <h1 className="text-2xl font-bold text-[var(--on-surface)] mb-2 tracking-tight">Operational Mode</h1>
                <p className="text-sm text-[var(--on-surface-muted)] mb-8 leading-relaxed font-medium">
                  Define the scale of your intelligence deployment.
                </p>

                <div className="space-y-4">
                  {/* Individual Card */}
                  <button
                    onClick={() => handleStep2('individual')}
                    className="w-full group relative flex items-start gap-5 p-6 rounded-2xl border border-white/5 bg-[var(--surface-bright)]/20 hover:border-[var(--accent-cyan)] hover:bg-[var(--accent-cyan)]/5 hover:shadow-[0_0_30px_rgba(34,211,238,0.1)] transition-all duration-300 text-left"
                  >
                    <div className="flex-none flex items-center justify-center w-12 h-12 rounded-xl bg-[var(--surface-bright)] border border-white/5 group-hover:bg-[var(--accent-cyan)] group-hover:text-[#050505] transition-all duration-300">
                      <User className="w-6 h-6" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="text-base font-bold text-[var(--on-surface)] mb-1 uppercase tracking-tight">Tactical / Solo</h3>
                      <p className="text-xs text-[var(--on-surface-muted)] font-medium">Single operator, specialized deployment, or laboratory testing.</p>
                    </div>
                    <ChevronRight className="w-5 h-5 text-[var(--on-surface-muted)] group-hover:text-[var(--accent-cyan)] group-hover:translate-x-1 transition-all duration-300 self-center" />
                  </button>

                  {/* Team Card */}
                  <button
                    onClick={() => handleStep2('team')}
                    className="w-full group relative flex items-start gap-5 p-6 rounded-2xl border border-white/5 bg-[var(--surface-bright)]/20 hover:border-teal-400 hover:bg-teal-400/5 hover:shadow-[0_0_30px_rgba(45,212,191,0.1)] transition-all duration-300 text-left"
                  >
                    <div className="flex-none flex items-center justify-center w-12 h-12 rounded-xl bg-[var(--surface-bright)] border border-white/5 group-hover:bg-teal-400 group-hover:text-[#050505] transition-all duration-300">
                      <Users className="w-6 h-6" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="text-base font-bold text-[var(--on-surface)] mb-1 uppercase tracking-tight">Strategic / Org</h3>
                      <p className="text-xs text-[var(--on-surface-muted)] font-medium">Multi-operator coordination, enterprise-grade auditing, and shared protocols.</p>
                    </div>
                    <ChevronRight className="w-5 h-5 text-[var(--on-surface-muted)] group-hover:text-teal-400 group-hover:translate-x-1 transition-all duration-300 self-center" />
                  </button>
                </div>

                <button
                  type="button"
                  onClick={() => setStep(1)}
                  className="mt-8 w-full rounded-xl border border-white/5 bg-[var(--surface-bright)]/30 px-4 py-3 text-[10px] font-bold uppercase tracking-[0.2em] text-[var(--on-surface-muted)] hover:text-[var(--on-surface)] hover:bg-[var(--surface-bright)] transition-all duration-200"
                >
                  Return to Base
                </button>
              </div>
            )}

            {/* ══════════════════════════════════════════════════════════
                STEP 3 — LLM API Key
            ══════════════════════════════════════════════════════════ */}
            {step === 3 && (
              <div className="relative z-10">
                <div className="flex items-center justify-center w-12 h-12 rounded-2xl bg-cyan-500/10 border border-cyan-500/20 mb-6">
                  <KeyRound className="w-6 h-6 text-[var(--accent-cyan)]" />
                </div>
                <h1 className="text-2xl font-bold text-[var(--on-surface)] mb-2 tracking-tight">Neural Uplink</h1>
                <p className="text-sm text-[var(--on-surface-muted)] mb-8 leading-relaxed font-medium">
                  Connect your primary LLM provider. Data remains encrypted via AES-256 standard.
                </p>

                <form onSubmit={handleFinish} className="space-y-6">
                  {/* Provider Select */}
                  <div className="relative group">
                    <div className="absolute top-[-10px] left-0 text-[10px] font-bold uppercase tracking-widest text-[var(--on-surface-muted)] group-focus-within:text-[var(--accent-cyan)] transition-colors">
                      Protocol Provider
                    </div>
                    <select
                      value={provider}
                      onChange={(e) => setProvider(e.target.value)}
                      className="w-full bg-transparent border-0 border-b border-[var(--surface-bright)] focus:border-[var(--accent-cyan)] focus:ring-0 rounded-none px-0 py-3 text-[var(--on-surface)] font-bold appearance-none cursor-pointer"
                    >
                      {Object.entries(providerLabels).map(([value, label]) => (
                        <option key={value} value={value} className="bg-[var(--surface-lowest)] text-[var(--on-surface)]">
                          {label}
                        </option>
                      ))}
                    </select>
                  </div>

                  {/* API Key Input */}
                  <div className="relative group">
                    <div className="absolute top-[-10px] left-0 text-[10px] font-bold uppercase tracking-widest text-[var(--on-surface-muted)] group-focus-within:text-[var(--accent-cyan)] transition-colors">
                      Secret Key Token
                    </div>
                    <input
                      type="password"
                      required
                      autoFocus
                      value={apiKey}
                      onChange={(e) => setApiKey(e.target.value)}
                      placeholder="sk-••••••••••••••••"
                      className="w-full bg-transparent border-0 border-b border-[var(--surface-bright)] focus:border-[var(--accent-cyan)] focus:ring-0 rounded-none px-0 py-3 text-[var(--on-surface)] font-jetbrains tracking-[0.2em]"
                    />
                  </div>

                  {/* ── Inline Error Alert ─────────────────────────────── */}
                  {error && (
                    <motion.div 
                      initial={{ opacity: 0, scale: 0.95 }}
                      animate={{ opacity: 1, scale: 1 }}
                      className="flex items-start gap-3 rounded-xl bg-rose-500/10 border border-rose-500/20 px-4 py-3"
                    >
                      <ShieldAlert className="w-4 h-4 text-rose-400 flex-shrink-0 mt-0.5" />
                      <p className="text-xs text-rose-300 font-medium leading-relaxed uppercase tracking-wide">{error}</p>
                    </motion.div>
                  )}

                  <div className="flex gap-3 pt-2">
                    <button
                      type="button"
                      onClick={() => setStep(2)}
                      className="flex-none rounded-xl border border-white/5 bg-[var(--surface-bright)]/30 px-6 py-3 text-[10px] font-bold uppercase tracking-[0.2em] text-[var(--on-surface-muted)] hover:text-[var(--on-surface)] transition-all duration-200"
                    >
                      Back
                    </button>
                    <button
                      type="submit"
                      disabled={loading}
                      className={BTN_3D}
                    >
                      {loading ? (
                        <>
                          <Loader2 className="w-4 h-4 animate-spin text-[var(--accent-cyan)]" />
                          Initiating Uplink...
                        </>
                      ) : (
                        <>
                          Establish Link <ChevronRight className="w-4 h-4" />
                        </>
                      )}
                    </button>
                  </div>
                </form>
              </div>
            )}

            {/* ══════════════════════════════════════════════════════════
                STEP 4 — RedEye Key Reveal
            ══════════════════════════════════════════════════════════ */}
            {step === 4 && (
              <div className="relative z-10">
                {/* Header */}
                <div className="flex items-center justify-center w-12 h-12 rounded-2xl bg-teal-500/10 border border-teal-500/20 mb-6">
                  <Terminal className="w-6 h-6 text-teal-400" />
                </div>
                <h1 className="text-2xl font-bold text-[var(--on-surface)] mb-2 tracking-tight">Gateway Credentials</h1>
                <p className="text-sm text-[var(--on-surface-muted)] mb-8 leading-relaxed font-medium">
                  Capture your unique gateway key. For neural security,{' '}
                  <strong className="text-[var(--accent-cyan)] font-bold underline decoration-[var(--accent-cyan)]/30 underline-offset-4">this token is single-reveal only</strong>.
                </p>

                {/* ── Tactical Key Display ─────────────────────────── */}
                <div className="relative rounded-2xl bg-[var(--surface-bright)]/40 border border-white/5 shadow-2xl overflow-hidden mb-8">
                  {/* Terminal chrome bar */}
                  <div className="flex items-center justify-between px-5 py-3 border-b border-white/5 bg-[var(--surface-lowest)]/50">
                    <div className="flex items-center gap-1.5">
                      <div className="w-2 h-2 rounded-full bg-rose-500/70" />
                      <div className="w-2 h-2 rounded-full bg-amber-500/70" />
                      <div className="w-2 h-2 rounded-full bg-emerald-500/70" />
                      <span className="ml-3 text-[9px] text-[var(--on-surface-muted)] font-bold uppercase tracking-[0.3em]">
                        SECRET_ACCESS_KEY
                      </span>
                    </div>
                  </div>
                  {/* Key content */}
                  <div className="flex items-center justify-between gap-6 p-6">
                    <code className="text-[var(--accent-cyan)] font-jetbrains text-sm break-all leading-relaxed flex-1 font-bold">
                      {generatedRedEyeKey}
                    </code>
                    <button
                      type="button"
                      onClick={() => {
                        if (generatedRedEyeKey) {
                          navigator.clipboard.writeText(generatedRedEyeKey);
                          setCopied(true);
                          setTimeout(() => setCopied(false), 2000);
                        }
                      }}
                      className="flex-none p-3 rounded-xl border border-[var(--accent-cyan)]/20 bg-[var(--accent-cyan)]/10 hover:bg-[var(--accent-cyan)]/20 transition-all duration-200 text-[var(--accent-cyan)]"
                      title="Capture to memory"
                    >
                      {copied
                        ? <Check className="w-5 h-5 text-emerald-400" />
                        : <Copy className="w-5 h-5" />}
                    </button>
                  </div>
                </div>

                {/* ── Proceed CTA ───────────────────────────────────────── */}
                <button
                  onClick={() => navigate('/dashboard')}
                  className={BTN_3D}
                >
                  Enter Command Center <ChevronRight className="w-4 h-4" />
                </button>
              </div>
            )}
          </motion.div>
        </AnimatePresence>

        {/* HUD footer info */}
        <div className="mt-8 flex items-center justify-center gap-6">
           <div className="flex items-center gap-1.5">
             <div className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse" />
             <span className="text-[9px] font-bold text-[var(--on-surface-muted)] uppercase tracking-widest">Secure Link Active</span>
           </div>
           <div className="h-3 w-[1px] bg-white/5" />
           <span className="text-[9px] font-bold text-[var(--on-surface-muted)] uppercase tracking-widest">Deployment Hash: 0X_RDY_0024</span>
        </div>
      </div>
    </div>
  );
}
