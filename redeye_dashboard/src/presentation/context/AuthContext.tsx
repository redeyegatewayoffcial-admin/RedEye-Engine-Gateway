// Presentation Context — AuthContext
// Provides isAuthenticated, user, login, signup, completeOnboarding globally.

import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from 'react';
import type { User } from '../../domain/entities/User';
import type { IAuthUseCaseExtended } from '../../data/services/authService';

interface AuthContextValue {
  isAuthenticated: boolean;
  user: User | null;
  requestMagicLink(email: string): Promise<void>;
  verifyMagicLink(email: string, otp: string): Promise<User>;
  ssoRedirect(provider: string): Promise<void>;
  syncOAuthState(token: string): Promise<void>;
  completeOnboarding(workspaceName: string, provider: string, apiKey: string): Promise<User>;
  logout(): void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

interface AuthProviderProps {
  children: ReactNode;
  authUseCase: IAuthUseCaseExtended;
}

export function AuthProvider({ children, authUseCase }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null);

  const requestMagicLink = useCallback(async (email: string) => {
    await authUseCase.requestMagicLink(email);
  }, [authUseCase]);

  const verifyMagicLink = useCallback(async (email: string, otp: string) => {
    const u = await authUseCase.verifyMagicLink(email, otp);
    setUser(u);
    return u;
  }, [authUseCase]);

  const ssoRedirect = useCallback(async (provider: string) => {
    if (authUseCase.ssoRedirect) {
      await authUseCase.ssoRedirect(provider);
    }
  }, [authUseCase]);

  const syncOAuthState = useCallback(async (token: string) => {
    // Persist token to localStorage via the Data layer
    if (authUseCase.saveToken) {
      authUseCase.saveToken(token);
    } else {
      localStorage.setItem('re_token', token);
    }
    // Hydrate React state — use refreshToken which reads from the saved token
    if (authUseCase.refreshToken) {
      const u = await authUseCase.refreshToken();
      if (u) setUser(u);
    }
  }, [authUseCase]);

  const completeOnboarding = useCallback(
    async (workspaceName: string, provider: string, apiKey: string) => {
      if (!user) throw new Error('Not authenticated');
      const updated = await authUseCase.completeOnboarding(
        user.id,
        workspaceName,
        provider,
        apiKey,
      );
      setUser(updated);
      return updated;
    },
    [user, authUseCase],
  );

  const logout = useCallback(() => {
    localStorage.removeItem('re_token');
    setUser(null);
    window.location.href = '/login';
  }, []);

  return (
    <AuthContext.Provider
      value={{
        isAuthenticated: user !== null,
        user,
        requestMagicLink,
        verifyMagicLink,
        ssoRedirect,
        syncOAuthState,
        completeOnboarding,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used inside <AuthProvider>');
  return ctx;
}
