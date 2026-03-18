// Presentation Page — OnboardingWizard
// Step 1: Workspace name | Step 2: OpenAI API key
// On finish → navigate /dashboard

import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { Loader2, ChevronRight, KeyRound, Building2 } from 'lucide-react';

type Step = 1 | 2;

export function OnboardingWizard() {
  const navigate = useNavigate();
  const { completeOnboarding } = useAuth();

  const [step, setStep] = useState<Step>(1);
  const [workspaceName, setWorkspaceName] = useState('');
  const [openAiApiKey, setOpenAiApiKey] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  function handleStep1(e: FormEvent) {
    e.preventDefault();
    if (!workspaceName.trim()) return;
    setStep(2);
  }

  async function handleFinish(e: FormEvent) {
    e.preventDefault();
    if (!openAiApiKey.trim()) return;
    setError(null);
    setLoading(true);
    try {
      await completeOnboarding(workspaceName.trim(), openAiApiKey.trim());
      navigate('/dashboard');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Something went wrong.');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen bg-slate-950 flex items-center justify-center px-4">
      <div className="w-full max-w-md">
        {/* Brand + step indicator */}
        <div className="flex items-center justify-between mb-10">
          <div className="flex items-center gap-2.5">
            <div className="h-8 w-8 rounded-xl bg-indigo-600 flex items-center justify-center shadow-[0_0_20px_rgba(99,102,241,0.5)]">
              <span className="text-xs font-bold text-white">RE</span>
            </div>
            <span className="text-sm font-semibold text-slate-100">RedEye</span>
          </div>
          {/* Step bubbles */}
          <div className="flex items-center gap-2">
            {([1, 2] as Step[]).map((s) => (
              <div
                key={s}
                className={`h-2 w-2 rounded-full transition-colors ${
                  s === step ? 'bg-indigo-500' : s < step ? 'bg-indigo-500/40' : 'bg-slate-700'
                }`}
              />
            ))}
          </div>
        </div>

        {/* Step 1 */}
        {step === 1 && (
          <div className="glass-panel bg-slate-900/50 border border-slate-800 p-8">
            <Building2 className="w-7 h-7 text-indigo-400 mb-4" />
            <h1 className="text-xl font-bold text-slate-50 mb-1">Name your workspace</h1>
            <p className="text-sm text-slate-400 mb-7">
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
                className="w-full rounded-lg bg-slate-950/70 border border-slate-800 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-600 focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 transition-colors"
              />
              <button
                type="submit"
                className="w-full inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 px-4 py-2.5 text-sm font-semibold text-white shadow-[0_0_20px_rgba(99,102,241,0.25)] transition-all duration-200"
              >
                Continue <ChevronRight className="w-4 h-4" />
              </button>
            </form>
          </div>
        )}

        {/* Step 2 */}
        {step === 2 && (
          <div className="glass-panel bg-slate-900/50 border border-slate-800 p-8">
            <KeyRound className="w-7 h-7 text-indigo-400 mb-4" />
            <h1 className="text-xl font-bold text-slate-50 mb-1">Connect your OpenAI key</h1>
            <p className="text-sm text-slate-400 mb-7">
              Your key is stored encrypted and never logged in plaintext.
            </p>
            <form onSubmit={handleFinish} className="space-y-4">
              <input
                type="password"
                required
                autoFocus
                value={openAiApiKey}
                onChange={(e) => setOpenAiApiKey(e.target.value)}
                placeholder="sk-••••••••••••••••"
                className="w-full rounded-lg bg-slate-950/70 border border-slate-800 px-3 py-2.5 text-sm text-slate-100 font-mono placeholder:text-slate-600 focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 transition-colors"
              />

              {error && (
                <p className="text-xs text-rose-400 bg-rose-500/10 border border-rose-500/20 rounded-lg px-3 py-2">
                  {error}
                </p>
              )}

              <div className="flex gap-3">
                <button
                  type="button"
                  onClick={() => setStep(1)}
                  className="flex-none rounded-lg border border-slate-700 bg-slate-900/50 px-4 py-2.5 text-sm font-semibold text-slate-400 hover:text-slate-200 transition-colors"
                >
                  Back
                </button>
                <button
                  type="submit"
                  disabled={loading}
                  className="flex-1 inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 disabled:opacity-60 disabled:cursor-not-allowed px-4 py-2.5 text-sm font-semibold text-white shadow-[0_0_20px_rgba(99,102,241,0.25)] transition-all duration-200"
                >
                  {loading && <Loader2 className="w-4 h-4 animate-spin" />}
                  Finish setup
                </button>
              </div>
            </form>
          </div>
        )}

        <p className="mt-5 text-center text-xs text-slate-600">Step {step} of 2</p>
      </div>
    </div>
  );
}
