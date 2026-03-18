// Presentation Page — LandingPage
// Minimal Hero, visual data flow, 3 dummy client logos, Login CTA.
// Theme: slate-950 bg, indigo-500 primary, massive whitespace.

import { useNavigate } from 'react-router-dom';
import { ArrowRight, Shield, Zap, Globe } from 'lucide-react';

const CLIENTS = [
  { name: 'Acme Corp', abbr: 'AC' },
  { name: 'Globex Inc', abbr: 'GX' },
  { name: 'Stark Industries', abbr: 'SI' },
];

const FEATURES = [
  { icon: Shield, title: 'Policy Enforcement', desc: 'OPA-native compliance at every request boundary.' },
  { icon: Zap, title: 'Sub-50ms Routing', desc: 'Semantic cache + smart load balancing out of the box.' },
  { icon: Globe, title: 'Data Residency', desc: 'Region-locked routing for GDPR, HIPAA, and SOC 2.' },
];

export function LandingPage() {
  const navigate = useNavigate();

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 flex flex-col">
      {/* Nav */}
      <nav className="border-b border-slate-800/60 px-6 sm:px-12 lg:px-20 h-16 flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <div className="h-7 w-7 rounded-lg bg-indigo-600 flex items-center justify-center shadow-[0_0_16px_rgba(99,102,241,0.5)]">
            <span className="text-[10px] font-bold tracking-tight text-white">RE</span>
          </div>
          <span className="text-sm font-semibold text-slate-100">RedEye</span>
        </div>
        <button
          onClick={() => navigate('/login')}
          className="text-sm font-medium text-slate-400 hover:text-slate-100 transition-colors"
        >
          Sign in →
        </button>
      </nav>

      {/* Hero */}
      <main className="flex-1 flex flex-col items-center justify-center px-6 text-center py-24 sm:py-32">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-indigo-500/25 bg-indigo-500/5 text-indigo-400 text-xs font-medium mb-8">
          <span className="w-1.5 h-1.5 rounded-full bg-indigo-400 inline-block" />
          Enterprise AI Gateway — v2
        </div>

        <h1 className="text-4xl sm:text-6xl lg:text-7xl font-extrabold tracking-tight text-white leading-[1.08] max-w-4xl">
          Enterprise AI Gateway
        </h1>
        <p className="mt-6 text-lg sm:text-xl text-slate-400 max-w-2xl leading-relaxed">
          Intelligent middleware that sits between your clients and AI providers — enforcing policy,
          redacting PII, and optimizing cost at every request.
        </p>

        <div className="mt-10 flex flex-col sm:flex-row items-center gap-4 justify-center">
          <button
            onClick={() => navigate('/login')}
            className="inline-flex items-center gap-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 px-6 py-3 text-sm font-semibold text-white shadow-[0_0_24px_rgba(99,102,241,0.35)] transition-all duration-200"
          >
            Get started free <ArrowRight className="w-4 h-4" />
          </button>
          <button
            onClick={() => navigate('/login')}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 bg-slate-900/50 hover:border-slate-600 px-6 py-3 text-sm font-semibold text-slate-300 transition-all duration-200"
          >
            Sign in
          </button>
        </div>

        {/* Flow Diagram */}
        <div className="mt-20 flex flex-col sm:flex-row items-center justify-center gap-0 sm:gap-0 w-full max-w-2xl">
          {[
            { label: 'Client', sub: 'Your product' },
            null,
            { label: 'RedEye', sub: 'This gateway', highlight: true },
            null,
            { label: 'OpenAI', sub: 'AI provider' },
          ].map((item, i) =>
            item === null ? (
              <div key={i} className="flex items-center justify-center w-12 sm:w-16 flex-shrink-0">
                <div className="h-px w-full bg-gradient-to-r from-slate-700 via-indigo-500/50 to-slate-700" />
                <ArrowRight className="w-3 h-3 text-indigo-500 -ml-1 flex-shrink-0" />
              </div>
            ) : (
              <div
                key={i}
                className={`flex-1 flex flex-col items-center gap-1 px-4 py-3 rounded-xl border ${
                  item.highlight
                    ? 'border-indigo-500/40 bg-indigo-500/10 shadow-[0_0_24px_rgba(99,102,241,0.15)]'
                    : 'border-slate-800 bg-slate-900/40'
                }`}
              >
                <span className={`text-sm font-bold ${item.highlight ? 'text-indigo-300' : 'text-slate-200'}`}>
                  {item.label}
                </span>
                <span className="text-[11px] text-slate-500">{item.sub}</span>
              </div>
            ),
          )}
        </div>

        {/* Feature Pills */}
        <div className="mt-20 grid grid-cols-1 sm:grid-cols-3 gap-4 max-w-3xl w-full">
          {FEATURES.map(({ icon: Icon, title, desc }) => (
            <div
              key={title}
              className="glass-panel bg-slate-900/30 border border-slate-800/60 p-5 text-left hover:border-indigo-500/30 transition-colors duration-300"
            >
              <Icon className="w-5 h-5 text-indigo-400 mb-3" />
              <p className="text-sm font-semibold text-slate-100 mb-1">{title}</p>
              <p className="text-xs text-slate-500 leading-relaxed">{desc}</p>
            </div>
          ))}
        </div>

        {/* Dummy Client Logos */}
        <div className="mt-20">
          <p className="text-xs uppercase tracking-[0.2em] text-slate-600 mb-6">Trusted by teams at</p>
          <div className="flex items-center justify-center gap-8 flex-wrap">
            {CLIENTS.map(({ name, abbr }) => (
              <div key={name} className="flex items-center gap-2 opacity-40 hover:opacity-60 transition-opacity">
                <div className="h-8 w-8 rounded-lg bg-slate-700 flex items-center justify-center text-[11px] font-bold text-slate-300">
                  {abbr}
                </div>
                <span className="text-slate-400 text-sm font-medium">{name}</span>
              </div>
            ))}
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="border-t border-slate-800/60 px-6 py-5 text-center text-xs text-slate-600">
        © {new Date().getFullYear()} RedEye AI Engine — Enterprise AI Gateway
      </footer>
    </div>
  );
}
