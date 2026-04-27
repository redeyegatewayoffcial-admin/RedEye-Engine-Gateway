export function KeyUsageHeatmap({ dailyRequests }: { dailyRequests: number[] }) {
  if (!dailyRequests || dailyRequests.length === 0) {
    return (
      <div className="flex gap-1 h-6 items-center">
        <span className="text-[9px] text-[var(--text-subtle)] font-mono">NO DATA</span>
      </div>
    );
  }

  const max = Math.max(...dailyRequests, 1);
  return (
    <div className="flex gap-1">
      {dailyRequests.map((val, i) => {
        const intensity = val / max;
        // Use cyan mapping exclusively based on intensity
        const color = '6, 182, 212';
        return (
          <div 
            key={i} 
            className="w-2 h-6 rounded-[2px]" 
            style={{ 
              backgroundColor: `rgba(${color}, ${Math.max(0.15, intensity)})`,
              boxShadow: intensity > 0.8 ? `0 0 8px rgba(${color}, 0.5)` : 'none'
            }}
            title={`${val} requests`}
          />
        );
      })}
    </div>
  );
}
