import { useEffect, useState } from 'react';
import {
  Area,
  AreaChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
  CartesianGrid
} from 'recharts';

interface HotSwapMetric {
  time: string;
  openai_success: number;
  openai_error: number;
  anthropic_fallback: number;
}

export const HotSwapLiveChart = () => {
  const [data, setData] = useState<HotSwapMetric[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  // Use the established gateway URL (defaulting to localhost:8080 as per other services)
  const HOT_SWAPS_URL = 'http://localhost:8080/v1/admin/metrics/hot-swaps';

  const fetchData = async () => {
    try {
      const token = localStorage.getItem('re_token');
      if (!token) throw new Error('No authentication token found. Please log in.');

      const response = await fetch(HOT_SWAPS_URL, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`Failed to fetch hot-swaps data: ${response.statusText}`);
      }

      const json = await response.json() as HotSwapMetric[];
      setData(json);
      setError(null);
    } catch (err: any) {
      setError(err.message || 'An unknown error occurred.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // Initial fetch
    fetchData();

    // Poll every 2 seconds
    const interval = setInterval(fetchData, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="w-full h-96 relative flex flex-col bg-gray-900 border border-gray-800 rounded-lg p-6 overflow-hidden">
      <div className="flex justify-between items-center mb-6 z-10">
        <div>
          <h3 className="text-xl font-semibold text-gray-100 uppercase tracking-wide">
            Zero-Downtime Hot-Swaps
          </h3>
          <p className="text-sm text-gray-400 mt-1">
            Real-time multi-LLM routing telemetry (polling 2s)
          </p>
        </div>
        <div className="flex items-center space-x-2">
          {error && <span className="text-xs text-red-400 font-medium">Offline</span>}
          {loading && !error && <span className="text-xs text-gray-400 font-medium animate-pulse">Connecting...</span>}
          {!error && !loading && (
            <div className="flex items-center space-x-1">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
              </span>
              <span className="text-xs text-emerald-400 font-medium tracking-wider">Live</span>
            </div>
          )}
        </div>
      </div>

      <div className="flex-grow w-full h-full min-h-[250px] relative z-0">
        {data.length === 0 && !loading && !error && (
          <div className="absolute inset-0 flex items-center justify-center text-gray-500 italic z-10 pointer-events-none">
            No telemetry data available for the last hour.
          </div>
        )}

        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
            <defs>
              <linearGradient id="colorSuccess" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.4} />
                <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="colorFallback" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#10b981" stopOpacity={0.5} />
                <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="colorError" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#ef4444" stopOpacity={0.6} />
                <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" vertical={false} />
            <XAxis
              dataKey="time"
              stroke="#9ca3af"
              fontSize={12}
              tickLine={false}
              axisLine={false}
              minTickGap={20}
            />
            <YAxis 
              stroke="#9ca3af" 
              fontSize={12} 
              tickLine={false} 
              axisLine={false} 
              tickFormatter={(value) => `${value}`}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: '#111827',
                border: '1px solid #374151',
                borderRadius: '8px',
                color: '#f3f4f6',
                boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.5)',
              }}
              itemStyle={{ fontWeight: 500 }}
              labelStyle={{ color: '#9ca3af', marginBottom: '8px', fontWeight: 600 }}
            />
            <Area
              type="monotone"
              dataKey="openai_success"
              name="OpenAI (Normal)"
              stroke="#3b82f6" // Solid Blue
              strokeWidth={2}
              fillOpacity={1}
              fill="url(#colorSuccess)"
              isAnimationActive={false}
            />
            <Area
              type="monotone"
              dataKey="anthropic_fallback"
              name="Anthropic (Hot-Swap)"
              stroke="#10b981" // Bright Green
              strokeWidth={2.5}
              fillOpacity={1}
              fill="url(#colorFallback)"
              isAnimationActive={false}
            />
            <Area
              type="monotone"
              dataKey="openai_error"
              name="OpenAI (Failed)"
              stroke="#ef4444" // Bright Red
              strokeWidth={2}
              fillOpacity={1}
              fill="url(#colorError)"
              isAnimationActive={false}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      <div className="mt-4 flex flex-wrap gap-4 text-xs font-medium text-gray-400 shrink-0 z-10">
        <div className="flex items-center gap-2 bg-gray-800/50 rounded-full px-3 py-1 border border-gray-700/50">
          <div className="w-2.5 h-2.5 rounded-full bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.6)]"></div>
          OpenAI (Normal)
        </div>
        <div className="flex items-center gap-2 bg-gray-800/50 rounded-full px-3 py-1 border border-gray-700/50">
          <div className="w-2.5 h-2.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.6)]"></div>
          Anthropic (Hot-Swapped)
        </div>
        <div className="flex items-center gap-2 bg-gray-800/50 rounded-full px-3 py-1 border border-gray-700/50">
          <div className="w-2.5 h-2.5 rounded-full bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.6)]"></div>
          OpenAI (Failed)
        </div>
      </div>
    </div>
  );
};

export default HotSwapLiveChart;
