import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ApiKeysView } from './ApiKeysView';
import { MemoryRouter } from 'react-router-dom';
import * as SWR from 'swr';
import * as AuthContext from '../context/AuthContext';

vi.mock('swr');
vi.mock('../context/AuthContext');

const SWRMock = SWR.default as any;
const AuthMock = AuthContext.useAuth as any;

describe('ApiKeysView Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    AuthMock.mockReturnValue({ user: { accountType: 'team', redeyeApiKey: 'user-key-123' } });

    SWRMock.mockImplementation((key: string) => {
      if (key?.includes('api-keys')) {
        return {
          data: [{ id: 'key1', name: 'Prod Key', key_hash: 'hashy', created_at: '2026', status: 'Active' }],
          isLoading: false,
          error: null
        };
      }
      if (key?.includes('provider-keys')) {
        return {
          data: [{ id: 'p1', provider_name: 'openai', created_at: '2026' }],
          isLoading: false,
          error: null
        };
      }
      return { data: null, isLoading: false, error: null };
    });
  });

  it('renders provider and virtual API keys for a team account', async () => {
    render(
      <MemoryRouter>
        <ApiKeysView />
      </MemoryRouter>
    );

    expect(screen.getByText('API Keys & Providers')).toBeInTheDocument();
    expect(await screen.findByText('Prod Key')).toBeInTheDocument();
    expect(await screen.findByText('openai')).toBeInTheDocument();
  });

  it('opens and closes the module to generate a new key', async () => {
    render(
      <MemoryRouter>
        <ApiKeysView />
      </MemoryRouter>
    );

    const generateBtn = screen.getByRole('button', { name: /Generate Key/i });
    fireEvent.click(generateBtn);

    expect(await screen.findByText('Generate Virtual API Key')).toBeInTheDocument();

    const input = screen.getByPlaceholderText('e.g. Production Frontend App');
    fireEvent.change(input, { target: { value: 'New Test Key' } });

    // Mock alert since it's not implemented yet
    vi.spyOn(window, 'alert').mockImplementation(() => {});
    const buttons = screen.getAllByRole('button', { name: 'Generate Key' });
    fireEvent.click(buttons[buttons.length - 1]);

    await waitFor(() => {
      expect(window.alert).toHaveBeenCalledWith('Generate endpoint not yet implemented');
    });
  });
});
