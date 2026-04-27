import { ShieldAlert, Flame, DollarSign, Loader2, AlertTriangle, Shield, ArrowRight } from 'lucide-react';
import useSWR from 'swr';
import { motion, AnimatePresence } from 'framer-motion';
import { Link } from 'react-router-dom';
import {
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend, Cell
} from 'recharts';
import { BentoCard } from '../components/ui/BentoCard';

// ── Types ────────────────────────────────────────────────────────────────────

interface Alert {
  timestamp: string;
  session_id: string;
  reason: string;
  severity: 'Critical' | 'High' | 'Medium';
}

interface SecurityData {
  total_blocked_loops: number;
  total_burn_rate_blocks: number;
  estimated_savings_usd: number;
  daily_blocks: { date: string; loops: number; burn_rate: number }[];
  recent_alerts: Alert[];
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const fetcher = async (url: string) => {
  const res = await fetch(url, { 
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' }
  });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

const LABEL_CLASS = 'font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[10px] font-bold';
const DATA_CLASS  = 'font-jetbrains text-[var(--on-surface)]';

function formatTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
}

// ── Framer Motion Variants ──────────────────────────────────────────────────

const containerVariants = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.1 }
  }
};

const itemVariants = {
  hidden: { opacity: 0, y: 20 },
  show: { opacity: 1, y: 0, transition: { duration: 0.5, ease: [0.16, 1, 0.3, 1] } }
};

// ── Component ────────────────────────────────────────────────────────────────

