// Domain Entity — User
// Represents an authenticated workspace operator.

export interface User {
  id: string;
  email: string;
  workspaceName: string;
  openAiApiKey: string;
  onboardingComplete: boolean;
}
