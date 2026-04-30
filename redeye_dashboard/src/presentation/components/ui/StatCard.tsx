// Shared UI — StatCard
// Glassmorphism stat card with framer-motion entrance animation.
// "Cool Revival" theme: Neon Cyan hover glow + active:scale-95 press effect.

import { motion } from 'framer-motion';

interface StatCardProps {
  title: string;
  value: string | number;
  icon: React.ComponentType<{ className?: string }>;
  accentClass?: string;
  subtitle?: string;
  index?: number;
}

export function StatCard({
  title,
  value,
  icon: Icon,
  accentClass = 'text-cyan-400',
  subtitle,
  index = 0,
}: StatCardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, delay: index * 0.08, ease: 'easeOut' }}
      whileHover={{ y: -3, transition: { duration: 0.2 } }}
      whileTap={{ scale: 0.97 }}
      className="group bg-[var(--glass-bg)] backdrop-blur-xl shadow-[var(--glass-shadow)] p-4 sm:p-6 flex items-start space-x-3 sm:space-x-4 hover:shadow-[var(--glass-shadow-hover)] transition-all duration-300 w-full cursor-default rounded-2xl relative overflow-hidden"
    >
      {/* Icon Badge */}
      <div
        className={`p-2 sm:p-3 rounded-xl bg-black/40 shadow-[inset_0_2px_8px_rgba(0,0,0,0.5),inset_0_1px_0_rgba(255,255,255,0.05)] flex-shrink-0 ${accentClass} group-hover:scale-110 transition-all duration-300`}
      >
        <Icon className="w-5 h-5 sm:w-6 sm:h-6" />
      </div>

      {/* Text */}
      <div className="min-w-0 flex-1">
        <p className="text-xs sm:text-sm font-medium text-slate-400 truncate">{title}</p>
        <h3 className="text-xl sm:text-2xl font-bold mt-1 tracking-tight text-slate-100 truncate">
          {value}
        </h3>
        {subtitle && (
          <p className="text-[10px] sm:text-xs text-cyan-400/70 mt-1 font-medium truncate">{subtitle}</p>
        )}
      </div>
    </motion.div>
  );
}
