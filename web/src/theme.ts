import { createContext, useContext } from 'react';

export const DEFAULT_PRIMARY_COLOR = '#2563eb';
export const DEFAULT_INFO_COLOR = '#0ea5e9';
export const PRIMARY_COLOR_STORAGE_KEY = 'tikeo.primaryColor';
export const THEME_MODE_STORAGE_KEY = 'tikeo.themeMode';
export type ThemeMode = 'light' | 'dark';
export type ThemePreference = 'light' | 'dark' | 'system';

export interface ThemeSettings {
  primaryColor: string;
  mode: ThemePreference;
  resolvedMode: ThemeMode;
  setPrimaryColor: (color: string) => void;
  resetPrimaryColor: () => void;
  setMode: (mode: ThemePreference) => void;
  toggleMode: () => void;
}

export const ThemeSettingsContext = createContext<ThemeSettings>({
  primaryColor: DEFAULT_PRIMARY_COLOR,
  mode: 'system',
  resolvedMode: 'light',
  setPrimaryColor: () => undefined,
  resetPrimaryColor: () => undefined,
  setMode: () => undefined,
  toggleMode: () => undefined,
});

export function useThemeSettings(): ThemeSettings {
  return useContext(ThemeSettingsContext);
}

export function normalizeHexColor(value: string | null | undefined): string | null {
  if (!value) return null;
  const trimmed = value.trim();
  if (/^#[0-9a-fA-F]{6}$/.test(trimmed)) return trimmed.toLowerCase();
  if (/^[0-9a-fA-F]{6}$/.test(trimmed)) return `#${trimmed.toLowerCase()}`;
  return null;
}

export function normalizeThemeMode(value: string | null | undefined): ThemePreference {
  if (value === 'light' || value === 'dark' || value === 'system') return value;
  return 'system';
}

export function resolveThemeMode(mode: ThemePreference, systemPrefersDark: boolean): ThemeMode {
  return mode === 'system' ? (systemPrefersDark ? 'dark' : 'light') : mode;
}
