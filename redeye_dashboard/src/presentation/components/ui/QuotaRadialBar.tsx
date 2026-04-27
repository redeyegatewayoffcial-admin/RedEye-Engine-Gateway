import { motion } from 'framer-motion';

export function QuotaRadialBar({ quotaUsed }: { quotaUsed: number }) {
  const radius = 12;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (quotaUsed / 100) * circumference;
  const isHazard = quotaUsed > 80;

  return (
    <div className="relative flex items-center justify-center w-8 h-8">
      <motion.svg 
        className="w-full h-full transform -rotate-90" 
        viewBox="0 0 32 32"
        animate={isHazard ? { scale: [1, 1.1, 1] } : {}}
        transition={isHazard ? { repeat: Infinity, duration: 2, ease: "easeInOut" } : {}}
      >
        <circle cx="16" cy="16" r={radius} fill="none" className="stroke-white/[0.05]" strokeWidth="3" />
        <circle 
          cx="16" cy="16" r={radius} fill="none" 
          stroke={isHazard ? "var(--rose)" : "var(--cyan)"} 
          strokeWidth="3"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          className={isHazard ? "drop-shadow-[0_4px_8px_rgba(244,63,94,0.6)] drop-shadow-[0_0_12px_var(--primary-container,rgba(244,63,94,0.8))] transition-all" : "drop-shadow-[0_2px_4px_rgba(0,0,0,0.5)] transition-all"}
        />
      </motion.svg>
      <span className="absolute font-jetbrains text-[0.75rem] tabular-nums font-bold text-[var(--on-surface)]">
        {Math.round(quotaUsed)}%
      </span>
    </div>
  );
}
