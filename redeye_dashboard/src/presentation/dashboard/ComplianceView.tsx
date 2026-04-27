import React from 'react';
import useSWR from 'swr';
import { motion } from 'framer-motion';
import { Treemap, ResponsiveContainer, Tooltip as RechartsTooltip } from 'recharts';
import { Eye, AlertTriangle, TrendingUp, Shield, ArrowRight } from 'lucide-react';
import { Link } from 'react-router-dom';
import { BentoCard } from '../components/ui/BentoCard';
import { AreaChartGradient } from '../components/ui/AreaChartGradient';
import { ProportionalArcDonut } from '../components/ui/ProportionalArcDonut';
import { Badge } from '../components/ui/Badge';
import { fetchComplianceMetrics, COMPLIANCE_METRICS_URL, type ComplianceMetrics } from '../../data/services/metricsService';

// ── Styles ────────────────────────────────────────────────────────────────────

const LABEL_CLASS = 'font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[10px] font-bold';
const DATA_CLASS  = 'font-jetbrains text-[var(--on-surface)]';

// ── Framer Motion Variants ──────────────────────────────────────────────────
const containerVariants = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: {
      staggerChildren: 0.1,
    },
  },
};

const itemVariants = {
  hidden: { opacity: 0, y: 20 },
  show: { 
    opacity: 1, 
    y: 0, 
    transition: { duration: 0.5, ease: [0.16, 1, 0.3, 1] } 
  },
};

// ── Custom Treemap Content ──────────────────────────────────────────────────
const CustomizedContent = (props: any) => {
  const { x, y, width, height, name, fill } = props;
  return (
    <g>
      <rect
        x={x}
        y={y}
        width={width}
        height={height}
        style={{
          fill: fill,
          stroke: 'var(--bg-canvas)',
          strokeWidth: 2,
        }}
      />
      {width > 40 && height > 30 && (
        <text
          x={x + width / 2}
          y={y + height / 2}
          textAnchor="middle"
          fill="#ffffff"
          fontSize={10}
          fontFamily="monospace"
          className="font-bold drop-shadow-md uppercase tracking-wider"
        >
          {name}
        </text>
      )}
    </g>
  );
};

