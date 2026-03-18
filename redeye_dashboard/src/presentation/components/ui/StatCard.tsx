// Shared UI — StatCard
// Glassmorphism stat card. Theme: slate-950/indigo-500 (no red).

interface StatCardProps {
  title: string;
  value: string | number;
  icon: React.ComponentType<{ className?: string }>;
  accentClass?: string;
  subtitle?: string;
}

export function StatCard({ title, value, icon: Icon, accentClass = 'text-indigo-400', subtitle }: StatCardProps) {
  return (
    <div className="group glass-panel p-4 sm:p-6 flex items-start space-x-3 sm:space-x-4 hover:-translate-y-1 hover:shadow-[0_0_25px_rgba(99,102,241,0.15)] transition-all duration-300 w-full">
      <div className={`p-2 sm:p-3 rounded-xl bg-slate-900/70 border border-indigo-500/10 flex-shrink-0 ${accentClass} group-hover:scale-110 transition-transform duration-300`}>
        <Icon className="w-5 h-5 sm:w-6 sm:h-6" />
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-xs sm:text-sm font-medium text-slate-400 truncate">{title}</p>
        <h3 className="text-xl sm:text-2xl font-bold mt-1 tracking-tight text-slate-100 truncate">{value}</h3>
        {subtitle && (
          <p className="text-[10px] sm:text-xs text-indigo-400 mt-1 font-medium truncate">{subtitle}</p>
        )}
      </div>
    </div>
  );
}
