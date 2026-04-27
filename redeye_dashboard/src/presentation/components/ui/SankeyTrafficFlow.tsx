import React, { useMemo, useState } from 'react';
import { Sankey, ResponsiveContainer, Tooltip, Layer } from 'recharts';

// ── Types & Constants ─────────────────────────────────────────────────────────

interface SankeyNode {
  name: string;
  color?: string;
}

interface SankeyLink {
  source: number;
  target: number;
  value: number;
}

interface SankeyData {
  nodes: SankeyNode[];
  links: SankeyLink[];
}

const COLORS = {
  ingress: '#22d3ee', // Cyan
  engine: '#3A3939',  // Surface Bright / Neutral
  gpt4o: '#22d3ee',   // Cyan
  claude: '#fbbf24',  // Amber
  gemini: '#fb7185',  // Rose
  success: '#10b981', // Emerald
  failed: '#f43f5e',  // Rose
  redacted: '#f59e0b', // Amber
};

// ── Magnitude Formatter ───────────────────────────────────────────────────────
function fmtMag(raw: string | number | undefined | null): string {
  if (raw === undefined || raw === null) return '—';
  const n = typeof raw === 'string' ? parseFloat(raw) : raw;
  if (isNaN(n)) return '—';
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000)     return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000)         return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(n % 1 === 0 ? 0 : 1);
}

// ── Mock Data ────────────────────────────────────────────────────────────────

const MOCK_DATA: SankeyData = {
  nodes: [
    { name: 'Global Ingress' },   // 0
    { name: 'Routing Engine' },   // 1
    { name: 'GPT-4o' },           // 2
    { name: 'Claude 3.5' },       // 3
    { name: 'Gemini' },           // 4
    { name: 'Success' },          // 5
    { name: 'Failed' },           // 6
    { name: 'PII Redacted' },      // 7
  ],
  links: [
    { source: 0, target: 1, value: 1200000 },
    
    { source: 1, target: 2, value: 650000 },
    { source: 1, target: 3, value: 350000 },
    { source: 1, target: 4, value: 200000 },
    
    { source: 2, target: 5, value: 610000 },
    { source: 2, target: 6, value: 30000 },
    { source: 2, target: 7, value: 10000 },
    
    { source: 3, target: 5, value: 330000 },
    { source: 3, target: 6, value: 15000 },
    { source: 3, target: 7, value: 5000 },
    
    { source: 4, target: 5, value: 195000 },
    { source: 4, target: 6, value: 4000 },
    { source: 4, target: 7, value: 1000 },
  ],
};

const getNodeColor = (name: string) => {
  if (name.includes('Ingress')) return COLORS.ingress;
  if (name.includes('Engine')) return COLORS.engine;
  if (name.includes('GPT')) return COLORS.gpt4o;
  if (name.includes('Claude')) return COLORS.claude;
  if (name.includes('Gemini')) return COLORS.gemini;
  if (name.includes('Success')) return COLORS.success;
  if (name.includes('Failed')) return COLORS.failed;
  if (name.includes('PII')) return COLORS.redacted;
  return 'var(--surface-bright)';
};

// ── Custom Components ─────────────────────────────────────────────────────────

