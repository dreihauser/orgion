"use client";

import { createContext, useCallback, useContext, useEffect, useState } from "react";
import * as api from "./api";
import type { User, Workspace } from "./types";

interface AuthState {
  user: User | null;
  workspaces: Workspace[];
  /** True until the initial `GET /api/auth/me` check resolves. */
  loading: boolean;
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  refresh: () => Promise<void>;
}

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const me = await api.getMe();
      setUser(me.user);
      setWorkspaces(me.workspaces);
    } catch {
      setUser(null);
      setWorkspaces([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const login = useCallback(async (username: string, password: string) => {
    const loggedInUser = await api.login(username, password);
    setUser(loggedInUser);
    await refresh();
  }, [refresh]);

  const logout = useCallback(async () => {
    await api.logout();
    setUser(null);
    setWorkspaces([]);
  }, []);

  return (
    <AuthContext.Provider value={{ user, workspaces, loading, login, logout, refresh }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthState {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within AuthProvider");
  return ctx;
}
