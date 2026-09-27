"use client";

import { createContext, useCallback, useContext, useEffect, useState } from "react";

export type ThemeMode = "light" | "dark" | "system";

const STORAGE_KEY = "orgion-theme";

interface ThemeState {
  mode: ThemeMode;
  resolved: "light" | "dark";
  setMode: (mode: ThemeMode) => void;
}

const ThemeContext = createContext<ThemeState | null>(null);

function resolve(mode: ThemeMode): "light" | "dark" {
  if (mode === "system") {
    if (typeof window === "undefined") return "light";
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return mode;
}

function apply(resolved: "light" | "dark") {
  document.documentElement.setAttribute("data-theme", resolved);
}

/**
 * Inline script injected into <head> (see layout.tsx) so the correct
 * theme is applied before first paint — this provider's own effect below
 * runs the same logic again on mount, which is redundant but harmless,
 * and is what keeps things in sync after that (mode changes, OS
 * preference changes while mode === "system").
 */
export const themeInitScript = `
(function () {
  try {
    var mode = localStorage.getItem('${STORAGE_KEY}') || 'system';
    var resolved = mode === 'system'
      ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
      : mode;
    document.documentElement.setAttribute('data-theme', resolved);
  } catch (e) {}
})();
`;

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [mode, setModeState] = useState<ThemeMode>("system");
  const [resolved, setResolved] = useState<"light" | "dark">("light");

  useEffect(() => {
    let stored: ThemeMode = "system";
    try {
      stored = (localStorage.getItem(STORAGE_KEY) as ThemeMode) || "system";
    } catch {
      // localStorage unavailable (private mode etc.) — fall back silently.
    }
    setModeState(stored);
    const r = resolve(stored);
    setResolved(r);
    apply(r);
  }, []);

  useEffect(() => {
    if (mode !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      const r = resolve("system");
      setResolved(r);
      apply(r);
    };
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [mode]);

  const setMode = useCallback((next: ThemeMode) => {
    setModeState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // ignore
    }
    const r = resolve(next);
    setResolved(r);
    apply(r);
  }, []);

  return <ThemeContext.Provider value={{ mode, resolved, setMode }}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeState {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
