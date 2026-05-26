import { createContext, useContext } from 'react';

export const DEFAULT_PRIMARY_COLOR = '#2563eb';
export const DEFAULT_INFO_COLOR = '#0ea5e9';
export const PRIMARY_COLOR_STORAGE_KEY = 'tikee.primaryColor';

export interface ThemeSettings {
  primaryColor: string;
  setPrimaryColor: (color: string) => void;
  resetPrimaryColor: () => void;
}

export const ThemeSettingsContext = createContext<ThemeSettings>({
  primaryColor: DEFAULT_PRIMARY_COLOR,
  setPrimaryColor: () => undefined,
  resetPrimaryColor: () => undefined,
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
