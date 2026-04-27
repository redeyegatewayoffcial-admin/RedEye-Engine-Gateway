// ── Types ─────────────────────────────────────────────────────────────────────

interface HeatmapCell {
  intensity: number; // 0–1
}

interface ModelUsageHeatmapProps {
  data: HeatmapCell[];
  rows?: number;      // cells per column (days per week)
  minCols?: number;   // minimum number of columns to always render
  className?: string;
}

const DEFAULT_ROWS     = 7;
const DEFAULT_MIN_COLS = 12; // Wider for honeycomb impact

// ── Component ─────────────────────────────────────────────────────────────────

export function ModelUsageHeatmap({
  data,
  rows     = DEFAULT_ROWS,
  minCols  = DEFAULT_MIN_COLS,
  className = '',
}: ModelUsageHeatmapProps) {
  const minCells = rows * minCols;

  const padded: Array<HeatmapCell & { ghost?: boolean }> =
    data.length >= minCells
      ? data
      : [
          ...data,
          ...Array.from({ length: minCells - data.length }, () => ({
            intensity: 0,
            ghost: true,
          })),
        ];

  const colCount = Math.ceil(padded.length / rows);

  return (
    <div className={`w-full flex gap-4 overflow-hidden py-4 ${className}`}>
      {/* Day labels */}
      <div
        className="flex flex-col justify-between text-[7px] uppercase tracking-[0.2em] font-jetbrains font-bold py-2 opacity-30 select-none"
        style={{ color: 'var(--on-surface)' }}
      >
        <span>Mon</span>
        <span>Wed</span>
        <span>Fri</span>
        <span>Sun</span>
      </div>

      <div
        className="grid grid-flow-col gap-x-[2px] gap-y-[2px] flex-1 h-full"
        style={{
          gridTemplateRows:    `repeat(${rows}, 1fr)`,
          gridTemplateColumns: `repeat(${colCount}, 1fr)`,
        }}
      >
        {padded.map((cell, i) => {
          const colIndex = Math.floor(i / rows);
          const isEvenCol = colIndex % 2 === 0;
          
          const intensity = Math.min(Math.max(cell.intensity, 0), 1);
          const isHighIntensity = intensity > 0.6;
          const isGhost = !!cell.ghost || intensity < 0.05;

          // Spatial Elevation Styles
          const elevationStyle: React.CSSProperties = isHighIntensity ? {
            transform: `scale(1.15) translateY(${isEvenCol ? 'calc(50% - 2px)' : '-2px'})`,
            filter: 'drop-shadow(0 6px 12px rgba(34,211,238,0.4)) saturate(150%)',
            zIndex: 10,
          } : {
            transform: isEvenCol ? 'translateY(50%)' : 'none',
            zIndex: 1,
          };

          return (
            <div
              key={i}
              className="group relative"
              style={{
                width: '100%',
                aspectRatio: '1 / 1.15', // Hexagonal proportion
                ...elevationStyle,
                transition: 'all 0.4s cubic-bezier(0.16, 1, 0.3, 1)',
              }}
            >
              <div
                className="w-full h-full cursor-crosshair transition-all duration-300 group-hover:scale-125 group-hover:saturate-200 group-hover:z-50"
                style={{
                  clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)',
                  background: isGhost 
                    ? 'rgba(255, 255, 255, 0.03)' 
                    : isHighIntensity 
                      ? 'var(--accent-cyan)' 
                      : `rgba(34, 211, 238, ${0.1 + intensity * 0.5})`,
                  border: isHighIntensity ? '1px solid rgba(255,255,255,0.2)' : 'none',
                }}
              />
              
              {/* Optional: Intensity Tooltip on hover could go here */}
            </div>
          );
        })}
      </div>

      <style dangerouslySetInnerHTML={{ __html: `
        .hexagon-matrix-container {
          perspective: 1000px;
        }
      `}} />
    </div>
  );
}

