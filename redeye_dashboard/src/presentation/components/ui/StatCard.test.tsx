import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { StatCard } from './StatCard';
import { Shield } from 'lucide-react';

describe('StatCard Component', () => {
  it('renders title and value correctly', () => {
    render(<StatCard title="Total Requests" value="10k" icon={Shield} />);
    expect(screen.getByText('Total Requests')).toBeInTheDocument();
    expect(screen.getByText('10k')).toBeInTheDocument();
  });

  it('renders the subtitle when provided', () => {
    render(<StatCard title="Latency" value="45ms" subtitle="Global Avg" icon={Shield} />);
    expect(screen.getByText('Global Avg')).toBeInTheDocument();
  });

  it('applies the optional accent class', () => {
    const { container } = render(
      <StatCard title="Active" value="1" icon={Shield} accentClass="text-rose-500" />
    );
    // Find the badge container which should have the accent class
    const iconContainer = container.querySelector('.text-rose-500');
    expect(iconContainer).toBeInTheDocument();
  });
});
