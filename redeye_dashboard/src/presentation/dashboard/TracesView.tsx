// Dashboard View — TracesView
// Trace Explorer: recent spans with policy outcome badges.

import { Activity, Loader2, AlertCircle } from 'lucide-react';
import { Badge } from '../components/ui/Badge';
import useSWR from 'swr';

interface Trace {
  traceId: string;
  tenantId: string;
  model: string;
  tokens: number;
  latency: string;
  policy: string;
}

const fetcher = async (url: string) => {
  // Authentication handled via HttpOnly cookies (credentials: 'include')
  const res = await fetch(url, { 
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' }
  });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

export function TracesView() {
  const { data: traces, error, isLoading } = useSWR<Trace[]>(
    'http://localhost:8080/v1/admin/traces',
    fetcher,
    { refreshInterval: 5000 }
  );

  return (
    <div className="space-y-6 animate-in fade-in duration-500">
      <header className="flex items-center justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">Observability</p>
          <h1 className="text-2xl sm:text-3xl font-bold text-slate-900 dark:text-slate-50 font-display">Trace Explorer</h1>
          <p className="text-sm text-slate-600 dark:text-slate-400 mt-1">Inspect request flows, tenant context, and policy outcomes.</p>
        </div>
        <div className="hidden sm:flex items-center gap-2 text-slate-600 dark:text-slate-400 text-xs font-medium bg-slate-100 dark:bg-slate-900/50 px-3 py-1.5 rounded-full border border-slate-300 dark:border-slate-800">
          <Activity className={`w-4 h-4 ${error ? 'text-rose-600 dark:text-rose-400' : 'text-emerald-500 dark:text-emerald-400'}`} />
          <span>{error ? 'Tracer Offline' : 'Streaming from Cluster'}</span>
        </div>
      </header>

      <div className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-800/80 p-5 flex flex-col overflow-hidden shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2">
            Recent Traces
            {isLoading && <Loader2 className="w-3 h-3 animate-spin text-indigo-500 dark:text-indigo-400" />}
          </h2>
          <span className="text-[11px] text-slate-500 tabular-nums">
            {traces ? `${traces.length} active spans` : 'Monitoring...'}
          </span>
        </div>

        {error ? (
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <AlertCircle className="w-8 h-8 text-rose-500/50 mb-3" />
            <p className="text-sm text-slate-500 dark:text-slate-400 max-w-xs">Failed to fetch live traces. Ensure the gateway service is reachable.</p>
          </div>
        ) : (
          <div className="overflow-x-auto w-full pb-2 custom-scrollbar">
            <table className="w-full min-w-[700px] text-left border-collapse bg-transparent">
              <thead>
                <tr className="border-b border-slate-200 dark:border-slate-800 text-xs text-slate-500 dark:text-slate-400">
                  <th className="pb-3 font-medium whitespace-nowrap pl-2">Trace ID</th>
                  <th className="pb-3 font-medium whitespace-nowrap">Tenant ID</th>
                  <th className="pb-3 font-medium whitespace-nowrap">Model</th>
                  <th className="pb-3 font-medium whitespace-nowrap">Tokens</th>
                  <th className="pb-3 font-medium whitespace-nowrap">Policy Result</th>
                  <th className="pb-3 font-medium whitespace-nowrap pr-2">Latency</th>
                </tr>
              </thead>
              <tbody className="text-xs sm:text-sm">
                {isLoading && !traces ? (
                  Array.from({ length: 5 }).map((_, i) => (
                    <tr key={i} className="border-b border-slate-100 dark:border-slate-900/70 animate-pulse">
                      <td className="py-4 pl-2"><div className="h-3 w-24 bg-slate-200 dark:bg-slate-800 rounded"></div></td>
                      <td className="py-4"><div className="h-3 w-16 bg-slate-200 dark:bg-slate-800 rounded"></div></td>
                      <td className="py-4"><div className="h-3 w-12 bg-slate-200 dark:bg-slate-800 rounded"></div></td>
                      <td className="py-4"><div className="h-3 w-8 bg-slate-200 dark:bg-slate-800 rounded"></div></td>
                      <td className="py-4"><div className="h-5 w-16 bg-slate-200 dark:bg-slate-800 rounded"></div></td>
                      <td className="py-4 pr-2"><div className="h-3 w-10 bg-slate-200 dark:bg-slate-800 rounded"></div></td>
                    </tr>
                  ))
                ) : (
                  traces?.map((trace) => (
                    <tr key={trace.traceId} className="border-b border-slate-100 dark:border-slate-900/70 hover:bg-slate-50 dark:hover:bg-slate-900/70 transition-colors group">
                      <td className="py-4 text-slate-700 dark:text-slate-200 whitespace-nowrap pr-4 font-mono text-[11px] pl-2 group-hover:text-indigo-600 dark:group-hover:text-indigo-300 transition-colors">
                        {trace.traceId}
                      </td>
                      <td className="py-4 text-slate-500 dark:text-slate-400 whitespace-nowrap pr-4">{trace.tenantId}</td>
                      <td className="py-4 text-slate-600 dark:text-slate-300 whitespace-nowrap pr-4">{trace.model}</td>
                      <td className="py-4 text-slate-400 dark:text-slate-500 whitespace-nowrap pr-4 font-mono text-xs">{trace.tokens}</td>
                      <td className="py-4 whitespace-nowrap pr-4">
                        <Badge variant={trace.policy === 'Allowed' ? 'success' : 'danger'} className="px-2 py-0.5 text-[10px] font-bold tracking-wider">
                          {trace.policy.toUpperCase()}
                        </Badge>
                      </td>
                      <td className="py-4 text-slate-600 dark:text-slate-300 whitespace-nowrap pr-2 tabular-nums font-medium text-right sm:text-left">
                        {trace.latency}
                      </td>
                    </tr>
                  ))
                )}
                {!isLoading && traces?.length === 0 && (
                  <tr>
                    <td colSpan={6} className="py-12 text-center text-slate-500 text-sm italic">
                      No active traces found in the last observation window.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
