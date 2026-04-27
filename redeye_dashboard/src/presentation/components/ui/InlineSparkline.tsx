import React from 'react';
import { LineChart, Line, ResponsiveContainer } from 'recharts';

const InlinePulseDot = (props: any) => {
  const { cx, cy, index, data, color } = props;
  if (index !== data.length - 1) return null;
  return (
    <g>
      <circle cx={cx} cy={cy} r={3} fill={color} className="animate-ping opacity-75" />
      <circle cx={cx} cy={cy} r={1.5} fill={color} />
    </g>
  );
};

export function InlineSparkline({ data, color = 'var(--accent-cyan)' }: { data: number[], color?: string }) {
  if (!data || data.length === 0) {
    return (
      <div className="h-6 w-16 flex items-center">
        <div className="w-full h-px bg-white/5 relative">
          <div className="absolute inset-0 bg-gradient-to-r from-transparent via-rose-500/20 to-transparent animate-pulse" />
        </div>
      </div>
    );
  }

  const chartData = data.map((val, i) => ({ value: val, index: i }));
  const gradId = `inline-comet-${Math.random().toString(36).substr(2, 9)}`;

  return (
    <div className="h-6 w-16 relative">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={chartData} margin={{ top: 2, right: 2, left: 2, bottom: 2 }}>
          <defs>
            <linearGradient id={gradId} x1="0" y1="0" x2="1" y2="0">
              <stop offset="0%" stopColor={color} stopOpacity={0} />
              <stop offset="100%" stopColor={color} stopOpacity={1} />
            </linearGradient>
            <filter id="inline-glow" x="-20%" y="-20%" width="140%" height="140%">
              <feDropShadow dx="0" dy="0" stdDeviation="1.5" floodColor={color} />
            </filter>
          </defs>
          <Line
            type="monotone"
            dataKey="value"
            stroke={`url(#${gradId})`}
            strokeWidth={1.5}
            dot={<InlinePulseDot data={chartData} color={color} />}
            isAnimationActive={false}
            style={{ filter: 'url(#inline-glow)' }}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

