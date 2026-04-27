import { useState, useMemo } from 'react';
import useSWR from 'swr';
import { CreditCard, Loader2, AlertTriangle, Download, Settings, Activity, ArrowRight, Shield } from 'lucide-react';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, ReferenceLine,
  ScatterChart, Scatter, ZAxis, BarChart, Bar, Legend, Cell
} from 'recharts';
import { fetchBillingBreakdown, BILLING_BREAKDOWN_URL, type BillingBreakdown } from '../../data/services/metricsService';
import { motion } from 'framer-motion';
import { Link } from 'react-router-dom';
import { BentoCard } from '../components/ui/BentoCard';

// ── Types & Constants ────────────────────────────────────────────────────────

type Currency = 'USD' | 'INR' | 'EUR';

const CURRENCY_RATES: Record<Currency, number> = {
  USD: 1,
  INR: 83.5,
  EUR: 0.92,
};

// Obsidian Command Palette for charts
const CHART_COLORS = [
  'var(--accent-cyan)',
  'var(--primary-amber)',
  'var(--primary-rose)',
  '#8b5cf6', // Violet
  '#10b981', // Emerald
  '#3b82f6'  // Blue
];

const BUDGET_LIMIT_USD = 5000;

// ── Formatter & Styles ────────────────────────────────────────────────────────

const LABEL_CLASS = 'font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold';
const DATA_CLASS  = 'font-jetbrains text-[var(--on-surface)]';

// ── Components ───────────────────────────────────────────────────────────────

const NumberMetric = ({ value, prefix = '' }: { value: string | number, prefix?: string }) => (
  <span className={`${DATA_CLASS} text-4xl font-extrabold tracking-tighter`}>
    <span className="text-[var(--text-muted)] font-normal mr-1">{prefix}</span>{value}
  </span>
);

const BackgroundSparkline = ({ data, color }: { data: number[], color: string }) => {
  const chartData = data.map((v, i) => ({ val: v, index: i }));
  return (
    <div className="absolute inset-0 z-0 opacity-20 pointer-events-none translate-y-8">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData}>
          <defs>
            <linearGradient id={`grad-${color.replace(/[^a-zA-Z0-9]/g, '')}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={color} stopOpacity={0.8}/>
              <stop offset="95%" stopColor={color} stopOpacity={0}/>
            </linearGradient>
          </defs>
          <Area type="monotone" dataKey="val" stroke={color} fill={`url(#grad-${color.replace(/[^a-zA-Z0-9]/g, '')})`} strokeWidth={2} isAnimationActive={false} />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
};

// ── Framer Motion variants ────────────────────────────────────────────────────

const cardVariants = {
  hidden: { opacity: 0, y: 20, scale: 0.95 },
  visible: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.4, ease: [0.25, 0.1, 0.25, 1] as const } },
  hover: { scale: 1.02, transition: { duration: 0.2 } },
};

const containerVariants = {
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: { staggerChildren: 0.1, delayChildren: 0.1 } },
};

// ── Main View ────────────────────────────────────────────────────────────────

