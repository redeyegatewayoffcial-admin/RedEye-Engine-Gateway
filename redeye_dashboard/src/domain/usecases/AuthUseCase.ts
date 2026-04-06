// Domain Use-Case — Authentication
// Defines the contract; implementations live in data/services.

import type { User } from '../entities/User';

export interface IAuthUseCase {
  requestMagicLink(email: string): Promise<void>;
  verifyMagicLink(email: string, otp: string): Promise<User>;
  ssoRedirect?(provider: string): Promise<void>;
  completeOnboarding(userId: string, workspaceName: string, provider: string, apiKey: string, accountType?: 'individual' | 'team'): Promise<User>;
}
