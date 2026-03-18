// Dashboard View — DashboardView
// Renders live traffic charts, stat cards, model distribution, latency histogram,
// and live request audit log. Mirrors original renderDashboard() from App.tsx.
// Theme: slate-950/indigo-500.

import { Activity, Zap, ShieldAlert, Cpu, DollarSign } from 'lucide-react';
import {
  LineChart, Line, BarChart, Bar, PieChart, Pie, Cell,
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend,
} from 'recharts';
import { StatCard } from '../components/ui/StatCard';
import { Badge } from '../components/ui/Badge';
import { mockLatencyData, mockModelData, mockAuditLogs, CHART_COLORS } from '../../data/repositories/mockData';

interface Metrics {
  total_requests: string;
  avg_latency_ms: number;
  total_tokens: string;
  rate_limited_requests: string;
}

interface DashboardViewProps {
  metrics: Metrics | null;
  chartData: { time: string; requests: number; latency: number }[];
  error: string | null;
  calculateSavedCost: () => string;
}

const tooltipStyle = {
  backgroundColor: 'rgba(15,23,42,0.92)',
  backdropFilter: 'blur(12px)',
  borderColor: 'rgba(99,102,241,0.20)',
  borderRadius: '10px',
  color: '#f1f5f9',
  fontSize: '12px',
  boxShadow: '0 0 20px rgba(99,102,241,0.10)',
};

export function DashboardView({ metrics, chartData, error, calculateSavedCost }: DashboardViewProps) {
  return (
    <div className="space-y-6">
      {/* Header */}
      <header className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-indigo-400 to-slate-200 bg-clip-text text-transparent pb-1">
            RedEye Gateway
          </h1>
          <p className="text-xs sm:text-sm text-slate-400 mt-1">Enterprise Telemetry &amp; Security Command Center</p>
        </div>
        <div className="flex items-center space-x-2 glass-panel px-3 py-1.5 sm:px-4 sm:py-2 rounded-full self-start sm:self-auto w-fit">
          <div className={`w-2 h-2 sm:w-3 sm:h-3 rounded-full ${error ? 'bg-rose-500' : 'bg-emerald-500 neon-dot'}`} />
          <span className="text-xs sm:text-sm font-medium text-slate-300">
            {error ? 'System Offline' : 'Live Sync Active'}
          </span>
        </div>
      </header>

      {/* Stat Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-3 sm:gap-4">
        <StatCard title="Total Traffic" value={metrics?.total_requests ?? '0'} icon={Activity} accentClass="text-indigo-400 ring-1 ring-indigo-400/20" />
        <StatCard title="Avg Latency" value={`${Math.round(metrics?.avg_latency_ms ?? 0)} ms`} icon={Zap} accentClass="text-violet-400 ring-1 ring-violet-400/20" />
        <StatCard title="Tokens Processed" value={metrics?.total_tokens ?? '0'} icon={Cpu} accentClass="text-sky-400 ring-1 ring-sky-400/20" />
        <StatCard title="Threats Blocked" value={metrics?.rate_limited_requests ?? '0'} icon={ShieldAlert} accentClass="text-rose-400 ring-1 ring-rose-400/20" />
        <div className="sm:col-span-2 lg:col-span-1 xl:col-span-1">
          <StatCard
            title="API Cost Saved"
            value={`$${calculateSavedCost()}`}
            icon={DollarSign}
            accentClass="text-emerald-400 ring-1 ring-emerald-400/20"
            subtitle="Prevented by Gateway"
          />
        </div>
      </div>

      {/* Charts Row 1 */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-2">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Live Traffic Overview</h2>
          <div className="h-[200px] sm:h-[250px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(99,102,241,0.08)" vertical={false} />
                <XAxis dataKey="time" stroke="#94a3b8" fontSize={10} tickMargin={8} />
                <YAxis stroke="#94a3b8" fontSize={10} width={30} />
                <Tooltip contentStyle={tooltipStyle} />
                <Line type="monotone" dataKey="requests" stroke="#6366f1" strokeWidth={2} dot={{ r: 3, fill: '#6366f1' }} activeDot={{ r: 5, fill: '#818cf8' }} />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-1">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Model Distribution</h2>
          <div className="h-[200px] sm:h-[250px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie data={mockModelData} innerRadius="60%" outerRadius="80%" paddingAngle={5} dataKey="value">
                  {mockModelData.map((_, index) => (
                    <Cell key={`cell-${index}`} fill={CHART_COLORS[index % CHART_COLORS.length]} />
                  ))}
                </Pie>
                <Tooltip contentStyle={tooltipStyle} />
                <Legend verticalAlign="bottom" height={36} wrapperStyle={{ fontSize: '11px', color: '#94a3b8' }} />
              </PieChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      {/* Charts Row 2 */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-1">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Latency Histogram</h2>
          <div className="h-[200px] sm:h-[250px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={mockLatencyData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(99,102,241,0.08)" vertical={false} />
                <XAxis dataKey="bucket" stroke="#94a3b8" fontSize={10} tickMargin={8} />
                <YAxis stroke="#94a3b8" fontSize={10} width={30} />
                <Tooltip cursor={{ fill: 'rgba(99,102,241,0.06)' }} contentStyle={tooltipStyle} />
                <Bar dataKey="count" fill="#6366f1" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-2 flex flex-col overflow-hidden">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Live Request Audit Log</h2>
          <div className="overflow-x-auto w-full pb-2 custom-scrollbar">
            <table className="w-full text-left border-collapse min-w-[600px] bg-transparent">
              <thead>
                <tr className="border-b border-indigo-500/10 text-xs sm:text-sm text-slate-400">
                  <th className="pb-2 sm:pb-3 font-medium whitespace-nowrap">Request ID</th>
                  <th className="pb-2 sm:pb-3 font-medium whitespace-nowrap">Tenant</th>
                  <th className="pb-2 sm:pb-3 font-medium whitespace-nowrap">Model</th>
                  <th className="pb-2 sm:pb-3 font-medium whitespace-nowrap">Status</th>
                  <th className="pb-2 sm:pb-3 font-medium whitespace-nowrap">Latency</th>
                  <th className="pb-2 sm:pb-3 font-medium text-right whitespace-nowrap">Time</th>
                </tr>
              </thead>
              <tbody className="text-xs sm:text-sm">
                {mockAuditLogs.map((log, i) => (
                  <tr key={i} className="border-b border-slate-800/60 hover:bg-indigo-500/[0.03] transition-colors">
                    <td className="py-2 sm:py-3 text-slate-300 font-mono text-[10px] sm:text-xs whitespace-nowrap pr-4">{log.id}</td>
                    <td className="py-2 sm:py-3 text-indigo-400 whitespace-nowrap pr-4">{log.tenant}</td>
                    <td className="py-2 sm:py-3 text-slate-300 whitespace-nowrap pr-4">{log.model}</td>
                    <td className="py-2 sm:py-3 whitespace-nowrap pr-4">
                      <Badge variant={log.status === 200 ? 'success' : 'danger'}>{log.status}</Badge>
                    </td>
                    <td className="py-2 sm:py-3 text-slate-300 whitespace-nowrap pr-4">{log.latency}</td>
                    <td className="py-2 sm:py-3 text-slate-500 text-right whitespace-nowrap">{log.time}</td>
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
