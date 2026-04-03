// Dashboard View — BillingView
// Renders daily cost breakdown, model usage, and a currency switcher.
// Theme: slate-950/indigo-500.

import { useState, useMemo } from 'react';
import useSWR from 'swr';
import { CreditCard, Loader2, AlertCircle } from 'lucide-react';
import { StatCard } from '../components/ui/StatCard';
import {
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend
} from 'recharts';
import { fetchBillingBreakdown, BILLING_BREAKDOWN_URL, type BillingBreakdown } from '../../data/services/metricsService';

type Currency = 'USD' | 'INR' | 'EUR';

const CURRENCY_RATES: Record<Currency, number> = {
  USD: 1,
  INR: 83.5, // Approx INR to USD
  EUR: 0.92, // Approx EUR to USD
};

const CHART_COLORS = ['#6366f1', '#a855f7', '#06b6d4', '#ec4899', '#f59e0b', '#14b8a6'];

export function BillingView() {
  const [currency, setCurrency] = useState<Currency>('USD');

  const { data: breakdown, error, isLoading } = useSWR<BillingBreakdown[]>(
    BILLING_BREAKDOWN_URL,
    fetchBillingBreakdown,
    { refreshInterval: 60_000, errorRetryCount: 3 }
  );

  // Currency formatter bounded to selected currency
  const formatCurrency = (usdAmount: number) => {
    const converted = usdAmount * CURRENCY_RATES[currency];
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: 4,
    }).format(converted);
  };

  // derived state for charts & summary
  const totalCostUsd = useMemo(() => {
    if (!breakdown) return 0;
    return breakdown.reduce((acc, row) => acc + row.estimated_cost, 0);
  }, [breakdown]);

  // Transform flat array into clustered array by Date, e.g., { date: '2026-03-24', 'llama-3': 0.05, 'gpt-4': 0.1 }
  const chartData = useMemo(() => {
    if (!breakdown) return [];
    const grouped = breakdown.reduce((acc, row) => {
      if (!acc[row.date]) acc[row.date] = { date: row.date };
      // store converted cost in the chart dataset directly so recharts tooltip sees it natively
      acc[row.date][row.model] = row.estimated_cost * CURRENCY_RATES[currency];
      return acc;
    }, {} as Record<string, any>);
    
    // Sort chronologically
    return Object.values(grouped).sort((a, b) => a.date.localeCompare(b.date));
  }, [breakdown, currency]);

  // Extract unique models for Bar keys
  const models = useMemo(() => {
    if (!breakdown) return [];
    return Array.from(new Set(breakdown.map((r) => r.model)));
  }, [breakdown]);

  return (
    <div className="space-y-6 animate-in fade-in duration-500">
      {/* Header and Controls */}
      <header className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-emerald-600 to-teal-500 dark:from-emerald-400 dark:to-teal-200 bg-clip-text text-transparent pb-1 flex items-center gap-3">
            Cost &amp; Billing
          </h1>
          <p className="text-xs sm:text-sm text-slate-500 dark:text-slate-400 mt-1">Real-time inference spend analysis</p>
        </div>
        
        <div className="flex items-center gap-4">
          {/* Currency Switcher */}
          <div className="flex bg-slate-100 dark:bg-slate-900/80 border border-slate-200 dark:border-slate-700 p-1 rounded-lg">
            {(['USD', 'INR', 'EUR'] as Currency[]).map((c) => (
              <button
                key={c}
                onClick={() => setCurrency(c)}
                className={`px-3 py-1 sm:px-4 sm:py-1.5 text-xs sm:text-sm font-medium rounded-md transition-all duration-200 ease-in-out active:scale-95 focus:outline-none focus:ring-2 focus:ring-indigo-500/50 ${
                  currency === c
                    ? 'bg-indigo-600 text-white shadow-md hover:shadow-lg'
                    : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-white dark:hover:bg-slate-800'
                }`}
              >
                {c}
              </button>
            ))}
          </div>

          <div className="flex items-center space-x-2 glass-panel bg-white/80 dark:bg-slate-900/50 border border-slate-200/60 dark:border-slate-700/50 px-3 py-1.5 sm:px-4 sm:py-2 rounded-full self-start sm:self-auto w-fit shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            {isLoading && !breakdown ? (
              <Loader2 className="w-4 h-4 text-emerald-500 dark:text-emerald-400 animate-spin" />
            ) : (
              <div className={`w-2 h-2 sm:w-3 sm:h-3 rounded-full ${error ? 'bg-rose-500' : 'bg-emerald-500 neon-dot'}`} />
            )}
            <span className="text-xs sm:text-sm font-medium text-slate-600 dark:text-slate-300 font-display">
              {isLoading && !breakdown ? 'Syncing...' : error ? 'Offline' : 'Live'}
            </span>
          </div>
        </div>
      </header>

      {/* Summary Row */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 sm:gap-4">
        <div className="sm:col-span-1">
          <StatCard
            title="Total Spend (Month)"
            value={(isLoading && !breakdown) ? '...' : formatCurrency(totalCostUsd)}
            icon={CreditCard}
            accentClass="text-emerald-400 ring-1 ring-emerald-400/20"
          />
        </div>
      </div>

      {error && !breakdown && (
        <div className="glass-panel bg-rose-50 dark:bg-rose-500/5 border border-rose-200 dark:border-rose-500/20 p-4 flex items-center gap-3 shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
          <AlertCircle className="w-5 h-5 text-rose-500" />
          <p className="text-sm text-rose-700 dark:text-slate-300">Connection to billing pipeline failed. Displaying local cache if available.</p>
        </div>
      )}

      {/* Bar Chart */}
      <div className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-800/80 p-4 sm:p-6 shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
        <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-6">Daily Cost Breakdown</h2>
        <div className="h-[300px] w-full min-h-[300px]">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e2e8f0" vertical={false} />
              <XAxis dataKey="date" stroke="#64748b" fontSize={10} tickLine={false} axisLine={false} />
              <YAxis 
                stroke="#64748b" 
                fontSize={10} 
                tickLine={false} 
                axisLine={false} 
                tickFormatter={(val) => `${currency === 'USD' ? '$' : currency === 'INR' ? '₹' : '€'}${val}`}
              />
              <Tooltip 
                contentStyle={{ backgroundColor: '#0f172a', borderColor: '#1e293b', borderRadius: '8px', fontSize: '13px', color: '#f1f5f9' }}
                itemStyle={{ fontWeight: 600 }}
                formatter={(value: any) => [
                  new Intl.NumberFormat('en-US', { style: 'currency', currency }).format(Number(value) || 0), 
                  ""
                ]}
              />
              <Legend iconType="circle" wrapperStyle={{ fontSize: '12px', paddingTop: '10px' }} />
              {models.map((model, idx) => (
                <Bar 
                  key={model} 
                  dataKey={model} 
                  name={model}
                  stackId="a" 
                  fill={CHART_COLORS[idx % CHART_COLORS.length]} 
                  animationDuration={1500}
                />
              ))}
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Breakdown Table */}
      <div className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-800/80 p-4 sm:p-6 overflow-hidden shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
        <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-4">Detailed Ledger</h2>
        <div className="overflow-x-auto w-full border border-slate-200 dark:border-slate-800/50 rounded-lg bg-slate-50 dark:bg-slate-950/40">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-slate-200 dark:border-slate-800 text-xs sm:text-sm font-semibold text-slate-500 dark:text-slate-400 bg-slate-100 dark:bg-slate-900/50">
                <th className="p-3">Date</th>
                <th className="p-3">AI Model</th>
                <th className="p-3 text-right">Tokens Processed</th>
                <th className="p-3 text-right">Cost ({currency})</th>
              </tr>
            </thead>
            <tbody className="text-xs sm:text-sm text-slate-700 dark:text-slate-300 divide-y divide-slate-100 dark:divide-slate-800">
              {!breakdown && isLoading ? (
                <tr>
                  <td colSpan={4} className="p-6 text-center text-slate-500">
                    <Loader2 className="w-6 h-6 animate-spin mx-auto mb-2" />
                    Loading ledger records...
                  </td>
                </tr>
              ) : breakdown?.length === 0 ? (
                <tr>
                  <td colSpan={4} className="p-6 text-center text-slate-500">No billing records found.</td>
                </tr>
              ) : (
                breakdown?.map((row, idx) => (
                  <tr key={`${row.date}-${row.model}-${idx}`} className="hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors">
                    <td className="p-3 font-mono text-slate-500 dark:text-slate-400">{row.date}</td>
                    <td className="p-3">
                      <span className="px-2 py-0.5 rounded text-xs font-medium bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-slate-700">
                        {row.model}
                      </span>
                    </td>
                    <td className="p-3 text-right font-mono">{row.total_tokens.toLocaleString('en-US')}</td>
                    <td className="p-3 text-right font-medium text-emerald-600 dark:text-emerald-400">{formatCurrency(row.estimated_cost)}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