const CustomNode = (props: any) => {
  const { x, y, width, height, index, payload, containerWidth } = props;
  const isOut = x > containerWidth / 2;
  const color = getNodeColor(payload.name);

  return (
    <Layer key={`node-${index}`}>
      <defs>
        <filter id={`glass-glow-${index}`} x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="2" result="blur" />
          <feComposite in="SourceGraphic" in2="blur" operator="over" />
        </filter>
        <linearGradient id={`node-grad-${index}`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor={color} stopOpacity={0.8} />
          <stop offset="100%" stopColor={color} stopOpacity={0.4} />
        </linearGradient>
      </defs>
      
      {/* 3D Glass Pill Shape */}
      <rect
        x={x}
        y={y}
        width={width}
        height={height}
        fill={`url(#node-grad-${index})`}
        rx={height / 2}
        ry={height / 2}
        style={{
          filter: `url(#glass-glow-${index})`,
          backdropFilter: 'blur(24px)',
          WebkitBackdropFilter: 'blur(24px)',
        }}
        className="transition-all duration-500"
      />
      
      <text
        x={isOut ? x - 12 : x + width + 12}
        y={y + height / 2}
        textAnchor={isOut ? 'end' : 'start'}
        verticalAnchor="middle"
        className="font-geist text-[9px] font-bold uppercase tracking-[0.2em] fill-[var(--on-surface-muted)] select-none"
      >
        {payload.name}
      </text>
    </Layer>
  );
};

const CustomLink = (props: any) => {
  const { sourceX, sourceY, targetX, targetY, width, payload, index } = props;
  const [isHovered, setIsHovered] = useState(false);

  const sourceColor = getNodeColor(payload.source.name);
  const targetColor = getNodeColor(payload.target.name);
  const gradientId = `link-gradient-${index}-${payload.source.name}-${payload.target.name}`.replace(/\s+/g, '-');

  return (
    <Layer 
      key={`link-${index}`}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <defs>
        <linearGradient id={gradientId} x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor={sourceColor} />
          <stop offset="100%" stopColor={targetColor} />
        </linearGradient>
        <filter id="cyan-glow">
          <feGaussianBlur stdDeviation="4" result="blur" />
          <feFlood floodColor="#22d3ee" floodOpacity="0.5" result="glowColor" />
          <feComposite in="glowColor" in2="blur" operator="in" result="softGlow" />
          <feMerge>
            <feMergeNode in="softGlow" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>
      
      {/* Liquid Curve Path */}
      <path
        d={`
          M${sourceX},${sourceY}
          C${(sourceX + targetX) / 2},${sourceY} ${(sourceX + targetX) / 2},${targetY} ${targetX},${targetY}
        `}
        fill="none"
        stroke={`url(#${gradientId})`}
        strokeWidth={Math.max(width, 2)}
        strokeOpacity={isHovered ? 0.8 : 0.2}
        style={{
          filter: isHovered ? 'url(#cyan-glow)' : 'none',
        }}
        className="transition-all duration-500 cursor-pointer"
      />
    </Layer>
  );
};

const GhostTooltip = ({ active, payload }: any) => {
  if (active && payload && payload.length) {
    const data = payload[0].payload;
    const isNode = data.source === undefined;
    
    return (
      <div className="bg-[#131313]/90 backdrop-blur-xl p-3 rounded-lg border-none shadow-[0_0_24px_rgba(0,0,0,0.5)] z-50">
        <div className="flex flex-col gap-1">
          <span className="font-geist text-[9px] font-bold text-[var(--on-surface-muted)] uppercase tracking-widest">
            {isNode ? 'Node Analytics' : 'Flow Vector'}
          </span>
          <span className="font-geist text-[11px] text-white font-black uppercase">
            {isNode ? data.name : `${data.source.name} ➔ ${data.target.name}`}
          </span>
          <div className="h-[1px] w-full bg-white/5 my-1" />
          <span className="font-jetbrains text-[12px] text-cyan-400 tabular-nums font-bold">
            {fmtMag(data.value)} <span className="text-[9px] text-cyan-400/50 ml-1">TOKENS</span>
          </span>
        </div>
      </div>
    );
  }
  return null;
};

// ── Main Component ────────────────────────────────────────────────────────────

export function SankeyTrafficFlow({ className = '' }: { className?: string }) {
  const data = useMemo(() => MOCK_DATA, []);

  return (
    <div className={`w-full h-[380px] min-h-[350px] relative p-8 ${className}`}>
      <ResponsiveContainer width="100%" height="100%">
        <Sankey
          data={data}
          node={<CustomNode />}
          link={<CustomLink />}
          nodePadding={50}
          nodeWidth={10}
          margin={{ top: 40, right: 140, bottom: 40, left: 140 }}
          iterations={64}
        >
          <Tooltip 
            content={<GhostTooltip />} 
            cursor={false}
            wrapperStyle={{ outline: 'none' }}
          />
        </Sankey>
      </ResponsiveContainer>

      {/* Spatial HUD Overlay */}
      <div className="absolute top-4 left-4 flex flex-col gap-1 pointer-events-none">
        <div className="flex items-center gap-2">
          <div className="w-1 h-4 bg-cyan-500 rounded-full" />
          <span className="font-jetbrains text-[10px] font-bold text-white uppercase tracking-tighter">
            Flow Visualization Mode: Adaptive
          </span>
        </div>
      </div>
    </div>
  );
}
