import { useEffect, useState } from 'react';
import {
  Activity,
  Zap,
  ShieldAlert,
  Cpu,
  DollarSign,
  LayoutDashboard,
  ShieldCheck,
  Database,
  Settings as SettingsIcon,
} from 'lucide-react';
import { 
  LineChart, Line, BarChart, Bar, PieChart, Pie, Cell, 
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend 
} from 'recharts';

// --- MOCK DATA ---
const mockLatencyData = [
  { bucket: '0-50ms', count: 420 },
  { bucket: '50-100ms', count: 210 },
  { bucket: '100-200ms', count: 80 },
  { bucket: '200-500ms', count: 30 },
  { bucket: '500ms+', count: 10 },
];

const mockModelData = [
  { name: 'GPT-4o', value: 65 },
  { name: 'Gemini-2.5', value: 25 },
  { name: 'Claude-3', value: 10 },
];
const COLORS = ['#ef4444', '#f97316', '#f59e0b'];

const mockAuditLogs = [
  { id: 'req-982', tenant: 'acme-corp', model: 'gpt-4o', status: 200, latency: '42ms', time: 'Just now' },
  { id: 'req-981', tenant: 'globex-inc', model: 'gemini-2.5', status: 429, latency: '12ms', time: '2s ago' },
  { id: 'req-980', tenant: 'acme-corp', model: 'gpt-4o', status: 200, latency: '85ms', time: '5s ago' },
  { id: 'req-979', tenant: 'stark-ind', model: 'claude-3.5', status: 200, latency: '110ms', time: '12s ago' },
  { id: 'req-978', tenant: 'globex-inc', model: 'gpt-4o', status: 429, latency: '8ms', time: '15s ago' },
];

const mockRedactedEntities = 1287;

const mockResidencyRoutes = [
  { region: 'us-east', endpoint: 'https://gateway.us-east.redeye', isolation: 'Strict' },
  { region: 'eu-central', endpoint: 'https://gateway.eu-central.redeye', isolation: 'Strict' },
  { region: 'ap-south', endpoint: 'https://gateway.ap-south.redeye', isolation: 'Relaxed' },
];

const mockTraces = [
  { traceId: 'trace-9012', tenantId: 'acme-corp', policy: 'Allowed', latency: '48ms' },
  { traceId: 'trace-9011', tenantId: 'globex-inc', policy: 'Blocked', latency: '15ms' },
  { traceId: 'trace-9010', tenantId: 'stark-ind', policy: 'Allowed', latency: '92ms' },
];

const mockCacheStats = {
  hitRatio: 0.82,
  missRatio: 0.18,
  totalLookups: 42319,
};

type ViewId = 'dashboard' | 'compliance' | 'traces' | 'cache' | 'settings';

const NAV_ITEMS: { id: ViewId; label: string; icon: any; description: string }[] = [
  {
    id: 'dashboard',
    label: 'Dashboard',
    icon: LayoutDashboard,
    description: 'Overall health & traffic',
  },
  {
    id: 'compliance',
    label: 'Compliance Oversight',
    icon: ShieldCheck,
    description: 'PII redaction & residency',
  },
  {
    id: 'traces',
    label: 'Trace Explorer',
    icon: Activity,
    description: 'Recent traces & audits',
  },
  {
    id: 'cache',
    label: 'Semantic Cache',
    icon: Database,
    description: 'Hit & miss ratios',
  },
  {
    id: 'settings',
    label: 'Settings',
    icon: SettingsIcon,
    description: 'Service endpoints',
  },
];

interface Metrics {
  total_requests: string;
  avg_latency_ms: number;
  total_tokens: string;
  rate_limited_requests: string;
}

