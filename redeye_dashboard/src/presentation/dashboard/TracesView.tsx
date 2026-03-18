// Dashboard View — TracesView
// Trace Explorer: recent spans with policy outcome badges.

import { Activity } from 'lucide-react';
import { Badge } from '../components/ui/Badge';
import { mockTraces } from '../../data/repositories/mockData';

export function TracesView() {
  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">Observability</p>
          <h1 className="text-2xl sm:text-3xl font-bold text-slate-50">Trace Explorer</h1>
          <p className="text-sm text-slate-400 mt-1">Inspect request flows, tenant context, and policy outcomes.</p>
        </div>
        <div className="hidden sm:flex items-center gap-2 text-slate-400 text-xs font-medium">
          <Activity className="w-4 h-4 text-emerald-400" />
          <span>Streaming from tracer @8082</span>
        </div>
      </header>

      <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 flex flex-col overflow-hidden">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-semibold text-slate-100">Recent Traces</h2>
          <span className="text-[11px] text-slate-500">Most recent {mockTraces.length} spans</span>
        </div>
        <div className="overflow-x-auto w-full pb-2 custom-scrollbar">
          <table className="w-full min-w-[540px] text-left border-collapse bg-transparent">
            <thead>
              <tr className="border-b border-slate-800 text-xs text-slate-400">
                <th className="pb-2 font-medium whitespace-nowrap">Trace ID</th>
                <th className="pb-2 font-medium whitespace-nowrap">Tenant ID</th>
                <th className="pb-2 font-medium whitespace-nowrap">Policy Result</th>
                <th className="pb-2 font-medium whitespace-nowrap">Latency</th>
              </tr>
            </thead>
            <tbody className="text-xs sm:text-sm">
              {mockTraces.map((trace) => (
                <tr key={trace.traceId} className="border-b border-slate-900/70 hover:bg-slate-900/70 transition-colors">
                  <td className="py-2 text-slate-200 whitespace-nowrap pr-4 font-mono text-[11px]">{trace.traceId}</td>
                  <td className="py-2 text-slate-300 whitespace-nowrap pr-4">{trace.tenantId}</td>
                  <td className="py-2 whitespace-nowrap pr-4">
                    <Badge variant={trace.policy === 'Allowed' ? 'success' : 'danger'}>{trace.policy}</Badge>
                  </td>
                  <td className="py-2 text-slate-300 whitespace-nowrap pr-4">{trace.latency}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
