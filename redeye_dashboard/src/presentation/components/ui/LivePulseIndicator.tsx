import { motion } from 'framer-motion';

interface LivePulseIndicatorProps {
  status?: 'active' | 'warning' | 'error' | 'offline';
  className?: string;
}

export function LivePulseIndicator({ status = 'active', className = '' }: LivePulseIndicatorProps) {
  const colorMap = {
    active: 'bg-cyan-400 shadow-[0_0_10px_rgba(34,211,238,0.8)]',
    warning: 'bg-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.8)]',
    error: 'bg-rose-500 shadow-[0_0_10px_rgba(244,63,94,0.8)]',
    offline: 'bg-slate-500 shadow-none'
  };

  const isLive = status === 'active' || status === 'warning';

  return (
    <div className={`relative flex items-center justify-center ${className}`}>
      {isLive && (
        <motion.div 
          className={`absolute w-3 h-3 rounded-full ${colorMap[status].split(' ')[0]} opacity-75`}
          animate={{ scale: [1, 2], opacity: [0.75, 0] }}
          transition={{ duration: 1.5, repeat: Infinity, ease: "easeOut" }}
        />
      )}
      <div className={`relative w-2 h-2 rounded-full ${colorMap[status]}`} />
    </div>
  );
}
