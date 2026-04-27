import { Sparkles, ArrowRight } from 'lucide-react';
import { motion } from 'framer-motion';

interface InsightPillProps {
  message: string;
  type: 'warning' | 'suggestion';
  onClick?: () => void;
}

export function InsightPill({ message, type, onClick }: InsightPillProps) {
  const isWarning = type === 'warning';
  const glowColor = isWarning ? 'var(--primary-rose)' : 'var(--accent-cyan)';
  const shadowColor = isWarning ? 'rgba(244, 63, 94, 0.4)' : 'rgba(34, 211, 238, 0.4)';

  return (
    <motion.div
      initial={{ y: -50, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      whileHover={{ y: -2, scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onClick={onClick}
      className={`
        pointer-events-auto cursor-pointer
        flex items-center gap-3 px-4 py-2.5 rounded-full
        bg-[var(--surface-bright)] backdrop-blur-[24px]
        border border-white/5 shadow-2xl
        group relative transition-all duration-300
      `}
      style={{
        boxShadow: `0 8px 32px -8px ${shadowColor}`,
      }}
    >
      {/* Pulsing Glow Effect */}
      <motion.div
        animate={{
          opacity: [0.4, 0.8, 0.4],
          scale: [1, 1.05, 1],
        }}
        transition={{
          duration: 3,
          repeat: Infinity,
          ease: "easeInOut"
        }}
        className="absolute inset-0 rounded-full blur-[12px]"
        style={{ backgroundColor: glowColor }}
      />

      <div className="relative flex items-center gap-3">
        <div className={`p-1.5 rounded-full bg-white/5`}>
          <Sparkles className={`w-3.5 h-3.5 animate-pulse`} style={{ color: glowColor }} />
        </div>
        
        <span className="font-geist text-[11px] font-bold text-[var(--on-surface)] tracking-wide whitespace-nowrap">
          {message}
        </span>

        <div className="flex items-center gap-1 pl-2 border-l border-white/10 group-hover:gap-2 transition-all">
          <span className="font-jetbrains text-[9px] uppercase tracking-widest text-[var(--on-surface-muted)]">
            Actions
          </span>
          <ArrowRight className="w-3 h-3 text-[var(--on-surface-muted)]" />
        </div>
      </div>
    </motion.div>
  );
}