export function SecurityView() {
  const { data, error, isLoading } = useSWR<SecurityData>(
    'http://localhost:8080/v1/admin/security/alerts',
    fetcher,
    {
      refreshInterval: 5000,
      errorRetryCount: 3,
    },
  );

  return (
    <motion.div
      variants={containerVariants}
      initial="hidden"
      animate="show"
      className="grid grid-cols-12 gap-6 p-6 auto-rows-max text-[var(--on-surface)]"
    >
      {/* Breadcrumb */}
      <motion.div variants={itemVariants} className="col-span-12 flex items-center gap-3 text-sm font-mono text-[var(--text-muted)] mb-2">
        <Link to="/dashboard" className="hover:text-[var(--on-surface)] transition-colors flex items-center gap-2 font-geist tracking-wide">
          <Shield className="w-4 h-4" />
          Dashboard
        </Link>
        <ArrowRight className="w-4 h-4" />
        <span className="text-[var(--on-surface)] font-geist">Security</span>
      </motion.div>

      {/* Header */}
      <motion.header variants={itemVariants} className="col-span-12 flex flex-col md:flex-row md:items-end justify-between gap-6 mb-8">
        <div>
          <p className={`${LABEL_CLASS} text-[var(--primary-rose)] mb-1`}>Threat Detection Center</p>
          <h1 className="text-4xl font-extrabold tracking-tight text-[var(--on-surface)] mb-2 font-geist">
            Security Intelligence
          </h1>
          <p className="text-sm text-[var(--text-muted)] max-w-2xl font-geist">
            Real-time monitoring of AI agent loops, cost abuse, and runaway spend prevention.
          </p>
        </div>
        
        <div className="flex items-center gap-3 p-1 rounded-full bg-[rgba(255,255,255,0.02)]">
          <div className="flex items-center gap-3 px-4 py-2 rounded-full bg-[var(--surface-bright)] shadow-md">
            {isLoading && !data ? (
              <Loader2 className="w-4 h-4 text-[var(--accent-cyan)] animate-spin" />
            ) : (
              <div className={`w-2 h-2 rounded-full ${error ? 'bg-[var(--primary-rose)]' : 'bg-[var(--accent-cyan)] shadow-[0_0_10px_var(--accent-cyan)]'} animate-pulse`} />
            )}
            <span className={`${LABEL_CLASS} normal-case tracking-normal`}>
              {isLoading && !data ? 'Syncing...' : error ? 'Link Failure' : 'Active Sentinel'}
            </span>
          </div>
        </div>
      </motion.header>

      {/* Stat Cards */}
      <motion.div variants={itemVariants} className="col-span-12 sm:col-span-6 lg:col-span-4 h-[160px]">
        <BentoCard glowColor="rose" className="h-full p-6 flex flex-col justify-between">
          <div className="flex items-center gap-3 text-[var(--primary-rose)]">
            <ShieldAlert className="w-5 h-5" />
            <h3 className={LABEL_CLASS}>Agent Loops Blocked</h3>
          </div>
          <div>
            <p className={`${DATA_CLASS} text-3xl font-bold tracking-tighter`}>
              {isLoading && !data ? '—' : data?.total_blocked_loops ?? 0}
            </p>
            <p className="text-[10px] text-[var(--text-muted)] uppercase tracking-widest font-bold mt-1">Circuit-breaker hits</p>
          </div>
        </BentoCard>
      </motion.div>

      <motion.div variants={itemVariants} className="col-span-12 sm:col-span-6 lg:col-span-4 h-[160px]">
        <BentoCard glowColor="amber" className="h-full p-6 flex flex-col justify-between">
          <div className="flex items-center gap-3 text-[var(--primary-amber)]">
            <Flame className="w-5 h-5" />
            <h3 className={LABEL_CLASS}>Runaway Spenders</h3>
          </div>
          <div>
            <p className={`${DATA_CLASS} text-3xl font-bold tracking-tighter`}>
              {isLoading && !data ? '—' : data?.total_burn_rate_blocks ?? 0}
            </p>
            <p className="text-[10px] text-[var(--text-muted)] uppercase tracking-widest font-bold mt-1">Velocity limit triggers</p>
          </div>
        </BentoCard>
      </motion.div>

      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4 h-[160px]">
        <BentoCard glowColor="cyan" className="h-full p-6 flex flex-col justify-between">
          <div className="flex items-center gap-3 text-[var(--accent-cyan)]">
            <DollarSign className="w-5 h-5" />
            <h3 className={LABEL_CLASS}>Estimated Savings</h3>
          </div>
          <div>
            <p className={`${DATA_CLASS} text-3xl font-bold tracking-tighter`}>
              <span className="text-[var(--text-muted)] font-normal mr-1">$</span>
              {isLoading && !data ? '—' : (data?.estimated_savings_usd ?? 0).toFixed(2)}
            </p>
            <p className="text-[10px] text-[var(--text-muted)] uppercase tracking-widest font-bold mt-1">Abuse prevention yield</p>
          </div>
        </BentoCard>
      </motion.div>

      {/* Error banner */}
      {error && !data && (
        <motion.div variants={itemVariants} className="col-span-12 p-4 rounded-2xl bg-[rgba(244,63,94,0.1)] flex items-center gap-3">
          <AlertCircle className="w-5 h-5 text-[var(--primary-rose)] flex-shrink-0" />
          <p className="text-sm text-[var(--on-surface)] font-geist">Connection to security endpoint failed. Showing cached data.</p>
        </motion.div>
      )}

      {/* Bar Chart — Blocked Threats */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-5 h-[400px]">
        <BentoCard glowColor="rose" className="h-full p-6 flex flex-col">
          <div className="flex items-center justify-between mb-6">
            <h3 className={LABEL_CLASS}>Blocked Threats / Day</h3>
            <span className="text-[10px] font-jetbrains text-[var(--primary-amber)] bg-[rgba(251,191,36,0.1)] px-2 py-1 rounded uppercase tracking-tighter">7-day window</span>
          </div>
          <div className="flex-1 w-full min-h-0">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={data?.daily_blocks ?? []} barGap={4}>
                <CartesianGrid strokeDasharray="3 3" stroke="var(--surface-bright)" strokeOpacity={0.1} vertical={false} />
                <XAxis dataKey="date" stroke="var(--on-surface-muted)" fontSize={10} tickLine={false} axisLine={false} />
                <YAxis stroke="var(--on-surface-muted)" fontSize={10} tickLine={false} axisLine={false} />
                <Tooltip
                  cursor={{ fill: 'var(--surface-bright)', opacity: 0.1 }}
                  contentStyle={{ backgroundColor: 'var(--surface-container-low)', borderColor: 'transparent', borderRadius: '12px', boxShadow: '0 8px 32px rgba(0,0,0,0.5)', color: 'var(--on-surface)', fontSize: '12px' }}
                />
                <Legend iconType="circle" wrapperStyle={{ fontSize: '10px', paddingTop: '12px', fontFamily: 'Geist' }} />
                <Bar dataKey="loops" name="Loop Blocks" fill="var(--primary-rose)" radius={[4, 4, 0, 0]} animationDuration={1500} />
                <Bar dataKey="burn_rate" name="Burn Rate Blocks" fill="var(--primary-amber)" radius={[4, 4, 0, 0]} animationDuration={1500} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </BentoCard>
      </motion.div>

      {/* Live Threat Feed */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-7 h-[400px]">
        <BentoCard glowColor="rose" className="h-full overflow-hidden flex flex-col">
          <div className="p-6 bg-[rgba(255,255,255,0.02)] flex items-center justify-between">
            <h3 className={LABEL_CLASS}>Live Threat Feed</h3>
            <span className="text-[9px] text-[var(--primary-rose)] font-bold tracking-[0.2em] uppercase animate-pulse">Live Intercept</span>
          </div>
          <div className="flex-1 overflow-x-auto overflow-y-auto custom-scrollbar p-2">
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="sticky top-0 bg-[var(--surface-lowest)] z-10">
                  <th className={`${LABEL_CLASS} py-3 px-4`}>Time</th>
                  <th className={`${LABEL_CLASS} py-3 px-4`}>Session</th>
                  <th className={`${LABEL_CLASS} py-3 px-4`}>Threat Reason</th>
                  <th className={`${LABEL_CLASS} py-3 px-4`}>Severity</th>
                </tr>
              </thead>
              <tbody>
                {(data?.recent_alerts ?? []).map((alert, i) => (
                  <tr
                    key={`${alert.session_id}-${i}`}
                    className={`group transition-colors ${i % 2 === 0 ? 'bg-[var(--surface-container-low)]' : 'bg-[var(--surface-container-lowest)]'} hover:bg-[var(--surface-bright)]`}
                  >
                    <td className="py-3 px-4 text-[0.75rem] font-jetbrains tabular-nums text-[var(--text-muted)]">{formatTime(alert.timestamp)}</td>
                    <td className="py-3 px-4 text-[0.75rem] font-jetbrains text-[var(--accent-cyan)] truncate max-w-[120px]" title={alert.session_id}>
                      {alert.session_id.split('-')[0]}
                    </td>
                    <td className="py-3 px-4 text-xs font-geist text-[var(--on-surface)]">{alert.reason}</td>
                    <td className="py-3 px-4">
                      <span
                        className={`inline-flex items-center px-2.5 py-1 rounded-full text-[9px] font-bold uppercase tracking-widest ${
                          alert.severity === 'Critical' ? 'bg-[rgba(244,63,94,0.1)] text-[var(--primary-rose)]' :
                          alert.severity === 'High' ? 'bg-[rgba(251,191,36,0.1)] text-[var(--primary-amber)]' :
                          'bg-[rgba(34,211,238,0.1)] text-[var(--accent-cyan)]'
                        }`}
                      >
                        {alert.severity}
                      </span>
                    </td>
                  </tr>
                ))}
                {(!data || data.recent_alerts.length === 0) && (
                  <tr>
                    <td colSpan={4} className="py-24 text-center">
                      <div className="flex flex-col items-center justify-center opacity-40">
                         <Shield className="w-8 h-8 mb-4 text-[var(--text-muted)]" />
                         <span className={`${LABEL_CLASS}`}>No active threats detected</span>
                      </div>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </BentoCard>
      </motion.div>
    </motion.div>
  );
}
