// Data Service — Auth API calls to RedEye Gateway
// Implements IAuthUseCase against http://localhost:8080/v1/auth
// NOTE: All authenticated requests use credentials: 'include' to send HttpOnly cookies automatically.

import { parseApiError, type StandardizedError } from '../utils/apiErrors';
import type { User } from '../../domain/entities/User';
import type { IAuthUseCase } from '../../domain/usecases/AuthUseCase';

export interface IAuthUseCaseExtended extends IAuthUseCase {
  refreshToken(): Promise<User | null>;
}

const BASE_URL = 'http://localhost:8084/v1/auth';

export { type StandardizedError };

// Shape expected from the backend on login/signup/onboard
interface AuthResponse {
  id: string; // user id
  email: string;
  tenant_id: string;
  auth_provider?: string;
  provider_id?: string;
  workspace_name: string;
  onboarding_complete: boolean;
  token?: string; // Legacy field - now sent via HttpOnly cookie
  redeye_api_key?: string;
  account_type?: 'individual' | 'team';
}

function mapUser(resp: AuthResponse): User {
  return {
    id: resp.id,
    email: resp.email,
    authProvider: resp.auth_provider ?? 'email_otp',
    providerId: resp.provider_id,
    tenantId: resp.tenant_id,
    workspaceName: resp.workspace_name ?? '',
    openAiApiKey: '', // We don't hold this client-side directly
    onboardingComplete: resp.onboarding_complete ?? false,
    redeyeApiKey: resp.redeye_api_key,
    accountType: resp.account_type ?? 'individual',
  };
}

async function postJson<T>(url: string, body: unknown, includeCredentials = false): Promise<T> {
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
    credentials: includeCredentials ? 'include' : 'same-origin',
  });

  if (!res.ok) {
    const error = await parseApiError(res);
    throw error;
  }

  return res.json() as Promise<T>;
}

export const authService: IAuthUseCaseExtended = {
  async requestMagicLink(email: string): Promise<void> {
    await postJson(`${BASE_URL}/otp/request`, { email });
  },

  async verifyMagicLink(email: string, otp: string): Promise<User> {
    const data = await postJson<AuthResponse>(`${BASE_URL}/otp/verify`, {
      email,
      otp_code: otp,
    }, true); // credentials: 'include' to receive HttpOnly cookies
    // Token is now set as HttpOnly cookie by backend - no localStorage needed
    return mapUser(data);
  },

  async ssoRedirect(provider: string): Promise<void> {
    window.location.href = `${BASE_URL}/${provider}/login`;
  },

  async completeOnboarding(
    _userId: string,
    workspaceName: string,
    provider: string,
    apiKey: string,
    accountType?: 'individual' | 'team'
  ): Promise<User> {
    const res = await fetch(`${BASE_URL}/onboard`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({
        account_type: accountType,
        provider: provider,
        api_key: apiKey,
        workspace_name: workspaceName,
      })
    });
    
    if (!res.ok) {
      const error = await parseApiError(res);
      throw error;
    }
    const data = await res.json() as AuthResponse;
    // Token is now set as HttpOnly cookie by backend - no localStorage needed
    return mapUser(data);
  },

  async refreshToken(): Promise<User | null> {
    try {
      const res = await fetch(`${BASE_URL}/refresh`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include' // Sends HttpOnly refresh_token cookie automatically
      });

      if (!res.ok) {
        // Parse error but return null for silent failures
        await parseApiError(res);
        return null;
      }
      
      const data = await res.json() as AuthResponse;
      // New JWT and refresh token are set as HttpOnly cookies by backend
      // Backend implements refresh token rotation for security
      return mapUser(data);
    } catch (e) {
      return null;
    }
  },

};
