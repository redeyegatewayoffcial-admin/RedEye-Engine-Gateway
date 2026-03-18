// Data Service — Auth API calls to RedEye Gateway
// Implements IAuthUseCase against http://localhost:8080/v1/auth

import type { User } from '../../domain/entities/User';
import type {
  IAuthUseCase,
  LoginPayload,
  SignupPayload,
} from '../../domain/usecases/AuthUseCase';

const BASE_URL = 'http://localhost:8080/v1/auth';

// Shape expected from the gateway on login/signup
interface AuthResponse {
  id: string;
  email: string;
  workspace_name: string;
  openai_api_key: string;
  onboarding_complete: boolean;
  token: string;
}

function mapUser(resp: AuthResponse): User {
  return {
    id: resp.id,
    email: resp.email,
    workspaceName: resp.workspace_name ?? '',
    openAiApiKey: resp.openai_api_key ?? '',
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

export const authService: IAuthUseCase = {
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

  async signup({ email, password }: SignupPayload): Promise<User> {
    const data = await postJson<AuthResponse>(`${BASE_URL}/signup`, {
      email,
      password,
    });
    if (data.token) {
      localStorage.setItem('re_token', data.token);
    }
    return mapUser(data);
  },

  async completeOnboarding(
    userId: string,
    workspaceName: string,
    openAiApiKey: string,
  ): Promise<User> {
    const data = await postJson<AuthResponse>(`${BASE_URL}/onboarding`, {
      user_id: userId,
      workspace_name: workspaceName,
      openai_api_key: openAiApiKey,
    });
    return mapUser(data);
  },
};
