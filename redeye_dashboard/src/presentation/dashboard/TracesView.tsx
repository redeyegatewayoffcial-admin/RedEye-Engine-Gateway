import { Activity, Loader2, AlertCircle, Shield, ArrowRight, X, Clock, Zap, Database, Cpu, Lock } from 'lucide-react';
import { Badge } from '../components/ui/Badge';
import { BentoCard } from '../components/ui/BentoCard';
import useSWR from 'swr';
import { motion, AnimatePresence } from 'framer-motion';
import { Link } from 'react-router-dom';
import { useState } from 'react';

// ── Types ────────────────────────────────────────────────────────────────────

interface Trace {
  traceId: string;
  tenantId: string;
  model: string;
  tokens: number;
  latency: string;
  policy: string;
  status: number;
  path: string;
}

// ── Kinetic CSS Injection ───────────────────────────────────────────────────

const KINETIC_STYLE_ID = 'kinetic-typography-styles';
if (typeof document !== 'undefined' && !document.getElementById(KINETIC_STYLE_ID)) {
  const style = document.createElement('style');
  style.id = KINETIC_STYLE_ID;
  style.textContent = `
    @keyframes kinetic-shiver {
      0%, 100% { transform: translateX(0); }
      25% { transform: translateX(-1px); }
      75% { transform: translateX(1px); }
    }
    @keyframes kinetic-glitch {
      0%, 100% { 
        text-shadow: 0.05em 0 0 var(--accent-cyan), -0.05em -0.025em 0 var(--primary-rose);
        transform: translate(0);
      }
      15% {
        text-shadow: -0.05em -0.025em 0 var(--accent-cyan), 0.025em 0.025em 0 var(--primary-rose);
        transform: translate(-1px, 1px);
      }
      49% {
        text-shadow: 0.05em 0.025em 0 var(--accent-cyan), -0.05em -0.025em 0 var(--primary-rose);
        transform: translate(1px, -1px);
      }
    }
  `;
  document.head.appendChild(style);
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
const DATA_CLASS = 'font-jetbrains text-[var(--on-surface)]';

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

const drawerVariants = {
  hidden: { x: '100%', opacity: 0.5 },
  visible: { 
    x: 0, 
    opacity: 1,
    transition: { type: 'spring', damping: 25, stiffness: 200 }
  },
  exit: { 
    x: '100%', 
    opacity: 0,
    transition: { duration: 0.3, ease: 'easeInOut' }
  }
};

// ── Kinetic Components ──────────────────────────────────────────────────────

const KineticLatency = ({ latency }: { latency: string }) => {
  const ms = parseInt(latency) || 0;
  
  let styles = "font-light tracking-tighter text-[var(--accent-cyan)]";
  let animate = {};

  if (ms >= 1000) {
    styles = "font-black tracking-[0.2em] text-[var(--primary-rose)]";
    animate = { scaleX: 1.1, originX: 1 };
  } else if (ms >= 200) {
    styles = "font-medium tracking-normal text-[var(--primary-amber)]";
  }

  return (
    <motion.span 
      animate={animate}
      className={`font-jetbrains tabular-nums ${styles}`}
    >
      {latency}
    </motion.span>
  );
};

const KineticStatus = ({ status }: { status: number }) => {
  const is4xx = status >= 400 && status < 500;
  const is5xx = status >= 500;

  let animation = "";
  let color = "text-[var(--accent-cyan)]";

  if (is5xx) {
    animation = "kinetic-glitch 0.3s infinite";
    color = "text-[var(--primary-rose)]";
  } else if (is4xx) {
    animation = "kinetic-shiver 0.1s infinite";
    color = "text-[var(--primary-amber)]";
  }

  return (
    <span 
      className={`font-jetbrains font-bold ${color}`}
      style={{ animation }}
    >
      {status}
    </span>
  );
};

const SpatialPath = ({ path }: { path: string }) => {
  const [isHovered, setIsHovered] = useState(false);
  const displayPath = isHovered ? path : (path.length > 20 ? `${path.slice(0, 15)}...` : path);

  return (
    <motion.div
      layout
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className="relative cursor-pointer overflow-hidden whitespace-nowrap"
      style={{ width: isHovered ? 'auto' : 150 }}
    >
      <motion.span
        layout
        className={`font-jetbrains text-[0.75rem] ${isHovered ? 'text-[var(--accent-cyan)]' : 'text-[var(--text-muted)]'}`}
      >
        {displayPath}
      </motion.span>
    </motion.div>
  );
};

function TraceDrawer({ trace, onClose }: { trace: Trace; onClose: () => void }) {
  // Mock journey data based on trace latency
  const totalMs = parseInt(trace.latency) || 850;
  const stages = [
    { name: 'Auth', icon: Lock, duration: '12ms', status: 'success', color: 'cyan' },
    { name: 'Cache', icon: Database, duration: '4ms', status: 'miss', color: 'amber' },
    { name: 'LLM', icon: Cpu, duration: `${totalMs - 40}ms`, status: 'success', color: 'cyan' },
    { name: 'Redaction', icon: Shield, duration: '24ms', status: 'success', color: 'cyan' },
  ];

  return (
    <motion.div
      variants={drawerVariants}
      initial="hidden"
      animate="visible"
      exit="exit"
      className="fixed top-0 right-0 h-full w-[450px] z-50 bg-[rgba(20,20,20,0.7)] backdrop-filter backdrop-blur-[40px] backdrop-saturate-[200%] shadow-[-1px_0_0_rgba(34,211,238,0.2)] flex flex-col"
    >
      {/* Header */}
      <div className="p-8 flex items-center justify-between border-b border-white/[0.03]">
        <div>
          <p className={LABEL_CLASS}>Trace Details</p>
          <h2 className="text-xl font-bold font-geist mt-1 text-[var(--on-surface)] truncate w-64">
            {trace.traceId}
          </h2>
        </div>
        <motion.button
          whileHover={{ scale: 1.05 }}
          whileTap={{ y: 2, scale: 0.95 }}
          onClick={onClose}
          className="p-2 rounded-xl bg-white/[0.05] text-[var(--on-surface)] shadow-[0_4px_0_0_rgba(0,0,0,0.3)] active:shadow-none transition-all"
        >
          <X className="w-5 h-5" />
        </motion.button>
      </div>

      <div className="flex-1 overflow-y-auto custom-scrollbar p-8 space-y-8">
        {/* Meta Stats */}
        <div className="grid grid-cols-2 gap-4">
          <div className="p-4 rounded-2xl bg-[var(--surface-container-low)]">
            <p className={LABEL_CLASS}>Latency</p>
            <div className="mt-1">
              <KineticLatency latency={trace.latency} />
            </div>
          </div>
          <div className="p-4 rounded-2xl bg-[var(--surface-container-low)]">
            <p className={LABEL_CLASS}>Tokens</p>
            <p className="font-jetbrains text-xl font-bold text-[var(--on-surface)] mt-1">{trace.tokens.toLocaleString()}</p>
          </div>
        </div>

        {/* Journey Timeline */}
        <div>
          <h3 className={`${LABEL_CLASS} mb-6`}>Journey Timeline</h3>
          <div className="relative space-y-0 pl-4">
            {/* Connecting Line */}
            <div className="absolute left-[27px] top-4 bottom-4 w-[2px] bg-gradient-to-b from-[var(--accent-cyan)]/40 via-white/5 to-white/5" />
            
            {stages.map((stage, i) => (
              <motion.div 
                key={stage.name}
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: i * 0.1 }}
                className="relative flex items-start gap-6 pb-10 last:pb-0"
              >
                {/* Node */}
                <div className={`relative z-10 w-7 h-7 rounded-full flex items-center justify-center bg-[#131313] border-2 ${stage.status === 'success' ? 'border-[var(--accent-cyan)] shadow-[0_0_10px_var(--accent-cyan)]' : 'border-[var(--primary-amber)] shadow-[0_0_10px_var(--primary-amber)]'}`}>
                  <stage.icon className={`w-3.5 h-3.5 ${stage.status === 'success' ? 'text-[var(--accent-cyan)]' : 'text-[var(--primary-amber)]'}`} />
                </div>

                {/* Content */}
                <div className="flex-1">
                  <div className="flex items-center justify-between">
                    <p className="font-geist font-bold text-sm text-[var(--on-surface)]">{stage.name}</p>
                    <p className="font-jetbrains text-[10px] text-[var(--on-surface-muted)] uppercase tracking-wider">{stage.duration}</p>
                  </div>
                  <div className="mt-2 p-3 rounded-xl bg-[var(--surface-container-low)] flex items-center justify-between">
                     <span className="font-jetbrains text-[10px] text-[var(--on-surface-muted)] capitalize">Status: {stage.status}</span>
                     <div className={`w-1.5 h-1.5 rounded-full ${stage.status === 'success' ? 'bg-[var(--accent-cyan)]' : 'bg-[var(--primary-amber)]'}`} />
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        </div>

        {/* Payload / Metadata Section */}
        <div className="space-y-4">
           <h3 className={LABEL_CLASS}>Metadata</h3>
           <div className="p-4 rounded-2xl bg-[var(--surface-container-low)] space-y-3">
              <div className="flex justify-between items-center text-xs">
                <span className="text-[var(--on-surface-muted)] font-geist">Tenant</span>
                <span className="font-jetbrains text-[var(--on-surface)]">{trace.tenantId}</span>
              </div>
              <div className="flex justify-between items-center text-xs">
                <span className="text-[var(--on-surface-muted)] font-geist">Model</span>
                <span className="font-jetbrains text-[var(--on-surface)]">{trace.model}</span>
              </div>
              <div className="flex justify-between items-center text-xs">
                <span className="text-[var(--on-surface-muted)] font-geist">Policy</span>
                <Badge variant={trace.policy === 'Allowed' ? 'success' : 'danger'} className="text-[9px] py-0 px-2 h-4 border-none font-bold">
                  {trace.policy}
                </Badge>
              </div>
           </div>
        </div>
      </div>

      {/* Footer / Action */}
      <div className="p-8 mt-auto border-t border-white/[0.03]">
        <button className="w-full btn-secondary font-geist tracking-widest text-[10px]">
          Download Full JSON
        </button>
      </div>
    </motion.div>
  );
}

// ── Main Component ───────────────────────────────────────────────────────────

export function TracesView() {
  const [selectedTrace, setSelectedTrace] = useState<Trace | null>(null);

  const { data: traces, error, isLoading } = useSWR<Trace[]>(
    'http://localhost:8080/v1/admin/traces',
    fetcher,
    { refreshInterval: 5000 }
  );

  return (
    <>
      <motion.div
        variants={containerVariants}
        initial="hidden"
        animate="show"
        className={`grid grid-cols-12 gap-6 p-6 auto-rows-max text-[var(--on-surface)] transition-all duration-500 ${selectedTrace ? 'pr-[480px]' : ''}`}
      >
        {/* Breadcrumb */}
        <motion.div variants={itemVariants} className="col-span-12 flex items-center gap-3 text-sm font-mono text-[var(--text-muted)] mb-2">
          <Link to="/dashboard" className="hover:text-[var(--on-surface)] transition-colors flex items-center gap-2 font-geist tracking-wide">
            <Shield className="w-4 h-4" />
            Dashboard
          </Link>
          <ArrowRight className="w-4 h-4" />
          <span className="text-[var(--on-surface)] font-geist">Traces</span>
        </motion.div>

        {/* Header */}
        <motion.header variants={itemVariants} className="col-span-12 flex flex-col md:flex-row md:items-end justify-between gap-6 mb-8">
          <div>
            <p className={`${LABEL_CLASS} text-[var(--accent-cyan)] mb-1`}>Observability Explorer</p>
            <h1 className="text-4xl font-extrabold tracking-tight text-[var(--on-surface)] mb-2 font-geist">
              Trace Explorer
            </h1>
            <p className="text-sm text-[var(--text-muted)] max-w-2xl font-geist">
              Inspect request flows, tenant context, and policy outcomes in real-time.
            </p>
          </div>

          <div className="flex items-center gap-3 p-1 rounded-full bg-[rgba(255,255,255,0.02)]">
            <div className="flex items-center gap-3 px-4 py-2 rounded-full bg-[var(--surface-bright)] shadow-md">
              {isLoading && !traces ? (
                <Loader2 className="w-4 h-4 text-[var(--accent-cyan)] animate-spin" />
              ) : (
                <div className={`w-2 h-2 rounded-full ${error ? 'bg-[var(--primary-rose)]' : 'bg-[var(--accent-cyan)] shadow-[0_0_10px_var(--accent-cyan)]'} animate-pulse`} />
              )}
              <span className={`${LABEL_CLASS} normal-case tracking-normal`}>
                {error ? 'Tracer Offline' : 'Streaming from Cluster'}
              </span>
            </div>
          </div>
        </motion.header>

        {/* Main Trace Table */}
        <motion.div variants={itemVariants} className="col-span-12 h-fit">
          <BentoCard glowColor="cyan" className="overflow-hidden flex flex-col">
            <div className="p-6 bg-[rgba(255,255,255,0.02)] flex items-center justify-between">
              <div className="flex items-center gap-4">
                <h3 className={LABEL_CLASS}>Recent Traces</h3>
                {isLoading && <Loader2 className="w-3 h-3 animate-spin text-[var(--accent-cyan)]" />}
              </div>
              <span className={`${DATA_CLASS} text-[10px] uppercase tracking-widest text-[var(--on-surface-muted)]`}>
                {traces ? `${traces.length} active spans` : 'Monitoring...'}
              </span>
            </div>

            <div className="flex-1 overflow-x-auto overflow-y-auto custom-scrollbar p-2">
              {error ? (
                <div className="flex flex-col items-center justify-center py-24 text-center">
                  <AlertCircle className="w-10 h-10 text-[var(--primary-rose)] opacity-40 mb-4" />
                  <p className="text-sm text-[var(--text-muted)] max-w-xs font-geist">Failed to fetch live traces. Ensure the gateway service is reachable.</p>
                </div>
              ) : (
                <div className="flex flex-col gap-2 min-w-[850px]">
                  {/* Table Header */}
                  <div className="flex items-center px-6 py-4 sticky top-0 bg-[var(--surface-lowest)] z-10">
                    <div className={`${LABEL_CLASS} w-48`}>Trace ID</div>
                    <div className={`${LABEL_CLASS} flex-1`}>Path</div>
                    <div className={`${LABEL_CLASS} w-40`}>Tenant ID</div>
                    <div className={`${LABEL_CLASS} w-24 text-center`}>Status</div>
                    <div className={`${LABEL_CLASS} w-32 text-right`}>Tokens</div>
                    <div className={`${LABEL_CLASS} w-40 text-center`}>Policy</div>
                    <div className={`${LABEL_CLASS} w-24 text-right`}>Latency</div>
                  </div>

                  {isLoading && !traces ? (
                    Array.from({ length: 6 }).map((_, i) => (
                      <div key={i} className={`flex items-center px-6 py-5 rounded-xl ${i % 2 === 0 ? 'bg-[var(--surface-container-low)]' : 'bg-[var(--surface-container-lowest)]'} animate-pulse`}>
                        <div className="w-48 h-2.5 bg-white/5 rounded-full" />
                        <div className="flex-1 h-2.5 bg-white/5 rounded-full ml-4" />
                        <div className="w-40 h-2.5 bg-white/5 rounded-full ml-4" />
                        <div className="w-24 h-2.5 bg-white/5 rounded-full ml-4" />
                        <div className="w-32 h-2.5 bg-white/5 rounded-full ml-auto" />
                        <div className="w-40 h-5 bg-white/5 rounded-lg mx-auto" />
                        <div className="w-24 h-2.5 bg-white/5 rounded-full ml-auto" />
                      </div>
                    ))
                  ) : traces && traces.length > 0 ? (
                    traces.map((trace, i) => (
                      <motion.div 
                        key={trace.traceId} 
                        layout
                        onClick={() => setSelectedTrace(trace)}
                        className={`flex items-center px-6 py-4 rounded-xl transition-colors group cursor-pointer ${selectedTrace?.traceId === trace.traceId ? 'bg-[var(--surface-bright)] ring-1 ring-[var(--accent-cyan)]/30' : i % 2 === 0 ? 'bg-[var(--surface-container-low)]' : 'bg-[var(--surface-container-lowest)]'} hover:bg-[var(--surface-bright)]`}
                      >
                        <div className="w-48 font-jetbrains text-[0.75rem] text-[var(--text-muted)] group-hover:text-[var(--accent-cyan)] transition-colors">
                          {trace.traceId.split('-')[0]}...{trace.traceId.slice(-4)}
                        </div>
                        <div className="flex-1 min-w-[150px]">
                          <SpatialPath path={trace.path || '/api/v1/generate'} />
                        </div>
                        <div className="w-40 font-geist text-xs font-bold text-[var(--on-surface)] uppercase tracking-tight">{trace.tenantId}</div>
                        <div className="w-24 text-center">
                          <KineticStatus status={trace.status || 200} />
                        </div>
                        <div className="w-32 font-jetbrains text-[0.75rem] text-[var(--on-surface)] text-right tabular-nums">{trace.tokens.toLocaleString()}</div>
                        <div className="w-40 text-center">
                          <Badge
                            variant={trace.policy === 'Allowed' ? 'success' : 'danger'}
                            className={`px-3 py-1 text-[9px] font-bold tracking-[0.1em] border-none ${trace.policy === 'Allowed' ? 'bg-[rgba(34,211,238,0.1)] text-[var(--accent-cyan)]' : 'bg-[rgba(244,63,94,0.1)] text-[var(--primary-rose)]'
                               }`}
                          >
                            {trace.policy.toUpperCase()}
                          </Badge>
                        </div>
                        <div className="w-24 text-right">
                          <KineticLatency latency={trace.latency} />
                        </div>
                      </motion.div>
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
              )}
            </div>
          </BentoCard>
        </motion.div>
      </motion.div>

      {/* Trace Drawer */}
      <AnimatePresence>
        {selectedTrace && (
          <TraceDrawer 
            trace={selectedTrace} 
            onClose={() => setSelectedTrace(null)} 
          />
        )}
      </AnimatePresence>
    </>
  );
}
