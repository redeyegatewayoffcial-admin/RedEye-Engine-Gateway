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
      className="group glass-panel p-4 sm:p-6 flex items-start space-x-3 sm:space-x-4 hover:shadow-[0_0_28px_rgba(34,211,238,0.12)] hover:border-cyan-500/25 transition-all duration-300 w-full cursor-default"
    >
      {/* Icon Badge */}
      <div
        className={`p-2 sm:p-3 rounded-xl bg-slate-900/70 border border-cyan-500/10 flex-shrink-0 ${accentClass} group-hover:scale-110 group-hover:border-cyan-500/25 transition-all duration-300`}
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