// Responsive Stat Card — Glassmorphism + Neon Red
function StatCard({ title, value, icon: Icon, colorClass, subtitle }: { title: string, value: string | number, icon: any, colorClass: string, subtitle?: string }) {
  return (
    <div className="group glass-panel p-4 sm:p-6 flex items-start space-x-3 sm:space-x-4 hover:-translate-y-1 hover:shadow-[0_0_25px_rgba(239,68,68,0.15)] transition-all duration-300 w-full">
      <div className={`p-2 sm:p-3 rounded-xl bg-black/50 backdrop-blur-md border border-red-500/10 flex-shrink-0 ${colorClass} group-hover:scale-110 transition-transform duration-300`}>
        <Icon className="w-5 h-5 sm:w-6 sm:h-6" />
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-xs sm:text-sm font-medium text-red-400/70 truncate">{title}</p>
        <h3 className="text-xl sm:text-2xl font-bold mt-1 tracking-tight text-slate-100 truncate">{value}</h3>
        {subtitle && <p className="text-[10px] sm:text-xs text-orange-500 mt-1 font-medium truncate">{subtitle}</p>}
      </div>
    </div>
  );
}

function App() {
  const [activeView, setActiveView] = useState<ViewId>('dashboard');
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [chartData, setChartData] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);

  const fetchMetrics = async () => {
    try {
      const res = await fetch('http://localhost:8080/v1/admin/metrics');
      if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
      const data = await res.json();
      setMetrics(data);
      setError(null);

      const now = new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second:'2-digit' });
      setChartData(prev => {
        const newData = [...prev, { 
          time: now, 
          requests: parseInt(data.total_requests) || Math.floor(Math.random() * 10), 
          latency: Math.round(data.avg_latency_ms) 
        }];
        return newData.slice(-10);
      });

    } catch (err: any) {
      setError(err.message);
    } 
  };

  useEffect(() => {
    fetchMetrics();
    const interval = setInterval(fetchMetrics, 3000);
    return () => clearInterval(interval);
  }, []);

  const calculateSavedCost = () => {
    if (!metrics) return "0.00";
    const blocked = parseInt(metrics.rate_limited_requests);
    return (blocked * 0.005).toFixed(2);
  };

  // Shared Tooltip Style — Cyber Glass
  const tooltipStyle = {
    backgroundColor: "rgba(0,0,0,0.70)",
    backdropFilter: "blur(12px)",
    borderColor: "rgba(239,68,68,0.20)",
    borderRadius: "10px",
    color: "#fff",
    fontSize: "12px",
    boxShadow: "0 0 20px rgba(239,68,68,0.08)"
  };

  const renderDashboard = () => (
    <div className="space-y-6">
      {/* Header */}
      <header className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-red-500 to-orange-500 bg-clip-text text-transparent pb-1">
            RedEye Gateway
          </h1>
          <p className="text-xs sm:text-sm text-red-400/60 mt-1">Enterprise Telemetry &amp; Security Command Center</p>
        </div>
        <div className="flex items-center space-x-2 glass-panel px-3 py-1.5 sm:px-4 sm:py-2 rounded-full self-start sm:self-auto w-fit">
          <div className={`w-2 h-2 sm:w-3 sm:h-3 rounded-full ${error ? 'bg-rose-500' : 'bg-emerald-500 neon-dot'}`} />
          <span className="text-xs sm:text-sm font-medium text-slate-300">
            {error ? 'System Offline' : 'Live Sync Active'}
          </span>
        </div>
      </header>

      {/* Stats */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-3 sm:gap-4">
        <StatCard title="Total Traffic" value={metrics?.total_requests || "0"} icon={Activity} colorClass="text-red-400 ring-1 ring-red-400/20" />
        <StatCard title="Avg Latency" value={`${Math.round(metrics?.avg_latency_ms || 0)} ms`} icon={Zap} colorClass="text-orange-400 ring-1 ring-orange-400/20" />
        <StatCard title="Tokens Processed" value={metrics?.total_tokens || "0"} icon={Cpu} colorClass="text-amber-400 ring-1 ring-amber-400/20" />
        <StatCard title="Threats Blocked" value={metrics?.rate_limited_requests || "0"} icon={ShieldAlert} colorClass="text-red-500 ring-1 ring-red-500/20" />
        <div className="sm:col-span-2 lg:col-span-1 xl:col-span-1">
          <StatCard title="API Cost Saved" value={`$${calculateSavedCost()}`} icon={DollarSign} colorClass="text-orange-500 ring-1 ring-orange-500/20" subtitle="Prevented by Gateway" />
        </div>
      </div>

      {/* Charts Row 1 */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-2">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Live Traffic Overview</h2>
          <div className="h-[200px] sm:h-[250px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(239,68,68,0.08)" vertical={false} />
                <XAxis dataKey="time" stroke="#94a3b8" fontSize={10} tickMargin={8} />
                <YAxis stroke="#94a3b8" fontSize={10} width={30} />
                <Tooltip contentStyle={tooltipStyle} />
                <Line type="monotone" dataKey="requests" stroke="#ef4444" strokeWidth={2} dot={{ r: 3, fill: '#ef4444' }} activeDot={{ r: 5, fill: '#f97316' }} />
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
                    <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
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
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(239,68,68,0.08)" vertical={false} />
                <XAxis dataKey="bucket" stroke="#94a3b8" fontSize={10} tickMargin={8} />
                <YAxis stroke="#94a3b8" fontSize={10} width={30} />
                <Tooltip cursor={{ fill: 'rgba(239,68,68,0.06)' }} contentStyle={tooltipStyle} />
                <Bar dataKey="count" fill="#ef4444" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* TABLE - Horizontal Scroll Magic for Mobile */}
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-2 flex flex-col overflow-hidden">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Live Request Audit Log</h2>

          <div className="overflow-x-auto w-full pb-2 custom-scrollbar">
            <table className="w-full text-left border-collapse min-w-[600px] bg-transparent">
              <thead>
                <tr className="border-b border-red-500/10 text-xs sm:text-sm text-red-400/60">
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
                  <tr key={i} className="border-b border-red-500/10 hover:bg-red-500/[0.03] transition-colors">
                    <td className="py-2 sm:py-3 text-slate-300 font-mono text-[10px] sm:text-xs whitespace-nowrap pr-4">{log.id}</td>
                    <td className="py-2 sm:py-3 text-orange-400 whitespace-nowrap pr-4">{log.tenant}</td>
                    <td className="py-2 sm:py-3 text-slate-300 whitespace-nowrap pr-4">{log.model}</td>
                    <td className="py-2 sm:py-3 whitespace-nowrap pr-4">
                      <span className={`px-2 py-1 rounded text-[10px] sm:text-xs font-semibold ${log.status === 200 ? 'bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/20' : 'bg-rose-500/10 text-rose-400 ring-1 ring-rose-500/20'}`}>
                        {log.status}
                      </span>
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

  const renderCompliance = () => (
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
                    <td className="py-2 text-emerald-300 whitespace-nowrap pr-4 text-xs">{row.endpoint}</td>
                    <td className="py-2 whitespace-nowrap">
                      <span
                        className={`inline-flex items-center px-2 py-1 rounded-full text-[10px] font-semibold ${
                          row.isolation === 'Strict'
                            ? 'bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/20'
                            : 'bg-rose-500/10 text-rose-400 ring-1 ring-rose-500/20'
                        }`}
                      >
                        {row.isolation}
                      </span>
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

  const renderTraces = () => (
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
                    <span
                      className={`inline-flex items-center px-2 py-1 rounded-full text-[10px] font-semibold ${
                        trace.policy === 'Allowed'
                          ? 'bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/20'
                          : 'bg-rose-500/10 text-rose-400 ring-1 ring-rose-500/20'
                      }`}
                    >
                      {trace.policy}
                    </span>
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

  const renderCache = () => (
    <div className="space-y-6">
      <header className="flex items-center justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">Performance</p>
          <h1 className="text-2xl sm:text-3xl font-bold text-slate-50">Semantic Cache</h1>
          <p className="text-sm text-slate-400 mt-1">
            Measure cache efficiency for embeddings and model responses across tenants.
          </p>
        </div>
        <div className="hidden sm:flex items-center gap-2 text-slate-400 text-xs font-medium">
          <Database className="w-4 h-4 text-emerald-400" />
          <span>Cache @8081</span>
        </div>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 lg:col-span-2">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-sm font-semibold text-slate-100">Hit / Miss Overview</h2>
              <p className="text-xs text-slate-500">Recent semantic lookups across all services.</p>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-4">
            <div>
              <p className="text-xs font-medium text-slate-400 mb-1">Hit Ratio</p>
              <p className="text-3xl font-semibold text-emerald-400">
                {(mockCacheStats.hitRatio * 100).toFixed(1)}%
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-slate-400 mb-1">Miss Ratio</p>
              <p className="text-3xl font-semibold text-rose-400">
                {(mockCacheStats.missRatio * 100).toFixed(1)}%
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-slate-400 mb-1">Total Lookups</p>
              <p className="text-3xl font-semibold text-slate-100">
                {mockCacheStats.totalLookups.toLocaleString()}
              </p>
            </div>
          </div>

          <p className="text-xs text-slate-500 mt-4">
            High cache hit ratios reduce both end-to-end latency and downstream token spend for LLM calls.
          </p>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 lg:col-span-1">
          <h2 className="text-sm font-semibold text-slate-100 mb-2">Cache Insights</h2>
          <ul className="space-y-2 text-xs text-slate-400">
            <li>• Embedding similarity thresholds tuned for low drift.</li>
            <li>• Multi-tenant partitioning prevents cross-tenant cache bleed.</li>
            <li>• Cache warming applied for high-traffic tenants.</li>
          </ul>
        </div>
      </div>
    </div>
  );

  const [gatewayUrl, setGatewayUrl] = useState('http://localhost:8080');
  const [cacheUrl, setCacheUrl] = useState('http://localhost:8081');
  const [tracerUrl, setTracerUrl] = useState('http://localhost:8082');
  const [complianceUrl, setComplianceUrl] = useState('http://localhost:8083');

  const renderSettings = () => (
    <div className="space-y-6">
      <header>
        <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">Configuration</p>
        <h1 className="text-2xl sm:text-3xl font-bold text-slate-50">Service Endpoints</h1>
        <p className="text-sm text-slate-400 mt-1">Manage internal API targets for RedEye microservices.</p>
      </header>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5">
          <p className="text-xs font-medium text-slate-400 mb-1">Gateway</p>
          <p className="text-sm text-slate-300 mb-3">Traffic, rate limiting &amp; policy enforcement.</p>
          <input
            className="w-full rounded-md bg-slate-950/60 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-emerald-500"
            value={gatewayUrl}
            onChange={(e) => setGatewayUrl(e.target.value)}
          />
          <p className="text-[11px] text-slate-500 mt-2">Default: http://localhost:8080</p>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5">
          <p className="text-xs font-medium text-slate-400 mb-1">Semantic Cache</p>
          <p className="text-sm text-slate-300 mb-3">Vector-aware cache for repeated prompts.</p>
          <input
            className="w-full rounded-md bg-slate-950/60 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-emerald-500"
            value={cacheUrl}
            onChange={(e) => setCacheUrl(e.target.value)}
          />
          <p className="text-[11px] text-slate-500 mt-2">Default: http://localhost:8081</p>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5">
          <p className="text-xs font-medium text-slate-400 mb-1">Tracer</p>
          <p className="text-sm text-slate-300 mb-3">Distributed traces and audit-grade spans.</p>
          <input
            className="w-full rounded-md bg-slate-950/60 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-emerald-500"
            value={tracerUrl}
            onChange={(e) => setTracerUrl(e.target.value)}
          />
          <p className="text-[11px] text-slate-500 mt-2">Default: http://localhost:8082</p>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5">
          <p className="text-xs font-medium text-slate-400 mb-1">Compliance</p>
          <p className="text-sm text-slate-300 mb-3">PII redaction and residency enforcement.</p>
          <input
            className="w-full rounded-md bg-slate-950/60 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-emerald-500"
            value={complianceUrl}
            onChange={(e) => setComplianceUrl(e.target.value)}
          />
          <p className="text-[11px] text-slate-500 mt-2">Default: http://localhost:8083</p>
        </div>
      </div>

      <div className="flex items-center justify-end">
        <button
          type="button"
          className="inline-flex items-center gap-2 rounded-md bg-slate-100/5 border border-slate-700 px-4 py-2 text-xs font-semibold text-slate-200 hover:bg-slate-100/10 transition-colors cursor-default"
        >
          <SettingsIcon className="w-3 h-3" />
          <span>Settings are local to this session</span>
        </button>
      </div>
    </div>
  );

  const renderActiveView = () => {
    switch (activeView) {
      case 'compliance':
        return renderCompliance();
      case 'traces':
        return renderTraces();
      case 'cache':
        return renderCache();
      case 'settings':
        return renderSettings();
      case 'dashboard':
      default:
        return renderDashboard();
    }
  };

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 flex">
      {/* Sidebar */}
      <aside className="hidden md:flex md:w-64 lg:w-72 flex-col border-r border-slate-800/80 bg-slate-900/80 backdrop-blur-xl">
        <div className="px-5 pt-5 pb-4 border-b border-slate-800/80">
          <div className="flex items-center gap-2">
            <div className="h-8 w-8 rounded-xl bg-gradient-to-tr from-red-500 to-emerald-500 flex items-center justify-center shadow-[0_0_24px_rgba(248,113,113,0.45)]">
              <span className="text-xs font-bold tracking-tight">RE</span>
            </div>
            <div>
              <p className="text-xs uppercase tracking-[0.2em] text-slate-500">RedEye</p>
              <p className="text-sm font-semibold text-slate-50">Control Plane</p>
            </div>
          </div>
        </div>

        <nav className="flex-1 px-3 py-4 space-y-1 overflow-y-auto custom-scrollbar">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            const isActive = activeView === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setActiveView(item.id)}
                className={`w-full flex items-center justify-between px-3 py-2 rounded-lg text-left text-sm transition-colors border ${
                  isActive
                    ? 'bg-slate-800/70 text-emerald-400 border-emerald-500/40 shadow-[0_0_24px_rgba(16,185,129,0.25)]'
                    : 'bg-transparent text-slate-400 border-transparent hover:bg-slate-800/40 hover:text-slate-100'
                }`}
                aria-pressed={isActive}
              >
                <span className="flex items-center gap-2">
                  <Icon className="w-4 h-4" />
                  <span>{item.label}</span>
                </span>
                <span className="hidden xl:inline text-[11px] text-slate-500">{item.description}</span>
              </button>
            );
          })}
        </nav>

        <div className="px-4 pb-4 pt-2 border-t border-slate-800/80 text-[11px] text-slate-500">
          <p className="flex items-center justify-between">
            <span>Cluster</span>
            <span className="font-mono text-emerald-400">local-dev</span>
          </p>
          <p className="mt-1 text-slate-600">Gateway @8080 · Cache @8081 · Tracer @8082 · Compliance @8083</p>
        </div>
      </aside>

      {/* Main content */}
      <div className="flex-1 flex flex-col">
        {/* Mobile top bar */}
        <header className="md:hidden flex items-center justify-between px-4 py-3 border-b border-slate-800 bg-slate-900/90 backdrop-blur-xl">
          <div className="flex items-center gap-2">
            <div className="h-7 w-7 rounded-lg bg-gradient-to-tr from-red-500 to-emerald-500 flex items-center justify-center">
              <span className="text-[10px] font-bold tracking-tight">RE</span>
            </div>
            <div>
              <p className="text-[10px] uppercase tracking-[0.2em] text-slate-500">RedEye</p>
              <p className="text-xs font-semibold text-slate-50">Control Plane</p>
            </div>
          </div>
          <span className="text-[11px] text-slate-500">{NAV_ITEMS.find((n) => n.id === activeView)?.label}</span>
        </header>

        <main className="flex-1 overflow-y-auto custom-scrollbar">
          <div className="max-w-7xl mx-auto w-full px-4 sm:px-6 lg:px-8 py-6 lg:py-8">
            {renderActiveView()}
          </div>
        </main>
      </div>
    </div>
  );
}

export default App;