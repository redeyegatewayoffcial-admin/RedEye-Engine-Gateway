// Data Service — Auth API calls to RedEye Gateway
// Implements IAuthUseCase against http://localhost:8080/v1/auth

import type { User } from '../../domain/entities/User';
import type {
  IAuthUseCase,
  LoginPayload,
  SignupPayload,
} from '../../domain/usecases/AuthUseCase';

export interface IAuthUseCaseExtended extends IAuthUseCase {
  refreshToken(): Promise<User | null>;
}

const BASE_URL = 'http://localhost:8084/v1/auth';

// Shape expected from the backend on login/signup/onboard
interface AuthResponse {
  id: string; // user id
  email: string;
  tenant_id: string;
  workspace_name: string;
  onboarding_complete: boolean;
  token: string;
  redeye_api_key?: string;
}

function mapUser(resp: AuthResponse): User {
  return {
    id: resp.id,
    email: resp.email,
    workspaceName: resp.workspace_name ?? '',
    openAiApiKey: '', // We don't hold this client-side directly
    onboardingComplete: resp.onboarding_complete ?? false,
  };
}

async function postJson<T>(url: string, body: unknown): Promise<T> {
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new Error(text || `HTTP ${res.status}`);
  }

  return res.json() as Promise<T>;
}

export const authService: IAuthUseCaseExtended = {
  async login({ email, password }: LoginPayload): Promise<User> {
    const data = await postJson<AuthResponse>(`${BASE_URL}/login`, {
      email,
      password,
    });
    if (data.token) {
      localStorage.setItem('re_token', data.token);
    }
    return mapUser(data);
  },

  async signup({ email, password, companyName = 'My Company' }: SignupPayload & { companyName?: string }): Promise<User> {
    const data = await postJson<AuthResponse>(`${BASE_URL}/signup`, {
      email,
      password,
      company_name: companyName,
    });
    if (data.token) {
      localStorage.setItem('re_token', data.token);
    }
    return mapUser(data);
  },

  async completeOnboarding(
    _userId: string,
    workspaceName: string,
    openAiApiKey: string,
  ): Promise<User> {
    const token = localStorage.getItem('re_token') || '';
    const res = await fetch(`${BASE_URL}/onboard`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`
      },
      body: JSON.stringify({
        openai_api_key: openAiApiKey,
        workspace_name: workspaceName,
      })
    });
    
    if (!res.ok) {
       throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json() as AuthResponse;
    if (data.token) {
      localStorage.setItem('re_token', data.token);
    }
    return mapUser(data);
  },

  async refreshToken(): Promise<User | null> {
    try {
      const res = await fetch(`${BASE_URL}/refresh`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include' // Sent explicitly since it is an HttpOnly token.
      });

      if (!res.ok) return null;
      
      const data = await res.json() as AuthResponse;
      if (data.token) {
        localStorage.setItem('re_token', data.token);
        return mapUser(data);
      }
      return null;
    } catch (e) {
      return null;
    }
  }
};
