// Presentation Page — OnboardingWizard
// Step 1: Workspace name | Step 2: LLM API key | Step 3: RedEye key reveal
// Theme: "Cool Revival" — Midnight Obsidian + Neon Cyan/Teal
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
} from 'lucide-react';

type Step = 1 | 2 | 3;

export function OnboardingWizard() {
  const navigate = useNavigate();
  const { completeOnboarding } = useAuth();

  const [step, setStep] = useState<Step>(1);
  const [workspaceName, setWorkspaceName] = useState('');
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

  async function handleFinish(e: FormEvent) {
    e.preventDefault();
    if (!apiKey.trim()) return;
    setError(null);
    setLoading(true);
    try {
      const user = await completeOnboarding(workspaceName.trim(), provider, apiKey.trim());
      if (user.redeyeApiKey) {
        setGeneratedRedEyeKey(user.redeyeApiKey);
        setStep(3);
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
    openai: 'OpenAI',
    gemini: 'Google Gemini',
    groq: 'Groq',
    anthropic: 'Anthropic',
  };

  return (
    <div className="relative min-h-screen bg-slate-950 flex items-center justify-center px-4 overflow-hidden">

      {/* ── Ambient Glow Blobs ─────────────────────────────────── */}
      <div className="absolute -top-32 -left-32 w-96 h-96 rounded-full bg-cyan-500/10 blur-[120px] pointer-events-none" />
      <div className="absolute -bottom-32 -right-32 w-96 h-96 rounded-full bg-teal-500/10 blur-[120px] pointer-events-none" />

      <div className="relative w-full max-w-md z-10">

        {/* ── Brand + Step Indicators ────────────────────────────── */}
        <div className="flex items-center justify-between mb-10">
          {/* Logo */}
          <div className="flex items-center gap-2.5">
            <div className="h-8 w-8 rounded-xl bg-gradient-to-br from-cyan-500 to-teal-500 flex items-center justify-center shadow-[0_0_20px_rgba(34,211,238,0.45)]">
              <span className="text-xs font-bold text-slate-950">RE</span>
            </div>
            <span className="text-sm font-semibold text-slate-100 tracking-wide">RedEye</span>
          </div>

          {/* Step dots — neon cyan when active / complete, muted slate otherwise */}
          <div className="flex items-center gap-3">
            {([1, 2, 3] as Step[]).map((s) => (
              <div key={s} className="relative flex items-center justify-center">
                {s === step ? (
                  /* Active — glowing cyan dot */
                  <div className="h-2.5 w-2.5 rounded-full bg-cyan-400 neon-dot" />
                ) : s < step ? (
                  /* Completed — solid teal */
                  <div className="h-2.5 w-2.5 rounded-full bg-teal-500/70" />
                ) : (
                  /* Upcoming — muted slate */
                  <div className="h-2.5 w-2.5 rounded-full bg-slate-700" />
                )}
              </div>
            ))}
          </div>
        </div>

        {/* ══════════════════════════════════════════════════════════
            STEP 1 — Workspace Name
        ══════════════════════════════════════════════════════════ */}
        {step === 1 && (
          <div className="glass-panel p-8">
            <div className="flex items-center justify-center w-11 h-11 rounded-xl bg-cyan-500/10 border border-cyan-500/20 mb-5">
              <Building2 className="w-5 h-5 text-cyan-400" />
            </div>
            <h1 className="text-xl font-bold text-slate-50 mb-1.5">Name your workspace</h1>
            <p className="text-sm text-slate-400 mb-7 leading-relaxed">
              This appears in your dashboard and audit logs.
            </p>
            <form onSubmit={handleStep1} className="space-y-4">
              <input
                type="text"
                required
                autoFocus
                value={workspaceName}
                onChange={(e) => setWorkspaceName(e.target.value)}
                placeholder="e.g. Acme AI Platform"
                className="premium-input"
              />
              <button
                type="submit"
                className="w-full inline-flex items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 px-4 py-3 text-sm font-semibold text-slate-950 shadow-[0_0_20px_rgba(34,211,238,0.25)] hover:shadow-[0_0_30px_rgba(34,211,238,0.4)] transition-all duration-200"
              >
                Continue <ChevronRight className="w-4 h-4" />
              </button>
            </form>
          </div>
        )}

        {/* ══════════════════════════════════════════════════════════
            STEP 2 — LLM API Key
        ══════════════════════════════════════════════════════════ */}
        {step === 2 && (
          <div className="glass-panel p-8">
            <div className="flex items-center justify-center w-11 h-11 rounded-xl bg-cyan-500/10 border border-cyan-500/20 mb-5">
              <KeyRound className="w-5 h-5 text-cyan-400" />
            </div>
            <h1 className="text-xl font-bold text-slate-50 mb-1.5">Connect your LLM provider</h1>
            <p className="text-sm text-slate-400 mb-7 leading-relaxed">
              Your key is stored encrypted and never logged in plaintext.
            </p>

            <form onSubmit={handleFinish} className="space-y-4">
              {/* Provider Select */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold uppercase tracking-widest text-slate-500">
                  Provider
                </label>
                <select
                  value={provider}
                  onChange={(e) => setProvider(e.target.value)}
                  className="premium-input appearance-none cursor-pointer"
                >
                  {Object.entries(providerLabels).map(([value, label]) => (
                    <option key={value} value={value} className="bg-slate-900 text-slate-100">
                      {label}
                    </option>
                  ))}
                </select>
              </div>

              {/* API Key Input */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold uppercase tracking-widest text-slate-500">
                  API Key
                </label>
                <input
                  type="password"
                  required
                  autoFocus
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="Paste your API key"
                  className="premium-input font-mono tracking-widest"
                />
              </div>

              {/* ── Inline Error Alert ─────────────────────────────── */}
              {error && (
                <div className="flex items-start gap-3 rounded-xl bg-rose-500/10 border border-rose-500/30 px-4 py-3 shadow-[0_0_20px_rgba(244,63,94,0.08)]">
                  <ShieldAlert className="w-4 h-4 text-rose-400 flex-shrink-0 mt-0.5" />
                  <p className="text-xs text-rose-300 leading-relaxed">{error}</p>
                </div>
              )}

              <div className="flex gap-3 pt-1">
                <button
                  type="button"
                  onClick={() => setStep(1)}
                  className="flex-none rounded-xl border border-slate-700/80 bg-slate-900/50 px-4 py-3 text-sm font-semibold text-slate-400 hover:text-slate-200 hover:border-slate-600 transition-all duration-200"
                >
                  Back
                </button>
                <button
                  type="submit"
                  disabled={loading}
                  className="flex-1 inline-flex items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 disabled:opacity-50 disabled:cursor-not-allowed px-4 py-3 text-sm font-semibold text-slate-950 shadow-[0_0_20px_rgba(34,211,238,0.25)] hover:shadow-[0_0_30px_rgba(34,211,238,0.4)] transition-all duration-200"
                >
                  {loading ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      Verifying connection securely...
                    </>
                  ) : (
                    <>
                      Verify &amp; Next <ChevronRight className="w-4 h-4" />
                    </>
                  )}
                </button>
              </div>
            </form>
          </div>
        )}

        {/* ══════════════════════════════════════════════════════════
            STEP 3 — RedEye Key Reveal
        ══════════════════════════════════════════════════════════ */}
        {step === 3 && (
          <div className="glass-panel p-8">
            {/* Header */}
            <div className="flex items-center justify-center w-11 h-11 rounded-xl bg-teal-500/10 border border-teal-500/20 mb-5">
              <Terminal className="w-5 h-5 text-teal-400" />
            </div>
            <h1 className="text-xl font-bold text-slate-50 mb-1.5">Your RedEye Gateway Key</h1>
            <p className="text-sm text-slate-400 mb-6 leading-relaxed">
              Copy this key now. For your security,{' '}
              <strong className="text-teal-400 font-semibold">you won't be able to see it again</strong>.
            </p>

            {/* ── Hacker-Terminal Key Display ──────────────────────── */}
            <div className="relative rounded-xl bg-slate-950 border border-teal-500/30 shadow-[0_0_24px_rgba(45,212,191,0.08)] overflow-hidden mb-6">
              {/* Terminal chrome bar */}
              <div className="flex items-center gap-1.5 px-4 py-2.5 border-b border-teal-500/20 bg-slate-900/60">
                <div className="w-2.5 h-2.5 rounded-full bg-rose-500/70" />
                <div className="w-2.5 h-2.5 rounded-full bg-amber-500/70" />
                <div className="w-2.5 h-2.5 rounded-full bg-emerald-500/70" />
                <span className="ml-2 text-[10px] text-teal-500/60 font-mono uppercase tracking-widest">
                  redeye_key.env
                </span>
              </div>
              {/* Key content */}
              <div className="flex items-center justify-between gap-4 p-4">
                <code className="text-teal-400 font-mono text-sm break-all leading-relaxed flex-1">
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
                  className="flex-none p-2 rounded-lg border border-teal-500/20 bg-teal-500/5 hover:bg-teal-500/15 hover:border-teal-500/40 transition-all duration-200 text-teal-400"
                  title="Copy to clipboard"
                >
                  {copied
                    ? <Check className="w-4 h-4 text-emerald-400" />
                    : <Copy className="w-4 h-4" />}
                </button>
              </div>
            </div>

            {/* ── Proceed CTA ───────────────────────────────────────── */}
            <button
              onClick={() => navigate('/dashboard')}
              className="w-full inline-flex items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 px-4 py-3 text-sm font-semibold text-slate-950 shadow-[0_0_24px_rgba(34,211,238,0.3)] hover:shadow-[0_0_36px_rgba(34,211,238,0.5)] transition-all duration-200"
            >
              Proceed to Dashboard <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        )}

        {/* Step counter */}
        <p className="mt-5 text-center text-xs text-slate-600">Step {step} of 3</p>
      </div>
    </div>
  );
}
