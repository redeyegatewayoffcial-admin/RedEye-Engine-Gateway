// Presentation Page — LandingPage
// Theme: "The Obsidian Command" — Liquid Glass, Dynamic Theme, 3D Neon Buttons.
// Features: framer-motion gateway animation, bento-grid, FAQ accordion, footer.

import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence, useMotionValue, useSpring } from 'framer-motion';
import { ArrowRight, Shield, Zap, Globe, Plus, Lock, BarChart3, Terminal, LifeBuoy } from 'lucide-react';

// ── Data ────────────────────────────────────────────────────────────────────

const FEATURES = [
  {
    icon: Shield,
    title: 'Policy Enforcement',
    desc: 'OPA-native compliance at every request boundary. Block PII leaks and prompt injections before they reach your LLM.',
    size: 'large',
    accent: 'cyan',
  },
  {
    icon: Zap,
    title: 'Sub-50ms Routing',
    desc: 'Semantic cache and smart load balancing out of the box.',
    size: 'small',
    accent: 'teal',
  },
  {
    icon: Globe,
    title: 'Data Residency',
    desc: 'Region-locked routing for GDPR, HIPAA, and SOC 2.',
    size: 'small',
    accent: 'teal',
  },
  {
    icon: Lock,
    title: 'Zero-Trust Auth',
    desc: 'Passwordless OTP + signed gateway keys with AES-256 encryption in transit and at rest.',
    size: 'small',
    accent: 'cyan',
  },
  {
    icon: BarChart3,
    title: 'Live Telemetry',
    desc: 'ClickHouse-backed real-time metrics for tokens, cost, and latency across every model call.',
    size: 'large',
    accent: 'cyan',
  },
];

const FAQS = [
  {
    q: 'How does RedEye protect my API keys?',
    a: 'All LLM provider keys are encrypted with AES-256 before storage. They are decrypted only in-process, in memory, and are never written to logs or telemetry.',
  },
  {
    q: 'Which LLM providers are supported?',
    a: 'RedEye natively proxies OpenAI, Google Gemini, Groq, and Anthropic. Any OpenAI-compatible provider can be added via custom routing rules.',
  },
  {
    q: 'Is there a free tier?',
    a: 'Yes — self-hosted deployments via Docker Compose are fully free. Cloud-managed plans with SLA guarantees are available for enterprise teams.',
  },
  {
    q: 'How does the semantic cache work?',
    a: 'Incoming prompts are hashed (xxhash) and matched against a Redis cache. Semantically similar prompts with a configurable cosine-similarity threshold are served from cache, cutting cost and latency dramatically.',
  },
];

const CLIENTS = [
  { name: 'Acme Corp', abbr: 'AC' },
  { name: 'Globex Inc', abbr: 'GX' },
  { name: 'Stark Industries', abbr: 'SI' },
];

const BTN_3D = "inline-flex items-center gap-2 bg-gradient-to-b from-[var(--surface-bright)] to-[var(--surface-container)] text-[var(--on-surface)] font-geist font-medium border border-[rgba(255,255,255,0.1)] dark:border-[rgba(255,255,255,0.05)] shadow-[inset_0_1px_1px_rgba(255,255,255,0.15)] hover:shadow-[0_0_20px_rgba(34,211,238,0.4)] hover:border-[var(--accent-cyan)] active:translate-y-[2px] active:shadow-none transition-all duration-200 rounded-lg px-6 py-3 cursor-pointer";

// ── Animation Variants ───────────────────────────────────────────────────────

const fadeUp = {
  hidden: { opacity: 0, y: 24 },
  show: { opacity: 1, y: 0, transition: { duration: 0.5, ease: [0.25, 0.1, 0.25, 1] as [number, number, number, number] } },
};

const stagger = {
  hidden: {},
  show: { transition: { staggerChildren: 0.1 } },
};

// ── Gateway Architecture Diagram ─────────────────────────────────────────────

const PROVIDERS = ['OpenAI', 'Gemini', 'Groq', 'Anthropic'];

