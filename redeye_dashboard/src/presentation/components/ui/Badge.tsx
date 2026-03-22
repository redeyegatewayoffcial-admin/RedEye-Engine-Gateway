// Shared UI — Badge
// Small pill badge. Variant: success (emerald) | danger (rose) | neutral (slate).

type BadgeVariant = 'success' | 'danger' | 'neutral';

interface BadgeProps {
  children: React.ReactNode;
  variant?: BadgeVariant;
  className?: string;
}

const variantClasses: Record<BadgeVariant, string> = {
  success: 'bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/20',
  danger:  'bg-rose-500/10 text-rose-400 ring-1 ring-rose-500/20',
  neutral: 'bg-slate-700/40 text-slate-400 ring-1 ring-slate-600/30',
};

export function Badge({ children, variant = 'neutral', className = '' }: BadgeProps) {
  return (
    <span className={`inline-flex items-center px-2 py-1 rounded-full text-[10px] font-semibold ${variantClasses[variant]} ${className}`}>
      {children}
    </span>
  );
}
