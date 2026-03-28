// Presentation Page — LandingPage
// Theme: "Cool Revival" — Midnight Obsidian + Neon Cyan/Teal
// Features: framer-motion gateway animation, bento-grid, FAQ accordion, footer.

import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence, useMotionValue, useSpring } from 'framer-motion';
import { ArrowRight, Shield, Zap, Globe, Plus, Lock, BarChart3 } from 'lucide-react';

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
        <div className="w-16 h-16 rounded-2xl border border-slate-700 bg-slate-900/80 flex flex-col items-center justify-center shadow-lg">
          <div className="text-[9px] font-bold text-slate-400 uppercase tracking-widest">Client</div>
          <div className="w-6 h-6 mt-1 rounded-full bg-slate-700 flex items-center justify-center">
            <div className="w-2.5 h-2.5 rounded-full bg-slate-500" />
          </div>
        </div>
        <span className="text-[10px] text-slate-500">Your App</span>
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
        <div className="w-20 h-20 rounded-2xl border border-cyan-500/40 bg-slate-950 flex flex-col items-center justify-center shadow-[0_0_30px_rgba(34,211,238,0.25)] relative overflow-hidden">
          {/* Ripple */}
          <motion.div
            className="absolute inset-0 rounded-2xl border border-cyan-400/30"
            animate={{ scale: [1, 1.5, 1], opacity: [0.5, 0, 0.5] }}
            transition={{ duration: 2.5, repeat: Infinity }}
          />
          <Shield className="w-7 h-7 text-cyan-400" />
          <span className="text-[9px] font-bold text-cyan-400 uppercase tracking-widest mt-1">RedEye</span>
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
            <div className="h-9 w-20 rounded-xl border border-slate-700 bg-slate-900/80 flex items-center justify-center text-[10px] font-semibold text-slate-300 shadow-sm">
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
          className={`absolute top-1/2 w-2 h-2 rounded-full ${fromLeft ? 'bg-cyan-400' : 'bg-teal-400'} shadow-[0_0_8px_rgba(34,211,238,0.9)]`}
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
  const iconBg = accent === 'cyan' ? 'bg-cyan-500/10 border-cyan-500/20 text-cyan-400' : 'bg-teal-500/10 border-teal-500/20 text-teal-400';

  return (
    <motion.div
      variants={fadeUp}
      onMouseMove={(e) => {
        const rect = e.currentTarget.getBoundingClientRect();
        mx.set(e.clientX - rect.left);
        my.set(e.clientY - rect.top);
      }}
      className="glass-panel p-5 sm:p-6 text-left relative overflow-hidden cursor-default group h-full"
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
      <p className="relative text-sm sm:text-base font-semibold text-slate-100 mb-2">{title}</p>
      <p className="relative text-xs sm:text-sm text-slate-500 leading-relaxed">{desc}</p>
    </motion.div>
  );
}

// ── FAQ Accordion Item ───────────────────────────────────────────────────────

