// Shared UI — Badge
// Small pill badge. Variant: success (emerald) | danger (rose) | neutral (slate).

type BadgeVariant = 'success' | 'danger' | 'neutral';

interface BadgeProps {
  children: React.ReactNode;
  variant?: BadgeVariant;
  className?: string;
}

const variantClasses: Record<BadgeVariant, string> = {
  success: 'bg-[rgba(16,185,129,0.15)] text-[var(--color-redeye-success)] shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]',
  danger:  'bg-[rgba(244,63,94,0.15)] text-[var(--color-redeye-danger)] shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]',
  neutral: 'bg-[var(--surface-bright)] text-[var(--text-muted)] shadow-[inset_0_1px_0_rgba(255,255,255,0.02)]',
};

export function Badge({ children, variant = 'neutral', className = '' }: BadgeProps) {
  return (
    <span className={`inline-flex items-center px-2 py-1 rounded-full text-[10px] font-semibold backdrop-blur-md ${variantClasses[variant]} ${className}`}>
      {children}
    </span>
  );
}
