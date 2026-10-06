export type ThemePreference = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

// Keep in sync with public/theme-init.js, which applies the theme before first paint.
export const THEME_STORAGE_KEY = 'themePreference';
export const DEFAULT_THEME_PREFERENCE: ThemePreference = 'light';
export const DARK_MODE_QUERY = '(prefers-color-scheme: dark)';
export const THEME_PREFERENCES: readonly ThemePreference[] = ['light', 'dark', 'system'];

const DARK_CLASS = 'dark';

export function isThemePreference(value: unknown): value is ThemePreference {
  return typeof value === 'string' && (THEME_PREFERENCES as readonly string[]).includes(value);
}

/**
 * Load the theme preference from localStorage
 *
 * @returns Saved preference, or the default when missing or invalid
 */
export function loadThemePreference(): ThemePreference {
  if (typeof window === 'undefined') return DEFAULT_THEME_PREFERENCE;

  try {
    const saved = localStorage.getItem(THEME_STORAGE_KEY);
    if (isThemePreference(saved)) return saved;
  } catch (error) {
    console.error('[Theme] Failed to load from localStorage:', error);
  }

  return DEFAULT_THEME_PREFERENCE;
}

/**
 * Save the theme preference to localStorage
 *
 * @param preference - Theme preference to save
 */
export function saveThemePreference(preference: ThemePreference): void {
  if (typeof window === 'undefined') return;

  try {
    localStorage.setItem(THEME_STORAGE_KEY, preference);
  } catch (error) {
    console.error('[Theme] Failed to save to localStorage:', error);
  }
}

export function resolveTheme(preference: ThemePreference, systemPrefersDark: boolean): ResolvedTheme {
  if (preference === 'system') return systemPrefersDark ? 'dark' : 'light';
  return preference;
}

export function applyThemeClass(theme: ResolvedTheme): void {
  const root = document.documentElement;
  root.classList.toggle(DARK_CLASS, theme === 'dark');
  root.style.colorScheme = theme;
}
