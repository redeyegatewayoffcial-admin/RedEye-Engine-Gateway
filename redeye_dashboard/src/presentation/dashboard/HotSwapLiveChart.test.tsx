import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { HotSwapLiveChart } from './HotSwapLiveChart';

// Mock Recharts components globally
vi.mock('recharts', () => ({
  ResponsiveContainer: ({ children }: any) => <div data-testid="responsive-container">{children}</div>,
  AreaChart: ({ children, data }: any) => <div data-testid="area-chart" data-points={data?.length}>{children}</div>,
  Area: () => <div data-testid="area" />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
  CartesianGrid: () => <div data-testid="cartesian-grid" />,
}));

describe('HotSwapLiveChart Component', () => {
  it('renders chart title and subheader', () => {
    render(<HotSwapLiveChart />);
    expect(screen.getByText(/Zero-Downtime Hot-Swaps/i)).toBeInTheDocument();
    expect(screen.getByText(/Real-time multi-LLM routing telemetry/i)).toBeInTheDocument();
  });

  it('renders mocked recharts components without actual SVGs', () => {
    render(<HotSwapLiveChart />);
    expect(screen.getByTestId('responsive-container')).toBeInTheDocument();
    expect(screen.getByTestId('area-chart')).toBeInTheDocument();
  });
});
