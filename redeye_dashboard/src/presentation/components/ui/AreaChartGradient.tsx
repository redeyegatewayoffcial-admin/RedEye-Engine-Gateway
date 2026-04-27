import React, { useState, useMemo, useEffect, useCallback } from 'react';
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Brush } from 'recharts';
import { motion, AnimatePresence, animate } from 'framer-motion';

interface DataPoint {
  [key: string]: string | number;
}

interface AreaSeries {
  dataKey: string;
  stroke: string; // Hex color
  fillId: string;
}

interface AreaChartGradientProps {
  data: DataPoint[];
  series: AreaSeries[];
  xAxisKey?: string;
  height?: number | string;
  className?: string;
}

const SEGMENTS = ['1H', '24H', '7D', '30D', '1Y'] as const;
type Segment = typeof SEGMENTS[number];

export function AreaChartGradient({ data, series, xAxisKey = 'timestamp', height = '100%', className = '' }: AreaChartGradientProps) {
  const [activeSegment, setActiveSegment] = useState<Segment>('1H');
  const [domain, setDomain] = useState<[number, number]>([0, 0]);
  const [activePayload, setActivePayload] = useState<any>(null);

  // Initial domain setup
  useEffect(() => {
    if (data.length > 0) {
      const start = new Date(data[0][xAxisKey] as string).getTime();
      const end = new Date(data[data.length - 1][xAxisKey] as string).getTime();
      setDomain([start, end]);
    }
  }, [data, xAxisKey]);

  // Handle segment change with smooth interpolation
  const handleSegmentChange = useCallback((segment: Segment) => {
    setActiveSegment(segment);
    
    if (data.length === 0) return;

    const end = new Date(data[data.length - 1][xAxisKey] as string).getTime();
    let start = new Date(data[0][xAxisKey] as string).getTime();

    const hour = 3600000;
    const day = 24 * hour;

    switch (segment) {
      case '1H': start = end - hour; break;
      case '24H': start = end - day; break;
      case '7D': start = end - 7 * day; break;
      case '30D': start = end - 30 * day; break;
      case '1Y': start = end - 365 * day; break;
    }

    // Clip to available data range
    const dataStart = new Date(data[0][xAxisKey] as string).getTime();
    start = Math.max(start, dataStart);

    // Animate the domain transition
    const currentStart = domain[0];
    const currentEnd = domain[1];

    animate(currentStart, start, {
      duration: 0.8,
      ease: [0.16, 1, 0.3, 1], // Cubic-bezier
      onUpdate: (v) => setDomain(prev => [v, prev[1]])
    });

    animate(currentEnd, end, {
      duration: 0.8,
      ease: [0.16, 1, 0.3, 1], // Cubic-bezier
      onUpdate: (v) => setDomain(prev => [prev[0], v])
    });
  }, [data, domain, xAxisKey]);

  const formattedData = useMemo(() => {
    return data.map(d => ({
      ...d,
      [xAxisKey]: new Date(d[xAxisKey] as string).getTime()
    }));
  }, [data, xAxisKey]);

  const LABEL_CLASS = 'font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[9px] font-bold';
  const DATA_CLASS  = 'font-jetbrains tabular-nums text-[10px] text-[var(--on-surface)]';

  const formatTime = (val: any) => {
    const d = new Date(val);
    return isNaN(d.getTime()) ? val : d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  const fmtHUDMag = (val: number) => {
    if (val >= 1_000_000) return `${(val / 1_000_000).toFixed(1)}M`;
    if (val >= 1_000) return `${(val / 1_000).toFixed(1)}K`;
    return val.toString();
  };

  const handleMouseMove = (e: any) => {
    if (e.activePayload) {
      setActivePayload(e.activePayload);
    }
  };

  return (
    <div style={{ height }} className={`w-full flex flex-col ${className}`}>
      {/* Chrono-Scrubber (Time Segment Control) */}
      <div className="flex items-center justify-between mb-4 px-2">
        <div className="flex bg-white/5 p-1 rounded-lg relative">
          <AnimatePresence>
            {SEGMENTS.map((seg) => (
              <button
                key={seg}
                onClick={() => handleSegmentChange(seg)}
                className={`relative px-3 py-1 text-[10px] font-bold tracking-tighter transition-colors duration-300 z-10 ${
                  activeSegment === seg ? 'text-white' : 'text-white/40 hover:text-white/70'
                }`}
              >
                {activeSegment === seg && (
                  <motion.div
                    layoutId="active-pill"
                    className="absolute inset-0 bg-cyan-500/20 rounded-md border border-cyan-500/30 shadow-[0_0_15px_rgba(6,182,212,0.2)]"
                    transition={{ type: 'spring', bounce: 0.2, duration: 0.6 }}
                  />
                )}
                <span className="relative z-10 font-jetbrains">{seg}</span>
              </button>
            ))}
          </AnimatePresence>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-cyan-500 animate-pulse shadow-[0_0_8px_rgba(6,182,212,0.5)]" />
          <span className={LABEL_CLASS}>Live Telemetry</span>
        </div>
      </div>

      <div className="relative flex-1 w-full h-full min-h-0">
        {/* Ghost HUD */}
        <AnimatePresence>
          {activePayload && (
            <motion.div
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -10 }}
              className="absolute top-0 left-4 z-20 pointer-events-none flex items-center gap-4 bg-black/40 backdrop-blur-md px-3 py-1.5 rounded-lg border border-white/5 shadow-2xl"
            >
              <span className="font-jetbrains text-[10px] tabular-nums text-white/40">
                {formatTime(activePayload[0].payload[xAxisKey])}
              </span>
              <div className="h-3 w-px bg-white/10" />
              <div className="flex items-center gap-3">
                <span className="font-jetbrains text-[10px] tabular-nums text-cyan-400 font-bold">
                  VOL: {fmtHUDMag(activePayload.reduce((acc: number, curr: any) => acc + curr.value, 0))}
                </span>
                {activePayload.map((p: any, i: number) => (
                  <div key={i} className="flex items-center gap-1.5">
                    <div className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: p.color }} />
                    <span className="font-jetbrains text-[10px] tabular-nums text-white/80 uppercase">
                      {p.name.substring(0, 3)}: {fmtHUDMag(p.value)}
                    </span>
                  </div>
                ))}
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        <ResponsiveContainer width="100%" height="100%">
          <AreaChart 
            data={formattedData} 
            margin={{ top: 10, right: 10, left: -20, bottom: 0 }} 
            stackOffset="silhouette"
            onMouseMove={handleMouseMove}
            onMouseLeave={() => setActivePayload(null)}
          >
            <defs>
              <filter id="chartDropShadow" x="-20%" y="-20%" width="140%" height="140%">
                <feDropShadow dx="0" dy="8" stdDeviation="12" floodColor="var(--on-surface)" floodOpacity="0.2" />
              </filter>
              {series.map(s => (
                <linearGradient key={s.fillId} id={s.fillId} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={s.stroke} stopOpacity={0.8} />
                  <stop offset="100%" stopColor={s.stroke} stopOpacity={0.2} />
                </linearGradient>
              ))}
            </defs>
            <XAxis 
              dataKey={xAxisKey} 
              type="number"
              stroke="rgba(255, 255, 255, 0.1)" 
              fontSize={10} 
              tickLine={false} 
              axisLine={false} 
              tick={{ fill: 'rgba(255, 255, 255, 0.3)', fontSize: 9, fontFamily: 'JetBrains Mono' }}
              tickFormatter={formatTime}
              domain={domain}
              scale="time"
            />
            <YAxis 
              hide
              axisLine={false} 
              tick={false}
            />
            <Tooltip
              content={<></>}
              cursor={{ 
                stroke: 'var(--accent-cyan)', 
                strokeWidth: 1, 
                strokeDasharray: '3 3', 
                style: { filter: 'drop-shadow(0 0 5px rgba(34,211,238,0.8))' } 
              }}
            />
            {series.map(s => (
              <Area 
                key={s.dataKey} 
                name={s.dataKey}
                type="monotone" 
                dataKey={s.dataKey} 
                stackId="1" 
                stroke={s.stroke} 
                fill={`url(#${s.fillId})`} 
                strokeWidth={0} 
                isAnimationActive={true} 
                animationDuration={1500}
                animationEasing="cubic-bezier(0.16, 1, 0.3, 1)"
                style={{ filter: 'url(#chartDropShadow)' }} 
              />
            ))}
            
            {/* Timeline Minimap (Brush Control) */}
            <Brush
              dataKey={xAxisKey}
              height={40}
              stroke="rgba(6, 182, 212, 0.2)"
              fill="transparent"
              gap={10}
              travellerWidth={10}
              padding={{ top: 10, bottom: 0 }}
              tickFormatter={formatTime}
            >
              <AreaChart data={formattedData}>
                {series.map(s => (
                  <Area 
                    key={`brush-${s.dataKey}`} 
                    type="monotone" 
                    dataKey={s.dataKey} 
                    stackId="1" 
                    stroke={s.stroke} 
                    fill={s.stroke} 
                    fillOpacity={0.1} 
                    strokeWidth={1}
                    isAnimationActive={false}
                  />
                ))}
              </AreaChart>
            </Brush>
          </AreaChart>
        </ResponsiveContainer>
      </div>


      {/* Global CSS for Brush Customization */}
      <style dangerouslySetInnerHTML={{ __html: `
        .recharts-brush-traveller rect {
          fill: #06b6d4 !important;
          rx: 2px;
        }
        .recharts-brush-slide {
          fill: rgba(6, 182, 212, 0.05) !important;
        }
        .recharts-brush-texts text {
          font-family: 'JetBrains Mono' !important;
          font-size: 9px !important;
          fill: rgba(255, 255, 255, 0.3) !important;
          text-transform: uppercase;
        }
      `}} />
    </div>
  );
}


