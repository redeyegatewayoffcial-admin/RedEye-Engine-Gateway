import type { CSSProperties, ReactNode } from 'react';

// ── Types ─────────────────────────────────────────────────────────────────────

type GlowColor = 'cyan' | 'amber' | 'rose' | 'emerald' | 'none';

interface BentoCardProps {
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
  glowColor?: GlowColor;
  /** Activates the red hazard-pulse animation */
  hazard?: boolean;
  as?: 'div' | 'section' | 'article';
}

// ── Component ─────────────────────────────────────────────────────────────────

export function BentoCard({
  children,
  className = '',
  style,
  glowColor = 'none',
  hazard = false,
  as: Tag = 'div',
}: BentoCardProps) {
  /**
   * NO-LINE RULE:
   *   - No `border` or `border-*` classes ever.
   *   - Depth is expressed purely through background tones + shadow.
   *
   * GRID LOCK CRITICAL FIX:
   *   - `flex flex-col overflow-hidden min-h-0` is enforced here — consumers
   *     must NOT override these on the card root.
   *   - `min-h-0` strictly prevents children (like Recharts) from infinitely
   *     expanding and breaking the parent grid.
   *
   * Base Styling (Obsidian Command):
   *   - bg-[var(--surface-container)]
   *   - text-[var(--on-surface)]
   *   - backdrop-blur-[24px]
   *   - rounded-[1rem]
   *   - box-shadow: 0 24px 48px rgba(0, 0, 0, 0.4)
   */
  const glowClass = `bento-card-glow-${glowColor}`;

  return (
    <Tag
      style={{
        ...style,
        boxShadow: '0 24px 48px rgba(0, 0, 0, 0.4)',
      }}
      className={[
        // Grid Lock - critical for preventing infinite expansion
        'flex flex-col overflow-hidden min-h-0',
        // Base Obsidian styling
        'bento-card text-[var(--on-surface)] rounded-[1rem]',
        // Glow variant on hover (CSS class for hover effects)
        glowClass,
        // Hazard state
        hazard ? 'hazard-active' : '',
        // Consumer overrides (layout positions, spans, padding, etc.)
        // IMPORTANT: Never pass border/outline/ring classes here.
        className,
      ]
        .filter(Boolean)
        .join(' ')}
    >
      {children}
    </Tag>
  );
}
