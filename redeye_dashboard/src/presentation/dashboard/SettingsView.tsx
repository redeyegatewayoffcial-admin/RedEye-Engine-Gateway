// Dashboard View — SettingsView
// Owns its own local state for the 4 service endpoint URLs.

import { useState } from 'react';
import { Settings as SettingsIcon } from 'lucide-react';

export function SettingsView() {
  const [gatewayUrl, setGatewayUrl] = useState('http://localhost:8080');
  const [cacheUrl, setCacheUrl] = useState('http://localhost:8081');
  const [tracerUrl, setTracerUrl] = useState('http://localhost:8082');
  const [complianceUrl, setComplianceUrl] = useState('http://localhost:8083');

  return (
    <div className="space-y-6">
      <header>
        <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">Configuration</p>
        <h1 className="text-2xl sm:text-3xl font-bold text-slate-900 dark:text-slate-50">Service Endpoints</h1>
        <p className="text-sm text-slate-600 dark:text-slate-400 mt-1">Manage internal API targets for RedEye microservices.</p>
      </header>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
        {[
          { label: 'Gateway', desc: 'Traffic, rate limiting & policy enforcement.', value: gatewayUrl, set: setGatewayUrl, def: '8080' },
          { label: 'Semantic Cache', desc: 'Vector-aware cache for repeated prompts.', value: cacheUrl, set: setCacheUrl, def: '8081' },
          { label: 'Tracer', desc: 'Distributed traces and audit-grade spans.', value: tracerUrl, set: setTracerUrl, def: '8082' },
          { label: 'Compliance', desc: 'PII redaction and residency enforcement.', value: complianceUrl, set: setComplianceUrl, def: '8083' },
        ].map(({ label, desc, value, set, def }) => (
          <div key={label} className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-700/50 p-5 shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            <p className="text-xs font-medium text-slate-500 dark:text-slate-400 mb-1">{label}</p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3">{desc}</p>
            <input
              className="w-full rounded-md bg-white dark:bg-slate-950/60 border border-slate-300 dark:border-slate-800 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-2 focus:ring-rose-500/50 focus:border-transparent transition-all duration-200"
              value={value}
              onChange={(e) => set(e.target.value)}
            />
            <p className="text-[11px] text-slate-500 mt-2">Default: http://localhost:{def}</p>
          </div>
        ))}
      </div>

      <div className="flex items-center justify-end">
        <button
          type="button"
          className="inline-flex items-center gap-2 rounded-md bg-slate-100 dark:bg-slate-100/5 border border-slate-200 dark:border-slate-700 px-4 py-2 text-xs font-semibold text-slate-600 dark:text-slate-200 hover:bg-slate-200 dark:hover:bg-slate-100/10 active:scale-95 transition-all duration-200 ease-in-out cursor-default focus:outline-none focus:ring-2 focus:ring-rose-500/50 focus:ring-offset-1 dark:focus:ring-offset-slate-900"
        >
          <SettingsIcon className="w-3 h-3" />
          <span>Settings are local to this session</span>
        </button>
      </div>
    </div>
  );
}
