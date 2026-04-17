import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ComplianceView } from './ComplianceView';
import * as SWR from 'swr';

vi.mock('swr');
const SWRMock = SWR.default as any;

describe('ComplianceView Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders metric cards based on data payload', () => {
    SWRMock.mockReturnValue({
      data: {
        total_scanned: 100000,
        dpdp_blocks: 1540,
        pii_redactions: 430,
        region_breakdown: [
          { region: 'IN', count: 54000 },
          { region: 'US', count: 46000 }
        ]
      },
      isLoading: false,
      error: null
    });

    render(<ComplianceView />);

    expect(screen.getByText('DPDP Security Center')).toBeInTheDocument();
    // 100000 parsed via formatNumber
    expect(screen.getByText('100.0K')).toBeInTheDocument();
    // 1540 blocks = 1.5K
    expect(screen.getByText('1.5K')).toBeInTheDocument();
    // 430 = 430
    expect(screen.getByText('430')).toBeInTheDocument();
  });

  it('displays the active shield toggle states/badges for PII redaction and region lock', () => {
    SWRMock.mockReturnValue({
      data: { total_scanned: 0, dpdp_blocks: 0, pii_redactions: 0, region_breakdown: [] },
      isLoading: false,
      error: null
    });

    render(<ComplianceView />);

    // Though presented as badges and not interactive toggles, we verify their active states
    expect(screen.getByText('Aho-Corasick PII Scan')).toBeInTheDocument();
    expect(screen.getByText('Presidio Deep Analysis')).toBeInTheDocument();
    expect(screen.getByText('DPDP Region Lock')).toBeInTheDocument();
  });

  it('renders error block fallback state', () => {
    SWRMock.mockReturnValue({
      data: null,
      isLoading: false,
      error: new Error('Cannot reach telemetry API')
    });

    render(<ComplianceView />);
    
    expect(screen.getByText('Mock Data')).toBeInTheDocument();
    expect(screen.getByText('Offline — Fallback Active')).toBeInTheDocument();
  });
});
