import React from 'react';
import { LineChart, Line, ResponsiveContainer, Dot } from 'recharts';

interface SparklineData {
  val: number;
}

interface SparklineChartProps {
  data: SparklineData[];
  color?: string; // Hex/CSS Var
  className?: string;
}

const CustomPulseDot = (props: any) => {
  const { cx, cy, index, payload, data, color } = props;
  
  // Only render for the last data point
  if (index !== data.length - 1) return null;

  return (
    <g>
      {/* Pulsating outer ring */}
      <circle cx={cx} cy={cy} r={4} fill={color} className="animate-ping opacity-75" />
      {/* Solid center core */}
      <circle cx={cx} cy={cy} r={2.5} fill={color} stroke="#000" strokeWidth={0.5} />
      {/* Glow aura */}
      <circle cx={cx} cy={cy} r={6} fill={color} opacity={0.2} style={{ filter: 'blur(4px)' }} />
    </g>
  );
};

export function SparklineChart({ data, color = 'var(--accent-cyan)', className = '' }: SparklineChartProps) {
  const id = `comet-tail-${color.replace(/[^a-zA-Z0-9]/g, '')}`;
  
  return (
    <div className={`w-full h-full absolute inset-0 z-0 opacity-40 dark:opacity-60 pointer-events-none flex flex-col ${className}`}>
      <div className="relative flex-1 w-full h-full min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} margin={{ top: 5, right: 5, left: 5, bottom: 5 }}>
            <defs>
              <linearGradient id={id} x1="0" y1="0" x2="1" y2="0">
                <stop offset="0%" stopColor={color} stopOpacity={0} />
                <stop offset="50%" stopColor={color} stopOpacity={0.2} />
                <stop offset="100%" stopColor={color} stopOpacity={1} />
              </linearGradient>
              <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
                <feDropShadow dx="0" dy="0" stdDeviation="2" floodColor={color} />
              </filter>
            </defs>
            <Line
              type="monotone"
              dataKey="val"
              stroke={`url(#${id})`}
              strokeWidth={2}
              dot={<CustomPulseDot data={data} color={color} />}
              isAnimationActive={false}
              style={{ filter: 'url(#glow)' }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

