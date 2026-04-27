import { Database, Loader2, Shield, ArrowRight, Activity, Zap, Layers } from 'lucide-react';
import useSWR from 'swr';
import { motion } from 'framer-motion';
import { Link } from 'react-router-dom';
import { BentoCard } from '../components/ui/BentoCard';
import { CACHE_METRICS_URL, fetchCacheMetrics } from '../../data/services/metricsService';

// ── Styles ────────────────────────────────────────────────────────────────────

const LABEL_CLASS = 'font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[10px] font-bold';
const DATA_CLASS  = 'font-jetbrains text-[var(--on-surface)]';

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

export function CacheView() {
  const { data: cacheStats, error, isLoading } = useSWR(CACHE_METRICS_URL, fetchCacheMetrics, {
    refreshInterval: 10000,
  });

  const stats = cacheStats || { hit_ratio: 0, miss_ratio: 0, total_lookups: 0 };

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
        <span className="text-[var(--on-surface)] font-geist">Cache</span>
      </motion.div>

      {/* Header */}
      <motion.header variants={itemVariants} className="col-span-12 flex flex-col md:flex-row md:items-end justify-between gap-6 mb-8">
        <div>
          <p className={`${LABEL_CLASS} text-[var(--accent-cyan)] mb-1`}>Memory Performance Layer</p>
          <h1 className="text-4xl font-extrabold tracking-tight text-[var(--on-surface)] mb-2 font-geist">
            Semantic Cache
          </h1>
          <p className="text-sm text-[var(--text-muted)] max-w-2xl font-geist">
            Measure cache efficiency for embeddings and model responses across all tenants in real-time.
          </p>
        </div>
        
        <div className="flex items-center gap-3 p-1 rounded-full bg-[rgba(255,255,255,0.02)]">
          <div className="flex items-center gap-3 px-4 py-2 rounded-full bg-[var(--surface-bright)] shadow-md">
            {isLoading && !cacheStats ? (
              <Loader2 className="w-4 h-4 text-[var(--accent-cyan)] animate-spin" />
            ) : (
              <div className={`w-2 h-2 rounded-full ${error ? 'bg-[var(--primary-rose)]' : 'bg-[var(--accent-cyan)] shadow-[0_0_10px_var(--accent-cyan)]'} animate-pulse`} />
            )}
            <span className={`${LABEL_CLASS} normal-case tracking-normal`}>
              {error ? 'Cache Node Failure' : 'Link: Node 8081'}
            </span>
          </div>
        </div>
      </motion.header>

      {/* Quick Stats Row */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4 h-[180px]">
        <BentoCard glowColor="cyan" className="h-full p-6 flex flex-col justify-between">
          <div className="flex items-center gap-3 text-[var(--accent-cyan)]">
            <Zap className="w-5 h-5" />
            <h3 className={LABEL_CLASS}>Hit Ratio</h3>
          </div>
          <div>
            <p className={`${DATA_CLASS} text-4xl font-bold tracking-tighter`}>
              {(stats.hit_ratio * 100).toFixed(1)}<span className="text-xl text-[var(--text-muted)] ml-1 font-normal font-geist">%</span>
            </p>
            <p className="text-[10px] text-[var(--text-muted)] uppercase tracking-widest font-bold mt-1">L1/L2 intersection</p>
          </div>
        </BentoCard>
      </motion.div>

      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4 h-[180px]">
        <BentoCard glowColor="rose" className="h-full p-6 flex flex-col justify-between">
          <div className="flex items-center gap-3 text-[var(--primary-rose)]">
            <Activity className="w-5 h-5" />
            <h3 className={LABEL_CLASS}>Miss Ratio</h3>
          </div>
          <div>
            <p className={`${DATA_CLASS} text-4xl font-bold tracking-tighter`}>
              {(stats.miss_ratio * 100).toFixed(1)}<span className="text-xl text-[var(--text-muted)] ml-1 font-normal font-geist">%</span>
            </p>
            <p className="text-[10px] text-[var(--text-muted)] uppercase tracking-widest font-bold mt-1">Inference pass-through</p>
          </div>
        </BentoCard>
      </motion.div>

      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4 h-[180px]">
        <BentoCard glowColor="none" className="h-full p-6 flex flex-col justify-between">
          <div className="flex items-center gap-3 text-[var(--on-surface-muted)]">
            <Layers className="w-5 h-5" />
            <h3 className={LABEL_CLASS}>Total Lookups</h3>
          </div>
          <div>
            <p className={`${DATA_CLASS} text-4xl font-bold tracking-tighter`}>
              {stats.total_lookups.toLocaleString()}
            </p>
            <p className="text-[10px] text-[var(--text-muted)] uppercase tracking-widest font-bold mt-1">Global request volume</p>
          </div>
        </BentoCard>
      </motion.div>

      {/* Main Analysis Block */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-8">
        <BentoCard glowColor="cyan" className="p-8 h-full flex flex-col">
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-xl font-bold font-geist">Hit / Miss Efficiency</h2>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-[var(--accent-cyan)]"></div>
              <span className={LABEL_CLASS}>Optimized</span>
            </div>
          </div>
          
          <div className="space-y-8 flex-1 flex flex-col justify-center">
            <div className="relative h-4 w-full bg-[var(--surface-container)] rounded-full overflow-hidden shadow-inner">
              <motion.div
                initial={{ width: 0 }}
                animate={{ width: `${stats.hit_ratio * 100}%` }}
                transition={{ duration: 1, ease: "circOut" }}
                className="absolute h-full bg-gradient-to-r from-[var(--accent-cyan)] to-[var(--primary-amber)] shadow-[0_0_15px_var(--accent-cyan)]"
              />
            </div>
            
            <div className="grid grid-cols-2 gap-8">
              <div>
                <p className={`${LABEL_CLASS} mb-2`}>Optimization Impact</p>
                <p className="text-sm text-[var(--text-muted)] font-geist leading-relaxed">
                  High cache hit ratios reduce both end-to-end latency and downstream token spend for LLM calls by bypassing redundant processing.
                </p>
              </div>
              <div>
                <p className={`${LABEL_CLASS} mb-2`}>Similarity Threshold</p>
                <p className="text-sm text-[var(--text-muted)] font-geist leading-relaxed">
                  Embedding similarity currently set to <span className="text-[var(--accent-cyan)] font-bold font-jetbrains">0.82 cosθ</span>. Drift monitored in real-time to prevent hallucinations.
                </p>
              </div>
            </div>
          </div>
        </BentoCard>
      </motion.div>

      {/* Cache Insights */}
      <motion.div variants={itemVariants} className="col-span-12 lg:col-span-4">
        <BentoCard glowColor="none" className="p-8 h-full">
          <div className="flex items-center gap-3 text-[var(--on-surface-muted)] mb-6">
            <Database className="w-5 h-5" />
            <h2 className="text-xl font-bold font-geist">Node Insights</h2>
          </div>
          
          <ul className="space-y-4">
            {[
              "Embedding similarity thresholds tuned for low drift.",
              "Multi-tenant partitioning prevents cross-tenant bleed.",
              "Cache warming applied for high-traffic tenants.",
              "LRU eviction policy active for older embeddings."
            ].map((insight, idx) => (
              <li key={idx} className="flex gap-4 group">
                <span className="text-[var(--accent-cyan)] font-jetbrains font-bold pt-0.5 opacity-40 group-hover:opacity-100 transition-opacity">0{idx+1}</span>
                <p className="text-sm text-[var(--on-surface)] font-geist leading-tight">{insight}</p>
              </li>
            ))}
          </ul>
        </BentoCard>
      </motion.div>

      {error && (
        <motion.div variants={itemVariants} className="col-span-12 p-4 rounded-2xl bg-[rgba(244,63,94,0.1)] flex items-center gap-3">
          <Shield className="w-5 h-5 text-[var(--primary-rose)] flex-shrink-0" />
          <p className="text-sm text-[var(--on-surface)] font-geist">Node synchronization error: {error.message}. Metrics may be out of date.</p>
        </motion.div>
      )}

    </motion.div>
  );
}
