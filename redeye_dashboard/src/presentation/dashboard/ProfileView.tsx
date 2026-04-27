import React, { useState, useMemo } from 'react';
import { 
  Radar, 
  RadarChart, 
  PolarGrid, 
  PolarAngleAxis, 
  PolarRadiusAxis, 
  ResponsiveContainer 
} from 'recharts';
import { motion, AnimatePresence } from 'framer-motion';
import { LogOut, Shield, Zap, Terminal, Users, ChevronRight, Building, Mail, Calendar } from 'lucide-react';
import { useAuth } from '../context/AuthContext';
import { BentoCard } from '../components/ui/BentoCard';
import { Badge } from '../components/ui/Badge';

import useSWR from 'swr';
import { fetchUsageMetrics, USAGE_METRICS_URL, type UsageMetrics } from '../../data/services/metricsService';
import { fetchComplianceMetrics, COMPLIANCE_METRICS_URL, type ComplianceMetrics } from '../../data/services/metricsService';

const fetcher = async (url: string) => {
  const res = await fetch(url, {
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' },
  });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

interface Metrics {
  total_requests: string;
  avg_latency_ms: number;
  total_tokens: string;
  rate_limited_requests: string;
}

// ── Components ────────────────────────────────────────────────────────────────

export function ProfileView() {
  const { user, logout } = useAuth();
  
  const [activeContext, setActiveContext] = useState<'individual' | 'team'>(user?.accountType || 'individual');

  const { data: metrics } = useSWR<Metrics>('http://localhost:8080/v1/admin/metrics', fetcher, { refreshInterval: 5000 });
  const { data: usage } = useSWR<UsageMetrics>(USAGE_METRICS_URL, fetchUsageMetrics, { refreshInterval: 10000 });
  const { data: compliance } = useSWR<ComplianceMetrics>(COMPLIANCE_METRICS_URL, fetchComplianceMetrics, { refreshInterval: 10000 });

  const isTeam = activeContext === 'team';

  // Derive Radar Data from Live Telemetry
  const radarData = useMemo(() => {
    const volume = metrics ? Math.min(100, (parseInt(metrics.total_requests) / (isTeam ? 100000 : 10000)) * 100) : 0;
    const speed = metrics ? Math.max(0, 100 - (metrics.avg_latency_ms / 20)) : 0; // Higher is better
    const cost = usage ? Math.min(100, (usage.estimated_cost / (isTeam ? 500 : 50)) * 100) : 0;
    const reasoning = compliance ? Math.min(100, (compliance.pii_redactions / (compliance.total_scanned || 1)) * 500) : 70;

    return [
      { subject: 'SPEED',     value: speed || 80, fullMark: 100 },
      { subject: 'REASONING', value: reasoning || 85, fullMark: 100 },
      { subject: 'COST',      value: cost || 60, fullMark: 100 },
      { subject: 'VOLUME',    value: volume || 75, fullMark: 100 },
    ];
  }, [metrics, usage, compliance, isTeam]);

  const quotaSegments = Array.from({ length: 24 });
  const capValue = isTeam ? 250000 : 25000;
  const currentReqs = metrics ? parseInt(metrics.total_requests) : 0;
  const filledSegments = Math.min(24, Math.floor((currentReqs / capValue) * 24));
  
  const totalCap = isTeam ? '250K' : '25K';
  const currentUsageStr = metrics ? (parseInt(metrics.total_requests) >= 1000 ? `${(parseInt(metrics.total_requests) / 1000).toFixed(1)}K` : metrics.total_requests) : '0';

  const memberList = isTeam ? [{ name: user?.workspaceName || 'Owner', role: 'Admin', avatar: user?.email?.[0].toUpperCase() || 'O' }] : [];

  return (
    <div className="space-y-8 max-w-5xl mx-auto pb-12">
      
      {/* ── Context Switcher & Breadcrumbs ──────────────────────── */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <nav className="flex items-center gap-2 text-[10px] font-jetbrains font-bold uppercase tracking-[0.2em] text-[var(--on-surface-muted)]">
          <span className="opacity-40">Accounts</span>
          <ChevronRight className="w-3 h-3 opacity-20" />
          <span className="text-[var(--accent-cyan)]">
            {isTeam ? (user?.workspaceName || 'RedEye Corp') : 'Personal'}
          </span>
        </nav>

        <div className="flex bg-[var(--surface-container-low)] p-1 rounded-xl relative">
          <button
            onClick={() => setActiveContext('individual')}
            className={`relative px-4 py-1.5 text-[10px] font-bold tracking-widest uppercase transition-colors z-10 font-geist ${
              !isTeam ? 'text-[var(--on-surface)]' : 'text-[var(--on-surface-muted)] hover:text-[var(--on-surface)]'
            }`}
          >
            {!isTeam && (
              <motion.div
                layoutId="context-pill"
                className="absolute inset-0 bg-[var(--surface-bright)] rounded-lg shadow-xl"
                transition={{ type: 'spring', bounce: 0.2, duration: 0.6 }}
              />
            )}
            <span className="relative z-10">Individual</span>
          </button>
          <button
            onClick={() => setActiveContext('team')}
            className={`relative px-4 py-1.5 text-[10px] font-bold tracking-widest uppercase transition-colors z-10 font-geist ${
              isTeam ? 'text-[var(--on-surface)]' : 'text-[var(--on-surface-muted)] hover:text-[var(--on-surface)]'
            }`}
          >
            {isTeam && (
              <motion.div
                layoutId="context-pill"
                className="absolute inset-0 bg-[var(--surface-bright)] rounded-lg shadow-xl"
                transition={{ type: 'spring', bounce: 0.2, duration: 0.6 }}
              />
            )}
            <span className="relative z-10">Organization</span>
          </button>
        </div>
      </div>

      <AnimatePresence mode="wait">
        <motion.div
          key={activeContext}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -10 }}
          transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
          className="space-y-8"
        >
          {/* ── Identity Module ───────────────────────────────────── */}
          <BentoCard className="p-8 relative overflow-hidden" glowColor={isTeam ? 'amber' : 'cyan'}>
            <div className={`absolute top-0 right-0 w-80 h-80 ${isTeam ? 'bg-[var(--primary-amber)]' : 'bg-[var(--accent-cyan)]'} opacity-[0.04] blur-[120px] -mr-40 -mt-40 rounded-full pointer-events-none`} />
            
            <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-8 relative z-10">
              <div className="flex items-center gap-6">
                <div className={`w-20 h-20 rounded-[1.25rem] bg-[var(--surface-bright)] flex items-center justify-center text-3xl font-black font-jetbrains ${isTeam ? 'text-[var(--primary-amber)]' : 'text-[var(--accent-cyan)]'} shadow-2xl`}>
                  {isTeam ? <Building className="w-10 h-10" /> : (user?.email?.[0].toUpperCase() || 'D')}
                </div>
                <div className="space-y-2">
                  <div className="flex items-center gap-3">
                    <h1 className="text-3xl font-geist font-black tracking-tighter text-[var(--on-surface)] uppercase">
                      {isTeam ? (user?.workspaceName || 'RedEye Engineering') : (user?.workspaceName || 'Developer Node')}
                    </h1>
                    <Badge variant={isTeam ? 'neutral' : 'success'} className="font-jetbrains">
                      {isTeam ? 'ORG_LEVEL_5' : 'ROOT_ACCESS'}
                    </Badge>
                  </div>
                  <div className="flex flex-wrap items-center gap-4 text-[var(--on-surface-muted)]">
                    <div className="flex items-center gap-1.5">
                      <Mail className="w-3.5 h-3.5" />
                      <span className="text-xs font-jetbrains lowercase">{isTeam ? `ops@${user?.workspaceName?.toLowerCase().replace(' ', '') || 'redeye'}.corp` : user?.email}</span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      <Calendar className="w-3.5 h-3.5" />
                      <span className="text-xs font-jetbrains">Est. April 2024</span>
                    </div>
                  </div>
                </div>
              </div>

              {isTeam && (
                <div className="flex flex-col items-end gap-3">
                  <span className="text-[10px] font-jetbrains text-[var(--on-surface-muted)] uppercase tracking-[0.3em] font-bold opacity-60">
                    Team Composition
                  </span>
                  <div className="flex -space-x-3">
                    {memberList.map((m, i) => (
                      <div 
                        key={i}
                        className="w-10 h-10 rounded-full border-2 border-[var(--surface)] bg-[var(--surface-bright)] flex items-center justify-center text-[10px] font-bold font-jetbrains text-[var(--on-surface)] shadow-lg"
                        title={`${m.name} (${m.role})`}
                      >
                        {m.avatar}
                      </div>
                    ))}
                    <div className="w-10 h-10 rounded-full border-2 border-[var(--surface)] bg-[var(--accent-cyan)]/20 flex items-center justify-center text-[10px] font-bold font-jetbrains text-[var(--accent-cyan)] shadow-lg cursor-pointer hover:bg-[var(--accent-cyan)]/30 transition-colors">
                      +0
                    </div>
                  </div>
                </div>
              )}
            </div>
          </BentoCard>

          {/* ── Member List (Team Only) ───────────────────────────── */}
          {isTeam && (
            <div className="flex items-center gap-4 overflow-x-auto pb-2 custom-scrollbar">
              {memberList.map((member, i) => (
                <motion.div
                  key={i}
                  initial={{ opacity: 0, x: 20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: i * 0.1 }}
                >
                  <BentoCard className="flex-shrink-0 px-4 py-3 min-w-[180px]" glowColor="none">
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-lg bg-[var(--surface-bright)] flex items-center justify-center text-[10px] font-bold font-jetbrains">
                        {member.avatar}
                      </div>
                      <div>
                        <p className="text-[11px] font-geist font-bold text-[var(--on-surface)] truncate w-24">
                          {member.name}
                        </p>
                        <p className="text-[9px] font-jetbrains text-[var(--on-surface-muted)] uppercase tracking-tighter">
                          {member.role}
                        </p>
                      </div>
                    </div>
                  </BentoCard>
                </motion.div>
              ))}
            </div>
          )}

          {/* ── Metrics Hub ───────────────────────────────────────── */}
          <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
            {/* Radar Analysis */}
            <BentoCard className="lg:col-span-7 p-8 h-[450px]" glowColor="none">
              <div className="flex items-center justify-between mb-8">
                <div className="flex items-center gap-3">
                  <div className={`p-2 rounded-lg ${isTeam ? 'bg-[var(--primary-amber)]/10 text-[var(--primary-amber)]' : 'bg-[var(--accent-cyan)]/10 text-[var(--accent-cyan)]'}`}>
                    <Zap className="w-5 h-5" />
                  </div>
                  <div>
                    <h3 className="text-sm font-geist font-black uppercase tracking-[0.2em] text-[var(--on-surface)]">
                      {isTeam ? 'Aggregated Capabilities' : 'Individual Performance'}
                    </h3>
                    <p className="text-[10px] font-jetbrains text-[var(--on-surface-muted)] uppercase tracking-widest">
                      Session Telemetry: 0X_DELTA_PROTOCOL
                    </p>
                  </div>
                </div>
                <div className="text-right">
                  <p className="text-[9px] font-jetbrains text-[var(--on-surface-muted)] uppercase font-bold">Health Score</p>
                  <p className={`text-xl font-jetbrains font-bold ${isTeam ? 'text-[var(--primary-amber)]' : 'text-[var(--accent-cyan)]'}`}>
                    {isTeam ? '91.4' : '84.2'}
                  </p>
                </div>
              </div>

              <div className="flex-1 w-full h-full min-h-0">
                <ResponsiveContainer width="100%" height="100%">
                  <RadarChart cx="50%" cy="50%" outerRadius="85%" data={radarData}>
                    <PolarGrid stroke="var(--on-surface-muted)" strokeOpacity={0.1} />
                    <PolarAngleAxis 
                      dataKey="subject" 
                      tick={{ 
                        fill: 'var(--on-surface-muted)', 
                        fontSize: 9, 
                        fontWeight: 700,
                        fontFamily: 'JetBrains Mono',
                        letterSpacing: '0.15em'
                      }}
                    />
                    <PolarRadiusAxis domain={[0, 100]} tick={false} axisLine={false} />
                    <Radar
                      name="Active Context"
                      dataKey="value"
                      stroke={isTeam ? 'var(--primary-amber)' : 'var(--accent-cyan)'}
                      fill={isTeam ? 'var(--primary-amber)' : 'var(--accent-cyan)'}
                      fillOpacity={0.1}
                      strokeWidth={2}
                    />
                  </RadarChart>
                </ResponsiveContainer>
              </div>
            </BentoCard>

            {/* Quota Protocol */}
            <BentoCard className="lg:col-span-5 p-8 flex flex-col justify-between" glowColor="none">
              <div className="space-y-8">
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-lg bg-[var(--primary-rose)]/10 text-[var(--primary-rose)]">
                    <Shield className="w-5 h-5" />
                  </div>
                  <div>
                    <h3 className="text-sm font-geist font-black uppercase tracking-[0.2em] text-[var(--on-surface)]">
                      {isTeam ? 'Organization Quota' : 'Account Quota'}
                    </h3>
                    <p className="text-[10px] font-jetbrains text-[var(--on-surface-muted)] uppercase tracking-widest">
                      Real-time throughput
                    </p>
                  </div>
                </div>

                <div className="space-y-4">
                  <div className="flex items-end justify-between">
                    <span className="text-4xl font-jetbrains font-black tabular-nums text-[var(--on-surface)] tracking-tighter">
                      {currentUsageStr}<span className="text-xl opacity-40 ml-1">REQS</span>
                    </span>
                    <div className="text-right">
                      <p className="text-[9px] font-jetbrains text-[var(--on-surface-muted)] uppercase font-bold">Allocation Limit</p>
                      <p className="text-xs font-jetbrains font-bold text-[var(--on-surface)]">{totalCap}</p>
                    </div>
                  </div>

                  <div className="flex gap-1.5 h-12">
                    {quotaSegments.map((_, i) => {
                      const isFilled = i < filledSegments;
                      const isWarning = i > 18;
                      
                      return (
                        <div
                          key={i}
                          className="flex-1 rounded-[4px] transition-all duration-700"
                          style={{
                            backgroundColor: isFilled 
                              ? (isWarning ? 'var(--primary-rose)' : (isTeam ? 'var(--primary-amber)' : 'var(--accent-cyan)'))
                              : 'var(--surface-bright)',
                            opacity: isFilled ? 1 : 0.1,
                            boxShadow: isFilled ? `0 0 16px ${isWarning ? 'var(--primary-rose)' : (isTeam ? 'var(--primary-amber)' : 'var(--accent-cyan)')}33` : 'none'
                          }}
                        />
                      );
                    })}
                  </div>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div className="p-5 rounded-[1rem] bg-[var(--surface-container-low)] space-y-2">
                    <p className="text-[9px] font-jetbrains text-[var(--on-surface-muted)] uppercase tracking-widest font-bold">Shared Nodes</p>
                    <p className="font-jetbrains text-xl font-bold text-[var(--accent-cyan)]">{isTeam ? '12 Active' : '1 Active'}</p>
                  </div>
                  <div className="p-5 rounded-[1rem] bg-[var(--surface-container-low)] space-y-2">
                    <p className="text-[9px] font-jetbrains text-[var(--on-surface-muted)] uppercase tracking-widest font-bold">Uptime Protocol</p>
                    <p className="font-jetbrains text-xl font-bold text-[var(--primary-rose)]">99.98%</p>
                  </div>
                </div>
              </div>

              <div className="pt-8 border-t border-[var(--surface-bright)]/10 mt-8">
                <div className="flex items-center gap-2 mb-2">
                  <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
                  <span className="text-[10px] font-jetbrains text-[var(--on-surface-muted)] uppercase font-bold tracking-widest">Gateway Status: Syncing</span>
                </div>
                <p className="text-[9px] font-jetbrains text-[var(--on-surface-muted)] opacity-50 font-medium">
                  {isTeam 
                    ? 'Org-wide telemetry is aggregated across 4 globally distributed clusters.' 
                    : 'Personal telemetry localized to dev-instance-01.'}
                </p>
              </div>
            </BentoCard>
          </div>
        </motion.div>
      </AnimatePresence>

      {/* ── Security Action (Bottom) ────────────────────────────── */}
      <div className="flex flex-col items-center gap-4 pt-12">
        <div className="h-px w-24 bg-gradient-to-r from-transparent via-[var(--surface-bright)] to-transparent" />
        <button
          onClick={logout}
          className="flex items-center gap-3 px-10 py-3 rounded-xl text-[var(--primary-rose)] font-geist font-black text-xs tracking-[0.2em] uppercase transition-all duration-300 hover:bg-[rgba(244,63,94,0.1)] active:scale-95 group"
        >
          <LogOut className="w-4 h-4 group-hover:-translate-x-1 transition-transform" />
          Terminate Session
        </button>
      </div>
    </div>
  );
}