// ── Component ───────────────────────────────────────────────────────────────
export function ComplianceView() {
  const { data: metrics, isLoading } = useSWR<ComplianceMetrics>(
    COMPLIANCE_METRICS_URL,
    fetchComplianceMetrics,
    { refreshInterval: 5000 }
  );

  const redactionCoverage = metrics && metrics.total_scanned > 0 
    ? ((metrics.pii_redactions / metrics.total_scanned) * 100) 
    : 100;
  
  // Map real region breakdown to treemap
  const treemapData = metrics?.region_breakdown.map((r) => {
    let fill = 'var(--accent-cyan)'; // Info
    if (r.count > 5000) fill = 'var(--primary-rose)'; // Critical
    else if (r.count > 1000) fill = 'var(--primary-amber)'; // Warn
    
    return { name: r.region, size: r.count, fill };
  }) || [];

  const violations: any[] = []; // Empty for real endpoints
  const piiTimeline: any[] = [];

  return (
    <motion.div
      variants={containerVariants}
      initial="hidden"
      animate="show"
      className="grid grid-cols-12 gap-6 p-6 auto-rows-max text-[var(--on-surface)]"
    >
      {/* ── BREADCRUMB ────────────────────────────────────────────────── */}
      <motion.div variants={itemVariants} className="col-span-12 flex items-center gap-3 text-sm font-mono text-[var(--text-muted)] mb-2">
        <Link to="/dashboard" className="hover:text-[var(--on-surface)] transition-colors flex items-center gap-2 font-geist tracking-wide">
          <Shield className="w-4 h-4" />
          Dashboard
        </Link>
        <ArrowRight className="w-4 h-4" />
        <span className="text-[var(--on-surface)] font-geist">Compliance</span>
      </motion.div>

      {/* ── HEADER ────────────────────────────────────────────────────── */}
      <motion.header variants={itemVariants} className="col-span-12 mb-8">
        <p className={`${LABEL_CLASS} text-[var(--accent-cyan)] mb-1`}>
          PrivacyOps Command Center
        </p>
        <h1 className="text-4xl font-extrabold tracking-tight text-[var(--on-surface)] mb-2 font-geist">
          DPDP Compliance
        </h1>
        <p className="text-sm text-[var(--text-muted)] font-geist max-w-2xl">
          Real-time enforcement of data residency, PII redaction, and global privacy policies.
        </p>
      </motion.header>

      {/* ── TOP ROW ──────────────────────────────────────────────── */}
      
      {/* Redaction Coverage — col-span-4 */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4">
        <BentoCard glowColor="cyan" className="h-full p-6 flex flex-col items-center justify-center">
          <h3 className={`${LABEL_CLASS} mb-6`}>
            Redaction Success
          </h3>
          <ProportionalArcDonut value={redactionCoverage} size={160} strokeWidth={10} />
        </BentoCard>
      </motion.div>

      {/* Data Residency Status — col-span-4 */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4">
        <BentoCard glowColor="cyan" className="h-full p-6 flex flex-col justify-center">
          <h3 className={`${LABEL_CLASS} mb-4 flex items-center gap-2`}>
            Data Residency Status
          </h3>
          <div className="flex items-center gap-4">
            <div className={`${DATA_CLASS} text-6xl font-bold tracking-tighter tabular-nums`}>
              {metrics?.dpdp_blocks || 0}<span className="text-3xl text-[var(--text-muted)] pl-2 font-normal font-geist">Blocks</span>
            </div>
            <div className="w-3 h-3 rounded-full bg-[var(--accent-cyan)] shadow-[0_0_15px_rgba(34,211,238,0.8)] animate-pulse" />
          </div>
          <p className="text-[10px] text-[var(--text-muted)] mt-4 font-geist leading-relaxed uppercase tracking-widest font-bold">
            Geo-routing enforcement active
          </p>
        </BentoCard>
      </motion.div>

      {/* Active Policies — col-span-4 */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4">
        <BentoCard glowColor="none" className="h-full p-6">
          <h3 className={`${LABEL_CLASS} mb-4`}>
            Active Policies
          </h3>
          <div className="space-y-2.5">
            <div className="flex items-center justify-between p-4 rounded-xl bg-[var(--surface-container-low)]">
              <span className="font-geist text-xs font-bold text-[var(--on-surface)]">Global DPDP Shield</span>
              <div className="flex items-center gap-2">
                <span className="text-[9px] text-[var(--accent-cyan)] font-geist tracking-widest uppercase font-bold">Active</span>
                <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent-cyan)] shadow-[0_0_10px_rgba(34,211,238,0.6)]" />
              </div>
            </div>
            <div className="flex items-center justify-between p-4 rounded-xl bg-[var(--surface-container-low)]">
              <span className="font-geist text-xs font-bold text-[var(--on-surface)]">Real-time PII Redaction</span>
              <div className="flex items-center gap-2">
                <span className="text-[9px] text-[var(--accent-cyan)] font-geist tracking-widest uppercase font-bold">Active</span>
                <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent-cyan)] shadow-[0_0_10px_rgba(34,211,238,0.6)]" />
              </div>
            </div>
          </div>
        </BentoCard>
      </motion.div>

      {/* ── MIDDLE ROW ─────────────────────────────────────────────── */}
      
      {/* PII Detection Timeline — col-span-8 */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-8">
        <BentoCard glowColor="cyan" className="h-80 p-6 flex flex-col">
          <h3 className={`${LABEL_CLASS} mb-4 flex items-center gap-2`}>
            <TrendingUp className="w-4 h-4 text-[var(--accent-cyan)]" />
            PII Detection Timeline
          </h3>
          <div className="flex-1 min-h-0 w-full relative flex items-center justify-center">
            {piiTimeline.length > 0 ? (
              <AreaChartGradient
                data={piiTimeline}
                series={[]}
                xAxisKey="timestamp"
              />
            ) : (
              <div className="flex flex-col items-center justify-center">
                <div className="relative flex items-center justify-center mb-6">
                  <div className="absolute w-12 h-12 rounded-full border border-[var(--accent-cyan)]/20 animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite]" />
                  <div className="absolute w-8 h-8 rounded-full border border-[var(--accent-cyan)]/40 animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite]" style={{ animationDelay: '0.5s' }} />
                  <div className="w-3 h-3 rounded-full bg-[var(--accent-cyan)] shadow-[0_0_15px_var(--accent-cyan)]" />
                </div>
                <span className="font-jetbrains text-[0.75rem] text-[var(--accent-cyan)] uppercase tracking-[0.2em] animate-pulse">
                  Awaiting Telemetry...
                </span>
              </div>
            )}
          </div>
        </BentoCard>
      </motion.div>

      {/* Severity Treemap — col-span-4 */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4">
        <BentoCard glowColor="rose" className="h-80 p-6 flex flex-col">
          <h3 className={`${LABEL_CLASS} mb-4 flex items-center gap-2`}>
            <AlertTriangle className="w-4 h-4 text-[var(--primary-rose)]" />
            Severity Treemap
          </h3>
          <div className="flex-1 min-h-0 relative">
            <ResponsiveContainer width="100%" height="100%">
              <Treemap
                data={treemapData}
                dataKey="size"
                aspectRatio={4 / 3}
                stroke="var(--bg-canvas)"
                fill="var(--accent-cyan)"
                content={<CustomizedContent />}
              >
                <RechartsTooltip 
                  cursor={false}
                  contentStyle={{ backgroundColor: 'var(--surface-container-low)', borderColor: 'transparent', borderRadius: '12px', boxShadow: '0 8px 32px rgba(0,0,0,0.5)', backdropFilter: 'blur(12px)' }}
                  itemStyle={{ fontFamily: 'monospace', color: 'var(--on-surface)', fontSize: '10px' }}
                />
              </Treemap>
            </ResponsiveContainer>
          </div>
        </BentoCard>
      </motion.div>

      {/* ── BOTTOM ROW ──────────────────────────────────────────────── */}
      
      {/* Live Violation Feed — col-span-12 */}
      <motion.div variants={itemVariants} className="col-span-12">
        <BentoCard glowColor="rose" className="overflow-hidden flex flex-col">
          <div className="p-6 bg-[rgba(255,255,255,0.02)] flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
            <h3 className={`${LABEL_CLASS} flex items-center gap-2`}>
              <Eye className="w-4 h-4 text-[var(--accent-cyan)]" />
              Live Violation Feed
            </h3>
            <Badge variant="danger" className="animate-pulse shadow-[0_0_15px_rgba(244,63,94,0.4)] border-none">
              Live Intercept
            </Badge>
          </div>
          
          <div className="overflow-x-auto max-h-[400px] overflow-y-auto custom-scrollbar p-2">
            <div className="flex flex-col gap-2 min-w-[800px]">
              <div className="flex items-center px-6 py-4 sticky top-0 bg-[var(--surface-lowest)] z-10">
                 <div className={`${LABEL_CLASS} w-48`}>Timestamp</div>
                 <div className={`${LABEL_CLASS} w-32`}>Severity</div>
                 <div className={`${LABEL_CLASS} w-48`}>Policy Executed</div>
                 <div className={`${LABEL_CLASS} flex-1`}>Intercepted Payload (Redacted)</div>
              </div>
              
              {violations.length > 0 ? (
                violations.map((log, index) => (
                  <div key={log.id} className={`flex items-center px-6 py-4 rounded-xl transition-colors group ${index % 2 === 0 ? 'bg-[var(--surface-container-low)]' : 'bg-[var(--surface-container-lowest)]'}`}>
                    <div className="w-48 text-[0.75rem] font-jetbrains tabular-nums text-[var(--text-muted)]">
                      {log.time}
                    </div>
                    <div className="w-32">
                      <span className={`px-2.5 py-1 text-[9px] uppercase font-bold tracking-widest rounded-full ${
                        log.severity === 'CRITICAL' ? 'bg-[rgba(244,63,94,0.1)] text-[var(--primary-rose)]' :
                        log.severity === 'WARN' ? 'bg-[rgba(245,158,11,0.1)] text-[var(--primary-amber)]' :
                        'bg-[rgba(34,211,238,0.1)] text-[var(--accent-cyan)]'
                      }`}>
                        {log.severity}
                      </span>
                    </div>
                    <div className="w-48 text-xs font-bold font-geist text-[var(--on-surface)]">
                      {log.policy}
                    </div>
                    <div className="flex-1 text-[0.75rem] font-jetbrains text-[var(--text-muted)] break-all leading-relaxed">
                      {log.payload.split(/(\[.*?\])/g).map((part: string, i: number) => {
                        if (part.startsWith('[') && part.endsWith(']')) {
                          return (
                            <span key={i} className="text-[var(--primary-rose)] bg-[var(--surface-bright)] px-1.5 py-0.5 rounded font-bold inline-block mx-0.5 shadow-sm">
                              {part}
                            </span>
                          );
                        }
                        return <span key={i}>{part}</span>;
                      })}
                    </div>
                  </div>
                ))
              ) : (
                <div className="flex flex-col items-center justify-center py-24">
                  <div className="relative flex items-center justify-center mb-6">
                    <div className="absolute w-12 h-12 rounded-full border border-[var(--accent-cyan)]/20 animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite]" />
                    <div className="absolute w-8 h-8 rounded-full border border-[var(--accent-cyan)]/40 animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite]" style={{ animationDelay: '0.5s' }} />
                    <div className="w-3 h-3 rounded-full bg-[var(--accent-cyan)] shadow-[0_0_15px_var(--accent-cyan)]" />
                  </div>
                  <span className="font-jetbrains text-[0.75rem] text-[var(--accent-cyan)] uppercase tracking-[0.2em] animate-pulse">
                    Awaiting Telemetry...
                  </span>
                </div>
              )}
            </div>
          </div>
        </BentoCard>
      </motion.div>
    </motion.div>
  );
}