function GatewayDiagram() {
  return (
    <div className="relative w-full max-w-3xl mx-auto h-52 sm:h-64 select-none">
      {/* Client Node */}
      <motion.div
        initial={{ opacity: 0, x: -30 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ delay: 0.3, duration: 0.5 }}
        className="absolute left-0 top-1/2 -translate-y-1/2 flex flex-col items-center gap-2"
      >
        <div className="w-16 h-16 rounded-2xl border border-white/5 bg-[var(--surface-container)] flex flex-col items-center justify-center shadow-lg">
          <div className="text-[9px] font-bold text-[var(--on-surface-muted)] uppercase tracking-widest">Client</div>
          <div className="w-6 h-6 mt-1 rounded-full bg-[var(--surface-bright)] flex items-center justify-center">
            <div className="w-2.5 h-2.5 rounded-full bg-[var(--on-surface-muted)]" />
          </div>
        </div>
        <span className="text-[10px] text-[var(--on-surface-muted)]">Your App</span>
      </motion.div>

      {/* Animated Data Packets → Gateway */}
      <PacketFlow fromLeft />

      {/* RedEye Gateway (center) */}
      <motion.div
        initial={{ opacity: 0, scale: 0.7 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ delay: 0.6, duration: 0.5, type: 'spring', stiffness: 200 }}
        className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 flex flex-col items-center gap-2 z-10"
      >
        <div className="w-20 h-20 rounded-2xl border border-[var(--accent-cyan)]/40 bg-[var(--bg-canvas)] flex flex-col items-center justify-center shadow-[0_0_30px_rgba(34,211,238,0.25)] relative overflow-hidden">
          {/* Ripple */}
          <motion.div
            className="absolute inset-0 rounded-2xl border border-[var(--accent-cyan)]/30"
            animate={{ scale: [1, 1.5, 1], opacity: [0.5, 0, 0.5] }}
            transition={{ duration: 2.5, repeat: Infinity }}
          />
          <Shield className="w-7 h-7 text-[var(--accent-cyan)]" />
          <span className="text-[9px] font-bold text-[var(--accent-cyan)] uppercase tracking-widest mt-1">RedEye</span>
        </div>
        <span className="text-[10px] text-teal-400 font-medium">Gateway</span>
      </motion.div>

      {/* Animated Data Packets → Providers */}
      <PacketFlow />

      {/* Provider Nodes (right) */}
      <motion.div
        initial={{ opacity: 0, x: 30 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ delay: 0.9, duration: 0.5 }}
        className="absolute right-0 top-0 h-full flex flex-col items-center justify-around py-2"
      >
        {PROVIDERS.map((p) => (
          <div key={p} className="flex flex-col items-center gap-0.5">
            <div className="h-9 w-20 rounded-xl border border-white/5 bg-[var(--surface-container)] flex items-center justify-center text-[10px] font-semibold text-[var(--on-surface)] shadow-sm">
              {p}
            </div>
          </div>
        ))}
      </motion.div>
    </div>
  );
}

/** Floating animated packet dots travelling left→right (or right to left) */
function PacketFlow({ fromLeft = false }: { fromLeft?: boolean }) {
  const packets = [0, 1, 2];
  return (
    <>
      {packets.map((i) => (
        <motion.div
          key={i}
          className={`absolute top-1/2 w-2 h-2 rounded-full ${fromLeft ? 'bg-[var(--accent-cyan)]' : 'bg-teal-400'} shadow-[0_0_8px_rgba(34,211,238,0.9)]`}
          style={{ left: fromLeft ? '13%' : '55%' }}
          animate={{
            x: fromLeft ? ['0%', '230%'] : ['0%', '160%'],
            opacity: [0, 1, 1, 0],
          }}
          transition={{
            duration: 1.8,
            delay: i * 0.6,
            repeat: Infinity,
            ease: 'linear',
          }}
        />
      ))}
    </>
  );
}

// ── Bento Feature Card ───────────────────────────────────────────────────────

