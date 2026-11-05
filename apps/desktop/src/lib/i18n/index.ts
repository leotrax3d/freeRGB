import { create } from "zustand";
import { de as coreDe } from "./de";
import { en as coreEn } from "./en";

/**
 * Minimal i18n for the desktop app (no runtime dependency, see docs/I18N.md).
 *
 * - Messages are flat `key -> template` catalogs; `{name}` is replaced by a parameter.
 * - Plurals use key suffixes (`_one`, `_other`) and `tn()`.
 * - English is the source language and the fallback for any missing key.
 * - Switching the language remounts the app (Shell keys its tree by locale), so
 *   plain `t()` calls in render code, memoised components and helpers all pick
 *   up the new language without per-component subscriptions.
 */
export type Locale = "en" | "de";
export type LocalePreference = Locale | "auto";
export type Params = Readonly<Record<string, string | number>>;
export type Catalog = Readonly<Record<string, string>>;

export const LOCALES: readonly { id: Locale; label: string }[] = [
  { id: "en", label: "English" },
  { id: "de", label: "Deutsch" },
];

const STORAGE_KEY = "freergb.locale";

/** Core catalog keys; feature catalogs use `defineMessages` for their own typed keys. */
export type MessageKey = keyof typeof coreEn;
/** Base names of plural pairs (`foo` for `foo_one` + `foo_other`). */
export type PluralKey = { [K in MessageKey]: K extends `${infer B}_other` ? B : never }[MessageKey];

const catalogs: Record<Locale, Record<string, string>> = { en: { ...coreEn }, de: { ...coreDe } };

/** Maps browser language tags (`de-AT`, `en-US`, …) to a supported locale. */
export function pickLocale(languages: readonly string[]): Locale {
  for (const tag of languages) {
    const base = tag.toLowerCase().split("-")[0];
    if (base === "de") return "de";
    if (base === "en") return "en";
  }
  return "en";
}

function browserLanguages(): readonly string[] {
  if (typeof navigator === "undefined") return [];
  return navigator.languages?.length ? navigator.languages : navigator.language ? [navigator.language] : [];
}

function storedPreference(): LocalePreference {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    if (value === "en" || value === "de") return value;
  } catch {
    // Storage unavailable: fall back to automatic detection.
  }
  return "auto";
}

export function resolveLocale(preference: LocalePreference, languages: readonly string[] = browserLanguages()): Locale {
  return preference === "auto" ? pickLocale(languages) : preference;
}

function applyDocumentLanguage(locale: Locale) {
  if (typeof document !== "undefined") document.documentElement.lang = locale;
}

const initialPreference = storedPreference();
const initialLocale = resolveLocale(initialPreference);
applyDocumentLanguage(initialLocale);

export const useLocale = create<{
  preference: LocalePreference;
  locale: Locale;
  setPreference: (preference: LocalePreference) => void;
}>((set) => ({
  preference: initialPreference,
  locale: initialLocale,
  setPreference: (preference) => {
    try {
      if (preference === "auto") localStorage.removeItem(STORAGE_KEY);
      else localStorage.setItem(STORAGE_KEY, preference);
    } catch {
      // The choice then only lasts for this session.
    }
    const locale = resolveLocale(preference);
    applyDocumentLanguage(locale);
    set({ preference, locale });
  },
}));

export const currentLocale = (): Locale => useLocale.getState().locale;

/** BCP 47 tag used for number and date formatting. */
export const formatTag = (locale: Locale = currentLocale()): string => (locale === "de" ? "de-DE" : "en-US");

function interpolate(template: string, params?: Params): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match));
}

function lookup(key: string, locale: Locale): string {
  return catalogs[locale][key] ?? catalogs.en[key] ?? key;
}

/** True if the core catalog has this key (for keys built at runtime). */
export function hasMessage(key: string): key is MessageKey {
  return key in coreEn;
}

/** Translates a core message in the current language. */
export function t(key: MessageKey, params?: Params): string {
  return interpolate(lookup(key, currentLocale()), params);
}

const pluralRules: Partial<Record<Locale, Intl.PluralRules>> = {};
function pluralCategory(count: number, locale: Locale): string {
  return (pluralRules[locale] ??= new Intl.PluralRules(formatTag(locale))).select(count);
}

/** Plural-aware translation: picks `<key>_one` / `<key>_other`; `{count}` is filled in. */
export function tn(key: PluralKey, count: number, params?: Params): string {
  return pluralMessage(key, count, params);
}

function pluralMessage(key: string, count: number, params?: Params): string {
  const locale = currentLocale();
  const category = pluralCategory(count, locale);
  const catalog = catalogs[locale];
  const template = catalog[`${key}_${category}`] ?? catalog[`${key}_other`] ?? catalogs.en[`${key}_${category}`] ?? catalogs.en[`${key}_other`] ?? key;
  return interpolate(template, { count, ...params });
}

/**
 * Registers a feature's messages (cooling, macros, …) and returns a `t` typed
 * on that feature's keys. Keys must be prefixed with the feature name so they
 * can never collide with core messages. Call at module scope.
 */
export function defineMessages<K extends string>(
  prefix: string,
  messages: { en: Readonly<Record<K, string>>; de: Readonly<Record<K, string>> },
) {
  for (const locale of ["en", "de"] as const) {
    for (const [key, value] of Object.entries<string>(messages[locale])) {
      if (!key.startsWith(`${prefix}.`)) throw new Error(`Message key "${key}" must start with "${prefix}."`);
      catalogs[locale][key] = value;
    }
  }
  type Plural = { [P in K]: P extends `${infer B}_other` ? B : never }[K];
  return {
    t: (key: K, params?: Params): string => interpolate(lookup(key, currentLocale()), params),
    tn: (key: Plural, count: number, params?: Params): string => pluralMessage(key, count, params),
  };
}

/** For tests: all registered keys of a locale. */
export function registeredKeys(locale: Locale): string[] {
  return Object.keys(catalogs[locale]);
}
