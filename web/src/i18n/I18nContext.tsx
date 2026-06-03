import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

import { localizeDom, observeLocalization, translateString } from './domLocalizer';
import { enUS, zhCN } from './messages';

export type LocaleCode = 'zh-CN' | 'en-US';

export const LOCALE_STORAGE_KEY = 'tikee.locale';

const SUPPORTED_LOCALES: LocaleCode[] = ['zh-CN', 'en-US'];

export interface LocaleOption {
  value: LocaleCode;
  label: string;
}

export const LOCALE_OPTIONS: LocaleOption[] = [
  { value: 'zh-CN', label: '中文' },
  { value: 'en-US', label: 'English' },
];

interface I18nContextValue {
  locale: LocaleCode;
  setLocale: (locale: LocaleCode) => void;
  t: (text: string) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function normalizeLocale(value: string | null | undefined): LocaleCode {
  if (value && SUPPORTED_LOCALES.includes(value as LocaleCode)) return value as LocaleCode;
  const browserLanguage = typeof navigator === 'undefined' ? '' : navigator.language ?? '';
  return browserLanguage.toLowerCase().startsWith('en') ? 'en-US' : 'zh-CN';
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<LocaleCode>(() => {
    if (typeof window === 'undefined') return 'zh-CN';
    return normalizeLocale(window.localStorage.getItem(LOCALE_STORAGE_KEY));
  });

  const messages = locale === 'en-US' ? enUS : zhCN;

  const setLocale = (nextLocale: LocaleCode) => {
    const normalized = normalizeLocale(nextLocale);
    setLocaleState(normalized);
    window.localStorage.setItem(LOCALE_STORAGE_KEY, normalized);
  };

  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.dataset.locale = locale;
    const root = document.getElementById('root');
    if (!root) return undefined;
    localizeDom(root, messages, true);
    const observer = observeLocalization(root, messages, true);
    return () => observer.disconnect();
  }, [locale, messages]);

  const value = useMemo<I18nContextValue>(() => ({
    locale,
    setLocale,
    t: (text: string) => translateString(text, messages, true),
  }), [locale, messages]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) throw new Error('useI18n must be used inside I18nProvider');
  return context;
}