function FaqItem({ q, a }: { q: string; a: string }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="border-b border-slate-800/70 last:border-0">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center justify-between py-4 text-left group"
      >
        <span className="text-sm sm:text-base font-medium text-slate-200 group-hover:text-cyan-300 transition-colors duration-200 pr-4">
          {q}
        </span>
        <motion.div animate={{ rotate: open ? 45 : 0 }} transition={{ duration: 0.25 }} className="flex-shrink-0">
          <Plus className={`w-4 h-4 transition-colors duration-200 ${open ? 'text-cyan-400' : 'text-slate-500'}`} />
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
            <p className="text-sm text-slate-400 leading-relaxed pb-5 pr-8">{a}</p>
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
    <div className="relative min-h-screen bg-slate-950 text-slate-100 flex flex-col overflow-hidden">

      {/* Ambient Blobs */}
      <div className="absolute -top-40 -left-40 w-[500px] h-[500px] rounded-full bg-cyan-500/10 blur-[140px] pointer-events-none" />
      <div className="absolute top-1/2 -right-40 w-[400px] h-[400px] rounded-full bg-teal-500/8 blur-[120px] pointer-events-none" />
      <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-[600px] h-[300px] bg-cyan-500/5 blur-[100px] pointer-events-none" />

      {/* ── Nav ──────────────────────────────────────────────────── */}
      <nav className="relative z-10 border-b border-slate-800/60 px-6 sm:px-12 lg:px-20 h-16 flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <div className="h-7 w-7 rounded-lg bg-gradient-to-br from-cyan-500 to-teal-500 flex items-center justify-center shadow-[0_0_16px_rgba(34,211,238,0.45)]">
            <span className="text-[10px] font-bold tracking-tight text-slate-950">RE</span>
          </div>
          <span className="text-sm font-semibold text-slate-100 tracking-wide">RedEye</span>
        </div>
        <div className="flex items-center gap-6">
          <button onClick={() => {}} className="text-sm text-slate-400 hover:text-slate-200 transition-colors hidden sm:block">Features</button>
          <button onClick={() => {}} className="text-sm text-slate-400 hover:text-slate-200 transition-colors hidden sm:block">Docs</button>
          <button
            onClick={() => navigate('/login')}
            className="text-sm font-medium text-slate-400 hover:text-cyan-300 transition-colors duration-200"
          >
            Sign in →
          </button>
        </div>
      </nav>

      {/* ── Hero ─────────────────────────────────────────────────── */}
      <main className="relative z-10 flex-1 flex flex-col items-center px-6 text-center pt-20 sm:pt-28 pb-16">

        <motion.div initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5 }}>
          <div className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-full border border-cyan-500/25 bg-cyan-500/8 text-cyan-400 text-xs font-semibold mb-8 shadow-[0_0_16px_rgba(34,211,238,0.1)]">
            <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 neon-dot inline-block" />
            Enterprise AI Gateway — v2
          </div>
        </motion.div>

        <motion.h1
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.1 }}
          className="text-4xl sm:text-6xl lg:text-7xl font-extrabold tracking-tight leading-[1.08] max-w-4xl"
        >
          <span className="bg-gradient-to-br from-slate-50 via-slate-100 to-slate-300 bg-clip-text text-transparent">
            Enterprise AI
          </span>
          <br />
          <span className="bg-gradient-to-r from-cyan-400 to-teal-400 bg-clip-text text-transparent">
            Gateway
          </span>
        </motion.h1>

        <motion.p
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.2 }}
          className="mt-6 text-lg sm:text-xl text-slate-400 max-w-2xl leading-relaxed"
        >
          Intelligent middleware that sits between your clients and AI providers — enforcing policy,
          redacting PII, and optimizing cost at every request.
        </motion.p>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.3 }}
          className="mt-10 flex flex-col sm:flex-row items-center gap-4 justify-center"
        >
          <button
            onClick={() => navigate('/login')}
            className="inline-flex items-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 active:scale-95 px-6 py-3 text-sm font-semibold text-slate-950 shadow-[0_0_24px_rgba(34,211,238,0.35)] hover:shadow-[0_0_36px_rgba(34,211,238,0.55)] transition-all duration-200"
          >
            Get started free <ArrowRight className="w-4 h-4" />
          </button>
          <button
            onClick={() => navigate('/login')}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 bg-slate-900/50 hover:border-cyan-500/30 hover:text-cyan-300 active:scale-95 px-6 py-3 text-sm font-semibold text-slate-300 transition-all duration-200"
          >
            Sign in
          </button>
        </motion.div>

        {/* ── Gateway Architecture Animation ──────────────────────── */}
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 0.5 }}
          className="mt-20 w-full max-w-3xl relative"
        >
          <div className="absolute inset-0 bg-slate-900/30 rounded-3xl border border-slate-800/60 blur-sm" />
          <div className="relative glass-panel p-6 sm:p-10">
            <p className="text-[10px] uppercase tracking-[0.25em] text-slate-600 mb-6 text-center">Live Gateway Flow</p>
            <GatewayDiagram />
          </div>
        </motion.div>

        {/* ── Bento Feature Grid ───────────────────────────────────── */}
        <div className="mt-24 w-full max-w-4xl">
          <motion.h2
            initial={{ opacity: 0, y: 16 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className="text-2xl sm:text-3xl font-bold text-slate-100 mb-2 text-center"
          >
            Everything you need
          </motion.h2>
          <motion.p
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className="text-slate-500 text-sm mb-10 text-center"
          >
            From security to observability — all in a single gateway.
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
          className="mt-20"
        >
          <p className="text-xs uppercase tracking-[0.2em] text-slate-600 mb-6">Trusted by teams at</p>
          <div className="flex items-center justify-center gap-8 flex-wrap">
            {CLIENTS.map(({ name, abbr }) => (
              <div key={name} className="flex items-center gap-2 opacity-40 hover:opacity-70 transition-opacity duration-200">
                <div className="h-8 w-8 rounded-lg bg-slate-800 border border-slate-700 flex items-center justify-center text-[11px] font-bold text-slate-300">
                  {abbr}
                </div>
                <span className="text-slate-400 text-sm font-medium">{name}</span>
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
          className="mt-24 w-full max-w-2xl text-left"
        >
          <h2 className="text-2xl sm:text-3xl font-bold text-slate-100 mb-2 text-center">Frequently Asked</h2>
          <p className="text-slate-500 text-sm mb-10 text-center">Answers to the most common questions.</p>
          <div className="glass-panel px-6 py-2">
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
          className="mt-24 flex flex-col items-center gap-5 text-center"
        >
          <h2 className="text-3xl sm:text-4xl font-extrabold text-slate-50">Ready to ship smarter AI?</h2>
          <p className="text-slate-400 max-w-md text-sm">Get up and running in under 5 minutes. No credit card required.</p>
          <button
            onClick={() => navigate('/login')}
            className="inline-flex items-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 active:scale-95 px-8 py-3.5 text-sm font-semibold text-slate-950 shadow-[0_0_28px_rgba(34,211,238,0.4)] hover:shadow-[0_0_40px_rgba(34,211,238,0.6)] transition-all duration-200"
          >
            Start for free <ArrowRight className="w-4 h-4" />
          </button>
        </motion.div>
      </main>

      {/* ── Footer ───────────────────────────────────────────────── */}
      <footer className="relative z-10 border-t border-slate-800/60 px-6 sm:px-12 lg:px-20 py-8 mt-16">
        <div className="max-w-6xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-6">
          {/* Brand */}
          <div className="flex items-center gap-2.5">
            <div className="h-6 w-6 rounded-md bg-gradient-to-br from-cyan-500 to-teal-500 flex items-center justify-center">
              <span className="text-[9px] font-bold text-slate-950">RE</span>
            </div>
            <span className="text-xs font-semibold text-slate-400">RedEye AI Engine</span>
          </div>

          {/* Links */}
          <nav className="flex items-center gap-6 flex-wrap justify-center">
            {['About Us', 'Features', 'Docs', 'Terms & Conditions', 'Privacy Policy'].map((link) => (
              <button
                key={link}
                onClick={() => {}}
                className="text-xs text-slate-600 hover:text-slate-300 transition-colors duration-200"
              >
                {link}
              </button>
            ))}
          </nav>

          {/* Copy */}
          <p className="text-xs text-slate-700">
            © {new Date().getFullYear()} RedEye. All rights reserved.
          </p>
        </div>
      </footer>
    </div>
  );
}