function FeatureCard({ icon: Icon, title, desc, accent }: { icon: React.ComponentType<{ className?: string }>; title: string; desc: string; size: string; accent: string }) {
  const mx = useMotionValue(0);
  const my = useMotionValue(0);
  const glowX = useSpring(mx, { stiffness: 150, damping: 20 });
  const glowY = useSpring(my, { stiffness: 150, damping: 20 });

  const accentColor = accent === 'cyan' ? 'rgba(34,211,238,0.18)' : 'rgba(45,212,191,0.18)';
  const iconBg = accent === 'cyan' ? 'bg-cyan-500/10 border-cyan-500/20 text-[var(--accent-cyan)]' : 'bg-teal-500/10 border-teal-500/20 text-teal-400';

  return (
    <motion.div
      variants={fadeUp}
      onMouseMove={(e) => {
        const rect = e.currentTarget.getBoundingClientRect();
        mx.set(e.clientX - rect.left);
        my.set(e.clientY - rect.top);
      }}
      className="backdrop-blur-[24px] saturate-[200%] bg-[var(--surface-container)] border border-white/5 rounded-[1.5rem] p-5 sm:p-6 text-left relative overflow-hidden cursor-default group h-full shadow-lg"
    >
      {/* Radial mouse-follow glow */}
      <motion.div
        className="pointer-events-none absolute -inset-px rounded-[1.5rem] opacity-0 group-hover:opacity-100 transition-opacity duration-300"
        style={{
          background: `radial-gradient(200px circle at ${glowX}px ${glowY}px, ${accentColor}, transparent 80%)`,
        }}
      />
      <div className={`relative flex items-center justify-center w-10 h-10 rounded-xl border mb-4 ${iconBg}`}>
        <Icon className="w-5 h-5" />
      </div>
      <p className="relative text-sm sm:text-base font-semibold text-[var(--on-surface)] mb-2 font-geist">{title}</p>
      <p className="relative text-xs sm:text-sm text-[var(--on-surface-muted)] leading-relaxed font-geist">{desc}</p>
    </motion.div>
  );
}

// ── FAQ Accordion Item ───────────────────────────────────────────────────────

function FaqItem({ q, a }: { q: string; a: string }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="border-b border-white/5 last:border-0">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center justify-between py-4 text-left group"
      >
        <span className="text-sm sm:text-base font-medium text-[var(--on-surface)] group-hover:text-[var(--accent-cyan)] transition-colors duration-200 pr-4 font-geist">
          {q}
        </span>
        <motion.div animate={{ rotate: open ? 45 : 0 }} transition={{ duration: 0.25 }} className="flex-shrink-0">
          <Plus className={`w-4 h-4 transition-colors duration-200 ${open ? 'text-[var(--accent-cyan)]' : 'text-[var(--on-surface-muted)]'}`} />
        </motion.div>
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            key="answer"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: 'easeInOut' }}
            className="overflow-hidden"
          >
            <p className="text-sm text-[var(--on-surface-muted)] leading-relaxed pb-5 pr-8 font-geist">{a}</p>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// ── LandingPage ──────────────────────────────────────────────────────────────

