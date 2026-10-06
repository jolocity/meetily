'use client';

import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useState, ReactNode } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  ThemePreference,
  ResolvedTheme,
  DEFAULT_THEME_PREFERENCE,
  DARK_MODE_QUERY,
  loadThemePreference,
  saveThemePreference,
  resolveTheme,
  applyThemeClass,
} from '@/lib/theme';

interface ThemeContextType {
  preference: ThemePreference;
  resolvedTheme: ResolvedTheme;
  setPreference: (preference: ThemePreference) => void;
}

const ThemeContext = createContext<ThemeContextType | null>(null);

export const useTheme = () => {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within ThemeProvider');
  return ctx;
};

// Layout effects let the saved theme apply before paint; they are no-ops during static export.
const useIsomorphicLayoutEffect = typeof window !== 'undefined' ? useLayoutEffect : useEffect;

async function syncWindowTheme(preference: ThemePreference): Promise<void> {
  try {
    // null lets the native title bar and the webview's prefers-color-scheme follow the OS.
    await getCurrentWindow().setTheme(preference === 'system' ? null : preference);
  } catch (error) {
    console.warn('[Theme] Failed to set window theme:', error);
  }
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<ThemePreference>(DEFAULT_THEME_PREFERENCE);
  const [systemPrefersDark, setSystemPrefersDark] = useState(false);
  const [isLoaded, setIsLoaded] = useState(false);

  const resolvedTheme = resolveTheme(preference, systemPrefersDark);

  useIsomorphicLayoutEffect(() => {
    const mediaQuery = window.matchMedia(DARK_MODE_QUERY);
    const handleChange = (event: MediaQueryListEvent) => setSystemPrefersDark(event.matches);

    setPreferenceState(loadThemePreference());
    setSystemPrefersDark(mediaQuery.matches);
    setIsLoaded(true);

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  useIsomorphicLayoutEffect(() => {
    if (isLoaded) applyThemeClass(resolvedTheme);
  }, [isLoaded, resolvedTheme]);

  useEffect(() => {
    if (isLoaded) syncWindowTheme(preference);
  }, [isLoaded, preference]);

  const setPreference = useCallback((next: ThemePreference) => {
    setPreferenceState(next);
    saveThemePreference(next);
  }, []);

  return (
    <ThemeContext.Provider value={{ preference, resolvedTheme, setPreference }}>
      {children}
    </ThemeContext.Provider>
  );
}
