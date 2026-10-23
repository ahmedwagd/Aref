import ar from './ar.json';
import en from './en.json';

export const locales = ['ar', 'en'] as const;
export type Locale = (typeof locales)[number];

/** Arabic is the primary quality bar (AGENTS.md §1), so it is the default. */
export const defaultLocale: Locale = 'ar';

/**
 * The English catalog defines the key set. `tests/i18n.catalog.test.ts` proves the
 * Arabic catalog carries exactly the same keys, so the lookup below cannot silently
 * fall back to an English string in the Arabic UI.
 */
export type MessageKey = keyof typeof en;

const catalogs: Record<Locale, Record<MessageKey, string>> = {
  ar,
  en,
};

export type Translate = (key: MessageKey) => string;

export function translate(locale: Locale, key: MessageKey): string {
  return catalogs[locale][key];
}

export function createTranslator(locale: Locale): Translate {
  return (key: MessageKey) => translate(locale, key);
}

export function directionFor(locale: Locale): 'rtl' | 'ltr' {
  return locale === 'ar' ? 'rtl' : 'ltr';
}

/**
 * Direction lives on <html> (DESIGN.md §7.6), so no component and no per-locale
 * stylesheet needs to branch on it.
 */
export function applyDocumentLocale(locale: Locale): void {
  document.documentElement.lang = locale;
  document.documentElement.dir = directionFor(locale);
}
