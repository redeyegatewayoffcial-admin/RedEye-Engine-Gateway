// Domain Entity — User
// Represents an authenticated workspace operator.

export interface User {
  id: string;
  email: string;
  password?: string;
  authProvider: string;
  providerId?: string;
  tenantId: string;
  workspaceName: string;
  openAiApiKey: string;
  onboardingComplete: boolean;
  redeyeApiKey?: string;
}
