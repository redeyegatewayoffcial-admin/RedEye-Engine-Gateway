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
import { authService } from '../../data/services/authService';

interface AuthContextValue {
  isAuthenticated: boolean;
  user: User | null;
  login(email: string, password: string): Promise<void>;
  signup(email: string, password: string): Promise<void>;
  completeOnboarding(workspaceName: string, provider: string, apiKey: string): Promise<User>;
  logout(): void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);

  const login = useCallback(async (email: string, password: string) => {
    const u = await authService.login({ email, password });
    setUser(u);
  }, []);

  const signup = useCallback(async (email: string, password: string) => {
    const u = await authService.signup({ email, password });
    setUser(u);
  }, []);

  const completeOnboarding = useCallback(
    async (workspaceName: string, provider: string, apiKey: string) => {
      if (!user) throw new Error('Not authenticated');
      const updated = await authService.completeOnboarding(
        user.id,
        workspaceName,
        provider,
        apiKey,
      );
      setUser(updated);
      return updated;
    },
    [user],
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
        login,
        signup,
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