export function BillingView() {
  const [currency, setCurrency] = useState<Currency>('USD');

  const { data: breakdown, error, isLoading } = useSWR<BillingBreakdown[]>(
    BILLING_BREAKDOWN_URL,
    fetchBillingBreakdown,
    { refreshInterval: 60_000, errorRetryCount: 3 }
  );

  // Formatting
  const formatCurrency = (usdAmount: number, maxFraction: number = 2) => {
    const converted = usdAmount * CURRENCY_RATES[currency];
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: maxFraction,
    }).format(converted);
  };

  const getCurrencySymbol = () => {
    if (currency === 'USD') return '$';
    if (currency === 'INR') return '₹';
    return '€';
  };

  const budgetLimit = BUDGET_LIMIT_USD * CURRENCY_RATES[currency];

  // ── Derived Data ───────────────────────────────────────────────────────────

  const uniqueDates = useMemo(() => Array.from(new Set(breakdown?.map(r => r.date) || [])).sort(), [breakdown]);
  
  const totalCostUsd = useMemo(() => breakdown?.reduce((acc, row) => acc + row.estimated_cost, 0) || 0, [breakdown]);
  const dailyBurnRateUsd = uniqueDates.length ? totalCostUsd / uniqueDates.length : 0;
  const projectedMonthEndUsd = dailyBurnRateUsd * 30;

  // Cumulative Area Data
  const cumulativeData = useMemo(() => {
    if (!breakdown) return [];
    const dailySums = breakdown.reduce((acc, row) => {
      acc[row.date] = (acc[row.date] || 0) + row.estimated_cost * CURRENCY_RATES[currency];
      return acc;
    }, {} as Record<string, number>);

    let cumSum = 0;
    return uniqueDates.map(date => {
      cumSum += dailySums[date];
      return { date, cost: cumSum, daily: dailySums[date] };
    });
  }, [breakdown, uniqueDates, currency]);

  const maxCumulativeCost = cumulativeData.length ? cumulativeData[cumulativeData.length - 1].cost : 0;

  // Scatter Data (Cost vs Volume)
  const scatterData = useMemo(() => {
    if (!breakdown) return [];
    const modelStats = breakdown.reduce((acc, row) => {
      if (!acc[row.model]) acc[row.model] = { requests: 0, cost: 0, tokens: 0 };
      acc[row.model].requests += 1; // Approx 1 request batch per row
      acc[row.model].cost += row.estimated_cost * CURRENCY_RATES[currency];
      acc[row.model].tokens += row.total_tokens;
      return acc;
    }, {} as Record<string, { requests: number, cost: number, tokens: number }>);

    return Object.entries(modelStats).map(([model, stats]) => ({
      model,
      requests: stats.requests,
      cost: Number(stats.cost.toFixed(4)),
      avgTokens: Math.round(stats.tokens / stats.requests),
    }));
  }, [breakdown, currency]);

  // Stacked Bar Data
  const stackedBarData = useMemo(() => {
    if (!breakdown) return [];
    const grouped = breakdown.reduce((acc, row) => {
      if (!acc[row.date]) acc[row.date] = { date: row.date };
      acc[row.date][row.model] = row.estimated_cost * CURRENCY_RATES[currency];
      return acc;
    }, {} as Record<string, any>);
    return Object.values(grouped).sort((a, b) => a.date.localeCompare(b.date));
  }, [breakdown, currency]);

  const uniqueModels = useMemo(() => Array.from(new Set(breakdown?.map(r => r.model) || [])), [breakdown]);

  // Heatmap Data Array
  const heatmapArray = useMemo(() => {
    if (!breakdown) return [];
    const dailySums = breakdown.reduce((acc, row) => {
      acc[row.date] = (acc[row.date] || 0) + row.estimated_cost * CURRENCY_RATES[currency];
      return acc;
    }, {} as Record<string, number>);
    const maxDaily = Math.max(...Object.values(dailySums), 1);
    
    return uniqueDates.map(date => ({
      date,
      cost: dailySums[date],
      intensity: dailySums[date] / maxDaily
    }));
  }, [breakdown, uniqueDates, currency]);

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <motion.div
      variants={containerVariants}
      initial="hidden"
      animate="visible"
      className="grid grid-cols-12 gap-6 p-6 auto-rows-max text-[var(--on-surface)]"
    >
      {/* Breadcrumb (col-span-12) */}
      <motion.div variants={cardVariants} className="col-span-12 flex items-center gap-3 text-sm font-mono text-[var(--text-muted)] mb-2">
        <Link to="/dashboard" className="hover:text-[var(--on-surface)] transition-colors flex items-center gap-2 font-geist tracking-wide">
          <Shield className="w-4 h-4" />
          Dashboard
        </Link>
        <ArrowRight className="w-4 h-4" />
        <span className="text-[var(--on-surface)] font-geist">Cost &amp; Billing</span>
      </motion.div>

      {/* Header Section (col-span-12) */}
      <motion.div variants={cardVariants} className="col-span-12 flex flex-col md:flex-row md:items-end justify-between gap-6 mb-8">
        <div>
          <h1 className="text-5xl font-extrabold tracking-tight mb-4 text-[var(--on-surface)] font-geist">
            FinOps &amp; Telemetry
          </h1>
          <p className="text-sm text-[var(--text-muted)] max-w-2xl font-geist">
            Real-time inference spend analysis and budget enforcement.
          </p>
        </div>
        
        <div className="flex flex-col sm:flex-row items-end sm:items-center gap-4">
          {/* Currency Switcher */}
          <div className="flex p-1 rounded-full bg-[rgba(255,255,255,0.02)]">
            {(['USD', 'INR', 'EUR'] as Currency[]).map((c) => (
              <button
                key={c}
                onClick={() => setCurrency(c)}
                className={`relative px-6 py-2.5 rounded-full font-geist text-[10px] font-bold uppercase tracking-widest transition-all duration-300 ${
                  currency === c
                    ? 'text-[var(--on-surface)] bg-[var(--surface-bright)] shadow-md'
                    : 'text-[var(--text-muted)] hover:text-[var(--on-surface)]'
                }`}
              >
                {currency === c && (
                  <motion.div
                    layoutId="currency-tab"
                    className="absolute inset-0 rounded-full"
                    initial={false}
                    transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                    style={{ boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.06)' }}
                  />
                )}
                {c}
              </button>
            ))}
          </div>

          <div className="flex items-center gap-3">
            <button className="p-3 rounded-xl bg-[var(--surface-bright)] hover:bg-[rgba(255,255,255,0.1)] transition-all text-[var(--on-surface)]">
              <Download className="w-5 h-5" />
            </button>
            <button className="px-5 py-2.5 rounded-xl font-bold uppercase tracking-widest text-[10px] flex items-center gap-2 font-geist transition-all duration-150 active:translate-y-[2px] bg-[var(--surface-bright)] text-[var(--on-surface)] hover:bg-[rgba(255,255,255,0.1)] shadow-md">
              <Settings className="w-4 h-4" />
              Manage Billing
            </button>
          </div>
        </div>
      </motion.div>

      {error && !breakdown && (
        <motion.div variants={cardVariants} className="col-span-12 p-4 rounded-2xl bg-[rgba(244,63,94,0.1)] flex items-center gap-3">
          <AlertTriangle className="w-5 h-5 text-[var(--primary-rose)]" />
          <p className="text-sm font-medium text-[var(--on-surface)] font-geist">Connection to billing pipeline failed. Displaying local cache if available.</p>
        </motion.div>
      )}

      {/* 1. Hero Stats Row — Asymmetric 12-col grid */}
      <motion.div variants={cardVariants} className="col-span-12 lg:col-span-5 h-[180px]">
        <BentoCard glowColor="cyan" className="z-10 h-full p-6 flex flex-col justify-between">
          <div className="relative z-10 flex flex-col h-full">
            <div className="flex items-center gap-3 mb-4 text-[var(--accent-cyan)]">
              <CreditCard className="w-5 h-5" />
              <span className={`${LABEL_CLASS}`}>Current Spend</span>
            </div>
            {isLoading && !breakdown ? (
              <Loader2 className="w-8 h-8 animate-spin mt-2 text-[var(--accent-cyan)]" />
            ) : (
              <NumberMetric value={formatCurrency(totalCostUsd).replace(/[^0-9.,]/g, '')} prefix={getCurrencySymbol()} />
            )}
          </div>
          <BackgroundSparkline data={cumulativeData.map(d => d.daily)} color="var(--accent-cyan)" />
        </BentoCard>
      </motion.div>

      <motion.div variants={cardVariants} className="col-span-12 lg:col-span-3 h-[180px]">
        <BentoCard glowColor="amber" className="z-10 h-full p-6 flex flex-col justify-between">
          <div className="relative z-10 flex flex-col h-full">
            <div className="flex items-center gap-3 mb-4 text-[var(--primary-amber)]">
              <Activity className="w-5 h-5" />
              <span className={`${LABEL_CLASS}`}>Daily Burn Rate</span>
            </div>
            <NumberMetric value={formatCurrency(dailyBurnRateUsd).replace(/[^0-9.,]/g, '')} prefix={getCurrencySymbol()} />
          </div>
          <BackgroundSparkline data={cumulativeData.map(d => d.daily)} color="var(--primary-amber)" />
        </BentoCard>
      </motion.div>

      <motion.div variants={cardVariants} className="col-span-12 lg:col-span-4 h-[180px]">
        <BentoCard glowColor="rose" className="z-10 h-full p-6 flex flex-col justify-between">
          <div className="relative z-10 flex flex-col h-full">
            <div className="flex items-center gap-3 mb-4 text-[var(--primary-rose)]">
              <Activity className="w-5 h-5" />
              <span className={`${LABEL_CLASS}`}>Projected Month-End</span>
            </div>
            <NumberMetric value={formatCurrency(projectedMonthEndUsd).replace(/[^0-9.,]/g, '')} prefix={getCurrencySymbol()} />
          </div>
          <BackgroundSparkline data={cumulativeData.map(d => d.daily)} color="var(--primary-rose)" />
        </BentoCard>
      </motion.div>

      {/* 2 & 3: Cumulative Spend (col-8) and Cost vs Volume (col-4) */}
      <motion.div variants={cardVariants} className="col-span-12 lg:col-span-8 h-[400px]">
        {/* Cumulative Spend Area Chart */}
        <BentoCard glowColor="cyan" className="h-full p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-bold tracking-tight text-[var(--on-surface)] font-geist">Cumulative Spend vs Budget</h2>
            <div className="px-3 py-1 rounded-full bg-[rgba(255,255,255,0.05)] text-[10px] uppercase font-bold tracking-widest text-[var(--text-muted)] font-geist">
              Limit: <span className={`${DATA_CLASS}`}>{formatCurrency(BUDGET_LIMIT_USD)}</span>
            </div>
          </div>
          <div className="h-[280px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={cumulativeData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                <defs>
                  <linearGradient id="primaryGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="var(--accent-cyan)" stopOpacity={0.6} />
                    <stop offset="95%" stopColor="var(--accent-cyan)" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="strokeColor" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="var(--accent-cyan)" stopOpacity={1} />
                    <stop offset="100%" stopColor="var(--accent-cyan)" stopOpacity={1} />
                  </linearGradient>
                  <filter id="glow-area-chart" x="-20%" y="-20%" width="140%" height="140%">
                    <feDropShadow dx="0" dy="8" stdDeviation="6" floodColor="rgba(0,0,0,0.6)" />
                  </filter>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#ffffff" strokeOpacity={0.05} vertical={false} />
                <XAxis dataKey="date" stroke="#ffffff50" fontSize={12} tickLine={false} axisLine={false} tickMargin={10} className="font-jetbrains tabular-nums" />
                <YAxis 
                  stroke="#ffffff50" 
                  fontSize={12} 
                  tickLine={false} 
                  axisLine={false} 
                  tickFormatter={(val) => `${getCurrencySymbol()}${val}`}
                  className="font-jetbrains tabular-nums"
                />
                <Tooltip 
                  contentStyle={{ backgroundColor: 'var(--surface-container-low)', borderColor: 'transparent', boxShadow: maxCumulativeCost > budgetLimit ? '0 0 16px rgba(244,63,94,0.4)' : '0 4px 12px rgba(0,0,0,0.5)', borderRadius: '12px', color: 'var(--on-surface)', fontFamily: '"JetBrains Mono", monospace' }}
                  itemStyle={{ fontWeight: 600 }}
                  formatter={(value: any) => [formatCurrency(Number(value)), "Spend"]}
                />
                <ReferenceLine y={budgetLimit} stroke="var(--primary-rose)" strokeDasharray="5 5" label={{ position: 'top', value: 'Budget Limit', fill: 'var(--primary-rose)', fontSize: 10, fontFamily: '"JetBrains Mono", monospace' }} style={{ filter: maxCumulativeCost > budgetLimit ? 'drop-shadow(0 0 8px rgba(244,63,94,0.6))' : 'none' }} />
                <Area type="monotone" dataKey="cost" stroke="url(#strokeColor)" fill="url(#primaryGradient)" strokeWidth={3} isAnimationActive={false} style={{ filter: 'url(#glow-area-chart)' }} />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </BentoCard>
      </motion.div>

      <motion.div variants={cardVariants} className="col-span-12 lg:col-span-4 h-[400px]">
        {/* Cost vs Volume Scatter Bubble */}
        <BentoCard glowColor="amber" className="h-full p-6">
          <h2 className="text-xl font-bold tracking-tight text-[var(--on-surface)] mb-2 font-geist">Cost vs Request Volume</h2>
          <p className={`${LABEL_CLASS} mb-4 normal-case tracking-normal`}>Bubble size represents Avg Tokens per Request</p>
          <div className="h-[280px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <ScatterChart margin={{ top: 20, right: 20, bottom: 0, left: 0 }}>
                <defs>
                  <filter id="glow-scatter" x="-20%" y="-20%" width="140%" height="140%">
                    <feDropShadow dx="0" dy="8" stdDeviation="6" floodColor="rgba(0,0,0,0.6)" />
                  </filter>
                </defs>
                <CartesianGrid stroke="#ffffff" strokeOpacity={0.05} />
                <XAxis type="number" dataKey="requests" name="Requests" stroke="#ffffff50" fontSize={10} tickLine={false} axisLine={false} className="font-jetbrains tabular-nums" />
                <YAxis type="number" dataKey="cost" name="Cost" stroke="#ffffff50" fontSize={10} tickLine={false} axisLine={false} className="font-jetbrains tabular-nums" tickFormatter={(val) => `${val}`} />
                <ZAxis type="number" dataKey="avgTokens" range={[60, 400]} name="Avg Tokens" />
                <Tooltip cursor={{ strokeDasharray: '3 3' }}
                  contentStyle={{ backgroundColor: 'var(--surface-container-low)', borderColor: 'transparent', borderRadius: '12px', color: 'var(--on-surface)', fontFamily: '"JetBrains Mono", monospace' }}
                  formatter={(val, name) => name === 'Cost' ? formatCurrency(Number(val)) : val}
                />
                <Scatter name="Models" data={scatterData} style={{ filter: 'url(#glow-scatter)' }}>
                  {scatterData.map((entry, index) => {
                    const isHighCost = entry.cost > (totalCostUsd * 0.3); // arbitrarily define outlier color logic
                    return <Cell key={`cell-${index}`} fill={isHighCost ? 'var(--primary-amber)' : 'var(--accent-cyan)'} opacity={0.8} />;
                  })}
                </Scatter>
              </ScatterChart>
            </ResponsiveContainer>
          </div>
        </BentoCard>
      </motion.div>

      {/* 4 & 5: Bottom Row (Stacked Bar and Heatmap) */}
      <motion.div variants={cardVariants} className="col-span-12 lg:col-span-8 h-[350px]">
        {/* Cost Per Model Stacked Bar */}
        <BentoCard glowColor="none" className="h-full p-6">
          <h2 className="text-xl font-bold tracking-tight text-[var(--on-surface)] mb-6 font-geist">Spend Distribution per Model</h2>
          <div className="h-[240px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={stackedBarData} layout="horizontal" margin={{ top: 0, right: 0, left: 0, bottom: 0 }}>
                <defs>
                  <filter id="glow-bar" x="-20%" y="-20%" width="140%" height="140%">
                    <feDropShadow dx="0" dy="8" stdDeviation="6" floodColor="rgba(0,0,0,0.6)" />
                  </filter>
                </defs>
                <CartesianGrid vertical={false} stroke="none" horizontalFill={['var(--surface-lowest)', 'transparent']} fillOpacity={0.5} />
                <XAxis dataKey="date" stroke="#ffffff50" fontSize={10} tickLine={false} axisLine={false} className="font-jetbrains tabular-nums" />
                <YAxis 
                  stroke="#ffffff50" 
                  fontSize={10} 
                  tickLine={false} 
                  axisLine={false} 
                  className="font-jetbrains tabular-nums"
                  tickFormatter={(val) => `${val}`}
                />
                <Tooltip 
                  contentStyle={{ backgroundColor: 'var(--surface-container-low)', borderColor: 'transparent', borderRadius: '12px', color: 'var(--on-surface)', fontFamily: '"JetBrains Mono", monospace' }}
                  itemStyle={{ fontWeight: 600 }}
                  formatter={(value: any) => [formatCurrency(Number(value)), ""]}
                />
                <Legend iconType="circle" wrapperStyle={{ fontSize: '12px', fontFamily: '"JetBrains Mono", monospace', opacity: 0.8, paddingTop: '10px' }} />
                {uniqueModels.map((model, idx) => (
                  <Bar 
                    key={model} 
                    dataKey={model} 
                    name={model}
                    stackId="a" 
                    fill={CHART_COLORS[idx % CHART_COLORS.length]} 
                    animationDuration={1500}
                    style={{ filter: 'url(#glow-bar)' }}
                  />
                ))}
              </BarChart>
            </ResponsiveContainer>
          </div>
        </BentoCard>
      </motion.div>

      <motion.div variants={cardVariants} className="col-span-12 lg:col-span-4 h-[350px]">
        {/* Daily Burn Rate Heatmap */}
        <BentoCard glowColor="amber" className="h-full p-6 flex flex-col">
          <h2 className="text-xl font-bold tracking-tight text-[var(--on-surface)] mb-2 font-geist">Daily Burn Heatmap</h2>
          <p className={`${LABEL_CLASS} mb-6 normal-case tracking-normal`}>Intensity correlates with daily spend</p>
          
          <div className="flex-1 flex items-center justify-center">
            <div className="flex flex-wrap gap-2 justify-center max-w-[280px]">
              {heatmapArray.map((day, i) => {
                const isActive = day.intensity >= 0.2;
                return (
                  <div 
                    key={i} 
                    className="w-8 h-8 rounded-[2px] transition-all hover:scale-110 hover:z-10 group relative"
                    style={{ 
                      backgroundColor: isActive ? 'var(--primary-amber)' : 'var(--surface-container)',
                      opacity: isActive ? Math.max(0.3, day.intensity) : 0.2,
                      filter: isActive && day.intensity > 0.8 ? 'drop-shadow(0 0 4px rgba(245,158,11,0.5))' : 'none'
                    }}
                  >
                    {/* Tooltip on hover */}
                    <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden group-hover:block z-50">
                      <div className="bg-[var(--surface-container-lowest)] text-[var(--on-surface)] text-xs font-jetbrains tabular-nums px-3 py-1.5 rounded-lg whitespace-nowrap shadow-xl">
                        <div>{day.date}</div>
                        <div className="text-[var(--primary-amber)] font-bold">{formatCurrency(day.cost)}</div>
                      </div>
                    </div>
                  </div>
                );
              })}
              {heatmapArray.length === 0 && !isLoading && (
                <div className={`${DATA_CLASS} text-[var(--text-muted)] text-sm`}>No daily data available.</div>
              )}
              {isLoading && heatmapArray.length === 0 && (
                <Loader2 className="w-6 h-6 animate-spin text-[var(--primary-amber)]" />
              )}
            </div>
          </div>
        </BentoCard>
      </motion.div>

    </motion.div>
  );
}
