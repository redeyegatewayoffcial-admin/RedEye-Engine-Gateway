// Dashboard View — ComplianceView
// Compliance Oversight: PII redaction count + data residency routing table.

import { ShieldCheck } from 'lucide-react';
import { Badge } from '../components/ui/Badge';
import { mockRedactedEntities, mockResidencyRoutes } from '../../data/repositories/mockData';

export function ComplianceView() {
  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">Compliance</p>
          <h1 className="text-2xl sm:text-3xl font-bold text-slate-50">Compliance Oversight</h1>
          <p className="text-sm text-slate-400 mt-1">Monitor PII redaction activity and data residency routing.</p>
        </div>
        <div className="hidden sm:flex items-center gap-2 text-emerald-400 text-xs font-medium">
          <ShieldCheck className="w-4 h-4" />
          <span>Policies enforced in real time</span>
        </div>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 lg:col-span-1">
          <p className="text-xs font-medium text-slate-400 mb-2">Redacted Entities</p>
          <p className="text-4xl font-semibold text-emerald-400 tracking-tight">
            {mockRedactedEntities.toLocaleString()}
          </p>
          <p className="text-xs text-slate-500 mt-2">
            Total entities masked across all tenants in the last 24 hours.
          </p>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 lg:col-span-2 overflow-hidden">
          <div className="flex items-center justify-between mb-3">
            <div>
              <h2 className="text-sm font-semibold text-slate-100">Data Residency Routing</h2>
              <p className="text-xs text-slate-500">Region-aware routing for strict localization and isolation rules.</p>
            </div>
          </div>
          <div className="overflow-x-auto pb-2 custom-scrollbar">
            <table className="w-full min-w-[520px] text-left border-collapse bg-transparent">
              <thead>
                <tr className="border-b border-slate-800 text-xs text-slate-400">
                  <th className="pb-2 font-medium whitespace-nowrap">Region</th>
                  <th className="pb-2 font-medium whitespace-nowrap">Endpoint</th>
                  <th className="pb-2 font-medium whitespace-nowrap">Strict Isolation</th>
                </tr>
              </thead>
              <tbody className="text-xs sm:text-sm">
                {mockResidencyRoutes.map((row) => (
                  <tr key={row.region} className="border-b border-slate-900/70 hover:bg-slate-900/70 transition-colors">
                    <td className="py-2 text-slate-200 whitespace-nowrap pr-4 font-mono text-[11px]">{row.region}</td>
                    <td className="py-2 text-indigo-300 whitespace-nowrap pr-4 text-xs">{row.endpoint}</td>
                    <td className="py-2 whitespace-nowrap">
                      <Badge variant={row.isolation === 'Strict' ? 'success' : 'danger'}>{row.isolation}</Badge>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
