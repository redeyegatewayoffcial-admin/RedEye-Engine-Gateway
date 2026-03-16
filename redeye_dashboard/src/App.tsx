import { useEffect, useState } from 'react';
import { Activity, Zap, ShieldAlert, Cpu, DollarSign } from 'lucide-react';
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

  return (
    // Dynamic Padding & hidden X-overflow to prevent page-level horizontal scrolling issues
    <div className="min-h-screen flex flex-col p-4 sm:p-6 lg:p-8 overflow-x-hidden">
      
      {/* Responsive Header: Stacks on mobile, Row on Desktop */}
      <header className="mb-6 sm:mb-8 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-red-500 to-orange-500 bg-clip-text text-transparent pb-1">
            RedEye Gateway
          </h1>
          <p className="text-xs sm:text-sm text-red-400/60 mt-1">Enterprise Telemetry & Security Command Center</p>
        </div>
        <div className="flex items-center space-x-2 glass-panel px-3 py-1.5 sm:px-4 sm:py-2 rounded-full self-start sm:self-auto w-fit">
          <div className={`w-2 h-2 sm:w-3 sm:h-3 rounded-full ${error ? 'bg-red-500' : 'bg-red-500 neon-dot'}`} />
          <span className="text-xs sm:text-sm font-medium text-slate-300">
             {error ? 'System Offline' : 'Live Sync Active'}
          </span>
        </div>
      </header>

      <main className="flex-1 max-w-7xl mx-auto w-full space-y-4 sm:space-y-6">
        
        {/* Dynamic Grid: 1 col (mobile) -> 2 col (tablet) -> 3 col (laptop) -> 5 col (desktop) */}
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
          <div className="glass-panel p-4 sm:p-6 lg:col-span-2">
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

          <div className="glass-panel p-4 sm:p-6 lg:col-span-1">
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
                  <Legend verticalAlign="bottom" height={36} wrapperStyle={{ fontSize: '11px', color: '#94a3b8' }}/>
                </PieChart>
              </ResponsiveContainer>
            </div>
          </div>
        </div>

        {/* Charts Row 2 */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
          <div className="glass-panel p-4 sm:p-6 lg:col-span-1">
            <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Latency Histogram</h2>
            <div className="h-[200px] sm:h-[250px] w-full">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={mockLatencyData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(239,68,68,0.08)" vertical={false} />
                  <XAxis dataKey="bucket" stroke="#94a3b8" fontSize={10} tickMargin={8} />
                  <YAxis stroke="#94a3b8" fontSize={10} width={30} />
                  <Tooltip cursor={{fill: 'rgba(239,68,68,0.06)'}} contentStyle={tooltipStyle} />
                  <Bar dataKey="count" fill="#ef4444" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* TABLE - Horizontal Scroll Magic for Mobile */}
          <div className="glass-panel p-4 sm:p-6 lg:col-span-2 flex flex-col overflow-hidden">
            <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Live Request Audit Log</h2>
            
            {/* This wrapper enables horizontal swipe on mobile without breaking the page */}
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
                        <span className={`px-2 py-1 rounded text-[10px] sm:text-xs font-semibold ${log.status === 200 ? 'bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/20' : 'bg-red-500/10 text-red-400 ring-1 ring-red-500/20'}`}>
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

      </main>
    </div>
  );
}

export default App;