export function LandingPage() {
  const navigate = useNavigate();

  return (
    <div className="relative min-h-screen bg-[var(--bg-canvas)] text-[var(--on-surface)] flex flex-col overflow-hidden font-geist">

      {/* ── Ambient Mesh Background (Dark Mode Only) ──────────────── */}
      <div className="fixed inset-0 overflow-hidden pointer-events-none z-0 hidden dark:block">
        <div className="absolute -top-[10%] -left-[10%] w-[40%] h-[40%] bg-amber-500/15 blur-[120px] rounded-full mix-blend-screen" />
        <div className="absolute -top-[10%] -right-[10%] w-[35%] h-[40%] bg-rose-500/15 blur-[120px] rounded-full mix-blend-screen" />
        <div className="absolute -bottom-[10%] -left-[10%] w-[40%] h-[40%] bg-cyan-500/15 blur-[120px] rounded-full mix-blend-screen" />
        <div className="absolute -bottom-[10%] -right-[10%] w-[30%] h-[30%] bg-yellow-500/10 blur-[100px] rounded-full mix-blend-screen" />
      </div>

      {/* ── Nav ──────────────────────────────────────────────────── */}
      <nav className="relative z-10 border-b border-white/5 px-6 sm:px-12 lg:px-20 h-16 flex items-center justify-between backdrop-blur-xl bg-[var(--bg-canvas)]/40">
        <div className="flex items-center gap-2.5">
          <div className="h-8 w-8 rounded-xl bg-cyan-500 flex items-center justify-center shadow-[0_0_20px_rgba(34,211,238,0.45)]">
            <span className="text-[11px] font-black tracking-tight text-[#050505]">RE</span>
          </div>
          <span className="text-sm font-bold text-[var(--on-surface)] tracking-wide uppercase">RedEye Command</span>
        </div>
        <div className="flex items-center gap-6">
          <button onClick={() => {}} className="text-xs font-bold uppercase tracking-widest text-[var(--on-surface-muted)] hover:text-[var(--on-surface)] transition-colors hidden sm:block">Features</button>
          <button onClick={() => {}} className="text-xs font-bold uppercase tracking-widest text-[var(--on-surface-muted)] hover:text-[var(--on-surface)] transition-colors hidden sm:block">Documentation</button>
          <button
            onClick={() => navigate('/login')}
            className="text-xs font-bold uppercase tracking-widest text-[var(--on-surface-muted)] hover:text-[var(--accent-cyan)] transition-colors duration-200"
          >
            Sign in →
          </button>
        </div>
      </nav>

      {/* ── Hero ─────────────────────────────────────────────────── */}
      <main className="relative z-10 flex-1 flex flex-col items-center px-6 text-center pt-20 sm:pt-28 pb-16">

        <motion.div initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5 }}>
          <div className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-full border border-[var(--accent-cyan)]/25 bg-[var(--accent-cyan)]/8 text-[var(--accent-cyan)] text-[10px] font-bold uppercase tracking-[0.2em] mb-8 shadow-[0_0_16px_rgba(34,211,238,0.1)]">
            <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-cyan)] neon-dot inline-block" />
            Neural Protocol Access — v2.4.0
          </div>
        </motion.div>

        <motion.h1
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.1 }}
          className="text-4xl sm:text-6xl lg:text-8xl font-black tracking-tight leading-[0.95] max-w-5xl"
        >
          <span className="bg-gradient-to-br from-[var(--on-surface)] to-[var(--on-surface-muted)] bg-clip-text text-transparent">
            Strategic AI
          </span>
          <br />
          <span className="bg-gradient-to-r from-[var(--accent-cyan)] to-teal-400 bg-clip-text text-transparent drop-shadow-[0_0_30px_rgba(34,211,238,0.3)]">
            Command
          </span>
        </motion.h1>

        <motion.p
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.2 }}
          className="mt-8 text-lg sm:text-xl text-[var(--on-surface-muted)] max-w-2xl leading-relaxed font-medium"
        >
          Intelligence middleware for the high-performance era. Enforce policy, 
          redact telemetry, and optimize neural throughput at the edge.
        </motion.p>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.3 }}
          className="mt-12 flex flex-col sm:flex-row items-center gap-4 justify-center"
        >
          <button
            onClick={() => navigate('/login')}
            className={BTN_3D}
          >
            Initiate Deployment <ArrowRight className="w-4 h-4" />
          </button>
          <button
            onClick={() => navigate('/login')}
            className="inline-flex items-center gap-2 rounded-lg border border-white/10 bg-[var(--surface-bright)]/50 hover:border-[var(--accent-cyan)]/30 hover:text-[var(--accent-cyan)] active:translate-y-[1px] px-6 py-3 text-sm font-bold uppercase tracking-widest text-[var(--on-surface-muted)] transition-all duration-200"
          >
            Developer Protocol
          </button>
        </motion.div>

        {/* ── Gateway Architecture Animation ──────────────────────── */}
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 0.5 }}
          className="mt-24 w-full max-w-4xl relative"
        >
          <div className="absolute inset-0 bg-[var(--surface-container)]/30 rounded-[2rem] border border-white/5 blur-sm" />
          <div className="relative backdrop-blur-[40px] saturate-[200%] bg-[var(--surface-container)]/60 border border-white/5 rounded-[2rem] p-6 sm:p-10 shadow-2xl">
            <p className="text-[10px] uppercase tracking-[0.25em] text-[var(--on-surface-muted)] mb-8 text-center font-bold">Real-time Transmission Logic</p>
            <GatewayDiagram />
          </div>
        </motion.div>

        {/* ── Bento Feature Grid ───────────────────────────────────── */}
        <div className="mt-32 w-full max-w-5xl">
          <motion.h2
            initial={{ opacity: 0, y: 16 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className="text-3xl sm:text-4xl font-black text-[var(--on-surface)] mb-2 text-center tracking-tight"
          >
            Operational Mastery
          </motion.h2>
          <motion.p
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className="text-[var(--on-surface-muted)] text-sm mb-12 text-center font-medium uppercase tracking-widest"
          >
            Everything required for a unified command layer.
          </motion.p>
          <motion.div
            variants={stagger}
            initial="hidden"
            whileInView="show"
            viewport={{ once: true }}
            className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 auto-rows-fr"
          >
            {FEATURES.map((f) => (
              <FeatureCard key={f.title} {...f} />
            ))}
          </motion.div>
        </div>

        {/* ── Client Logos ─────────────────────────────────────────── */}
        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="mt-32"
        >
          <p className="text-[10px] uppercase tracking-[0.3em] text-[var(--on-surface-muted)] font-bold mb-10">Validated by Command Centers at</p>
          <div className="flex items-center justify-center gap-12 flex-wrap">
            {CLIENTS.map(({ name, abbr }) => (
              <div key={name} className="flex items-center gap-3 opacity-40 hover:opacity-100 transition-opacity duration-300 group cursor-default">
                <div className="h-10 w-10 rounded-xl bg-[var(--surface-bright)] border border-white/5 flex items-center justify-center text-[11px] font-black text-[var(--on-surface)] group-hover:border-[var(--accent-cyan)]/50 transition-colors">
                  {abbr}
                </div>
                <span className="text-[var(--on-surface-muted)] text-xs font-bold uppercase tracking-widest group-hover:text-[var(--on-surface)] transition-colors">{name}</span>
              </div>
            ))}
          </div>
        </motion.div>

        {/* ── FAQ Accordion ────────────────────────────────────────── */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.55 }}
          className="mt-32 w-full max-w-3xl text-left"
        >
          <h2 className="text-3xl sm:text-4xl font-black text-[var(--on-surface)] mb-2 text-center tracking-tight">Intelligence Briefing</h2>
          <p className="text-[var(--on-surface-muted)] text-sm mb-12 text-center font-medium uppercase tracking-widest">Standardized Procedures & FAQ</p>
          <div className="backdrop-blur-[24px] saturate-[200%] bg-[var(--surface-container)] border border-white/5 rounded-[2rem] px-8 py-4 shadow-xl">
            {FAQS.map((faq) => (
              <FaqItem key={faq.q} {...faq} />
            ))}
          </div>
        </motion.div>

        {/* ── Final CTA ────────────────────────────────────────────── */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.55 }}
          className="mt-32 flex flex-col items-center gap-6 text-center"
        >
          <h2 className="text-4xl sm:text-5xl font-black text-[var(--on-surface)] tracking-tight">Ready for Deployment?</h2>
          <p className="text-[var(--on-surface-muted)] max-w-md text-sm font-medium">Activate your gateway in under 300 seconds. Zero friction deployment.</p>
          <button
            onClick={() => navigate('/login')}
            className={BTN_3D}
          >
            Establish Command <ArrowRight className="w-4 h-4" />
          </button>
        </motion.div>
      </main>

      {/* ── Footer ───────────────────────────────────────────────── */}
      <footer className="relative z-10 border-t border-white/5 px-6 sm:px-12 lg:px-20 py-12 mt-32 bg-[var(--surface-lowest)]/40 backdrop-blur-md">
        <div className="max-w-7xl mx-auto flex flex-col lg:flex-row items-center justify-between gap-10">
          {/* Brand */}
          <div className="flex items-center gap-3">
            <div className="h-8 w-8 rounded-xl bg-cyan-500 flex items-center justify-center shadow-[0_0_20px_rgba(34,211,238,0.4)]">
              <span className="text-[11px] font-black text-[#050505]">RE</span>
            </div>
            <div>
              <p className="text-[9px] uppercase tracking-[0.25em] font-black text-[var(--on-surface-muted)]">RedEye</p>
              <p className="text-xs font-bold tracking-tight leading-none text-[var(--on-surface)]">Command Center</p>
            </div>
          </div>

          {/* Links */}
          <nav className="flex items-center gap-8 flex-wrap justify-center">
            {['Protocol Docs', 'Telemetry Hub', 'Security Policy', 'Operator Terms', 'Privacy Matrix'].map((link) => (
              <button
                key={link}
                onClick={() => {}}
                className="text-[10px] font-bold uppercase tracking-widest text-[var(--on-surface-muted)] hover:text-[var(--accent-cyan)] transition-colors duration-200"
              >
                {link}
              </button>
            ))}
          </nav>

          {/* Copy */}
          <div className="flex flex-col items-center lg:items-end gap-1">
            <p className="text-[10px] font-bold text-[var(--on-surface-muted)] uppercase tracking-[0.1em]">
              © {new Date().getFullYear()} RedEye Operational Intelligence
            </p>
            <p className="text-[9px] text-[var(--on-surface-muted)]/50 uppercase tracking-widest">
              Strategic Asset Deployment v2.4.0
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
