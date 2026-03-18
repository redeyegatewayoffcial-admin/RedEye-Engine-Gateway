// Domain Use-Case — Authentication
// Defines the contract; implementations live in data/services.

import type { User } from '../entities/User';

export interface LoginPayload {
  email: string;
  password: string;
}

export interface SignupPayload {
  email: string;
  password: string;
}

export interface IAuthUseCase {
  login(payload: LoginPayload): Promise<User>;
  signup(payload: SignupPayload): Promise<User>;
  completeOnboarding(userId: string, workspaceName: string, openAiApiKey: string): Promise<User>;
}
