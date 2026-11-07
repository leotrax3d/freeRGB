import { create } from "zustand";
import en from "./locales/en.json";
import metadata from "./locale-metadata.json";
import { pseudoCatalog } from "./pseudo";

export type Locale = string;
export type LocalePreference = Locale | "auto";
export type Params = Readonly<Record<string, string | number>>;
export type Catalog = Readonly<Record<string, string>>;
/** en.json is the single typed source for core and feature keys. */
export type MessageKey = keyof typeof en;
export type FeatureKey<P extends string> = Extract<MessageKey, `${P}.${string}`>;
type PluralBase<K extends string> = K extends `${infer B}_other` ? B : never;
export type PluralKey = PluralBase<MessageKey>;

const modules = import.meta.glob<Catalog>("./locales/*.json", { eager: true, import: "default" });
const catalogs: Record<string, Catalog> = Object.fromEntries(
  Object.entries(modules).map(([path, catalog]) => [path.split("/").pop()!.replace(/\.json$/, ""), catalog]),
);
// The pseudo-locale expands every message and preserves placeholders for layout QA.
if (import.meta.env.DEV) catalogs["en-XA"] = pseudoCatalog(en);
const meta: Record<string, { reviewed?: boolean; machineTranslated?: boolean }> = metadata;
export const LOCALES = Object.keys(catalogs).sort().map((id) => ({
  id,
  label: id === "en-XA" ? "Pseudo (expanded)" : new Intl.DisplayNames([id], { type: "language" }).of(id) ?? id,
  reviewed: id === "en" || meta[id]?.reviewed === true,
  // Newly generated catalogs are unreviewed and machine-translated by default.
  machineTranslated: id !== "en" && id !== "en-XA" && meta[id]?.machineTranslated !== false,
}));
const STORAGE_KEY = "freergb.locale";
export function pickLocale(languages: readonly string[]): Locale {
  for (const tag of languages) {
    const normalized = tag.toLowerCase();
    const exact = LOCALES.find((l) => l.id.toLowerCase() === normalized);
    if (exact) return exact.id;
    const base = normalized.split("-")[0];
    const match = LOCALES.find((l) => l.id.toLowerCase() === base);
    if (match) return match.id;
  }
  return "en";
}
function browserLanguages(): readonly string[] {
  if (typeof navigator === "undefined") return [];
  return navigator.languages?.length ? navigator.languages : [navigator.language];
}
function storedPreference(): LocalePreference {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    if (value && Object.hasOwn(catalogs, value)) return value;
  } catch { /* Storage unavailable; use automatic detection. */ }
  return "auto";
}
export function resolveLocale(preference: LocalePreference, languages: readonly string[] = browserLanguages()): Locale {
  return preference === "auto" ? pickLocale(languages) : Object.hasOwn(catalogs, preference) ? preference : "en";
}
export function direction(locale: string): "rtl" | "ltr" {
  return /^(ar|he|fa|ur)(-|$)/i.test(locale) ? "rtl" : "ltr";
}
function applyDocumentLanguage(locale: Locale) {
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
    document.documentElement.dir = direction(locale);
  }
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
    } catch { /* The choice lasts for this session. */ }
    const locale = resolveLocale(preference);
    applyDocumentLanguage(locale);
    set({ preference, locale });
  },
}));
export const currentLocale = (): Locale => useLocale.getState().locale;
export const formatTag = (locale: Locale = currentLocale()): string => locale === "en-XA" ? "en" : locale;
function interpolate(template: string, params?: Params): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) => Object.hasOwn(params, name) ? String(params[name]) : match);
}
export function hasMessage(key: string): key is MessageKey {
  return Object.hasOwn(en, key);
}

/** Safety text always includes English unless this locale has an explicit review. */
export function isSafetyMessage(key: string): boolean {
  const english = (en as Catalog)[key] ?? "";
  // Explanations and warnings are bilingual; compact status/control labels stay
  // usable in selectors. New safety explanations inherit this policy.
  return ((key.startsWith("cooling.") || key.startsWith("onboarding.") || key.startsWith("settings.setup.")) &&
    (english.length >= 60 || (english.length >= 25 && /pump|unrestricted|firmware|safety|critical|watchdog|fan stop|risk|crash|never|minimum|locked/i.test(english)))) ||
    key === "cooling.fanCard.pumpsStayAt100" || key === "onboarding.fanStep.pumpStaysAt100" ||
    /^validation\.(curve|minimumDuty|criticalTemperature|hysteresis|point|monotonic|rateLimit|equilibrium)/.test(key);
}
function translated(key: string, locale: Locale, params?: Params): string {
  const english = (en as Catalog)[key] ?? key;
  const text = interpolate(catalogs[locale]?.[key] ?? english, params);
  const fallback = interpolate(english, params);
  return isSafetyMessage(key) && !LOCALES.find((l) => l.id === locale)?.reviewed && text !== fallback
    ? `${text}\n\nEnglish: ${fallback}` : text;
}
export function t(key: MessageKey, params?: Params): string {
  return translated(key, currentLocale(), params);
}
const pluralRules: Record<string, Intl.PluralRules> = {};
export function pluralCategory(count: number, locale: Locale): Intl.LDMLPluralRule {
  return (pluralRules[locale] ??= new Intl.PluralRules(formatTag(locale))).select(count);
}
export function tn(key: PluralKey, count: number, params?: Params): string {
  const locale = currentLocale();
  const category = pluralCategory(count, locale);
  const catalog = catalogs[locale] ?? {};
  const selected = Object.hasOwn(catalog, `${key}_${category}`) ? `${key}_${category}` : `${key}_other`;
  // English uses its own plural rules, even if the translated form is few/many.
  const englishKey = `${key}_${pluralCategory(count, "en")}`;
  const values = { ...params, count };
  const fallback = interpolate((en as Catalog)[englishKey] ?? (en as Catalog)[`${key}_other`] ?? key, values);
  const text = interpolate(catalog[selected] ?? (en as Catalog)[englishKey] ?? (en as Catalog)[`${key}_other`] ?? key, values);
  return isSafetyMessage(key) && !LOCALES.find((l) => l.id === locale)?.reviewed && text !== fallback
    ? `${text}\n\nEnglish: ${fallback}` : text;
}
/** Feature helpers constrain callers to their namespace; catalogs load centrally. */
export function defineMessages<P extends string>(prefix: P) {
  if (!Object.keys(en).some((key) => key.startsWith(`${prefix}.`))) throw new Error(`Unknown message prefix: ${prefix}`);
  return {
    t: (key: FeatureKey<P>, params?: Params): string => t(key, params),
    tn: (key: PluralBase<FeatureKey<P>>, count: number, params?: Params): string => tn(key as PluralKey, count, params),
  };
}
export function registeredKeys(locale: Locale): string[] {
  return Object.keys(catalogs[locale] ?? {});
}
