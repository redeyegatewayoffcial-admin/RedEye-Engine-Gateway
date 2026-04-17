import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { TracesView } from './TracesView';
import * as SWR from 'swr';

vi.mock('swr');
const SWRMock = SWR.default as any;

describe('TracesView Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders trace data payload correctly in table', async () => {
    SWRMock.mockReturnValue({
      data: [
        {
          traceId: 'tr-001',
          tenantId: 'tn-999',
          model: 'gpt-4',
          tokens: 1540,
          latency: '240ms',
          policy: 'Allowed',
        }
      ],
      isLoading: false,
      error: null
    });

    render(<TracesView />);

    expect(screen.getByText('Trace Explorer')).toBeInTheDocument();
    expect(screen.getByText('tr-001')).toBeInTheDocument();
    expect(screen.getByText('1540')).toBeInTheDocument(); // Tokens
    expect(screen.getByText('240ms')).toBeInTheDocument(); // Latency
    expect(screen.getByText('ALLOWED')).toBeInTheDocument(); // Policy formatted uppercase
  });

  it('handles loading state', () => {
    SWRMock.mockReturnValue({ data: null, isLoading: true, error: null });
    render(<TracesView />);
    expect(screen.getByText('Monitoring...')).toBeInTheDocument();
  });

  it('handles error state gracefully', () => {
    SWRMock.mockReturnValue({ data: null, isLoading: false, error: new Error('Failed to fetch') });
    render(<TracesView />);
    expect(screen.getByText('Failed to fetch live traces. Ensure the gateway service is reachable.')).toBeInTheDocument();
  });
});
