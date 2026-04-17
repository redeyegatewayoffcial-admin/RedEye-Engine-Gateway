import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { Badge } from './Badge';

describe('Badge Component', () => {
  it('renders children correctly', () => {
    render(<Badge>Test Badge</Badge>);
    expect(screen.getByText('Test Badge')).toBeInTheDocument();
  });

  it('applies default neutral variant classes', () => {
    render(<Badge>Neutral</Badge>);
    const badge = screen.getByText('Neutral');
    expect(badge).toHaveClass('bg-slate-700/40');
  });

  it('applies success variant classes', () => {
    render(<Badge variant="success">Active</Badge>);
    const badge = screen.getByText('Active');
    expect(badge).toHaveClass('bg-emerald-500/10', 'text-emerald-400');
  });

  it('applies danger variant classes', () => {
    render(<Badge variant="danger">Offline</Badge>);
    const badge = screen.getByText('Offline');
    expect(badge).toHaveClass('bg-rose-500/10', 'text-rose-400');
  });

  it('applies custom className', () => {
    render(<Badge className="custom-class">Custom</Badge>);
    expect(screen.getByText('Custom')).toHaveClass('custom-class');
  });
});
