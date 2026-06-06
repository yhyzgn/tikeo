import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

import { localizeDom, observeLocalization, translateString } from './domLocalizer';
import { enUS, zhCN } from './messages';

const LOCALE_REGISTRY = {
  'zh-CN': { label: '中文', messages: zhCN },
  'en-US': { label: 'English', messages: enUS },
} as const;

export type LocaleCode = keyof typeof LOCALE_REGISTRY;

export const LOCALE_STORAGE_KEY = 'tikee.locale';

const SUPPORTED_LOCALES = Object.keys(LOCALE_REGISTRY) as LocaleCode[];

export interface LocaleOption {
  value: LocaleCode;
  label: string;
}

export const LOCALE_OPTIONS: LocaleOption[] = SUPPORTED_LOCALES.map((value) => ({ value, label: LOCALE_REGISTRY[value].label }));

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

  const messages = LOCALE_REGISTRY[locale].messages;

  const setLocale = (nextLocale: LocaleCode) => {
    const normalized = normalizeLocale(nextLocale);
    setLocaleState(normalized);
    window.localStorage.setItem(LOCALE_STORAGE_KEY, normalized);
  };

  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.dataset.locale = locale;
    const root = document.body;
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
