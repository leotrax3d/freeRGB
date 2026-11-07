import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

export const localeDirectory = fileURLToPath(new URL("../apps/desktop/src/lib/i18n/locales/", import.meta.url));
export const readCatalog = (code) => JSON.parse(readFileSync(`${localeDirectory}/${code}.json`, "utf8").replace(/^\uFEFF/, ""));
const plural = /^(.*)_(zero|one|two|few|many|other)$/;
const placeholders = (text) => [...text.matchAll(/\{\w+\}/g)].map((m) => m[0]).sort().join(",");

/** Return actionable errors for just one language; safe to use from tests/CI. */
export function validateCatalog(code, catalog, source = readCatalog("en")) {
  const errors = [];
  let categories;
  try {
    if (Intl.getCanonicalLocales(code)[0] !== code) errors.push(`${code}: use a canonical BCP 47 filename`);
    if (!Intl.PluralRules.supportedLocalesOf([code]).length) errors.push(`${code}: unsupported CLDR locale`);
    categories = new Intl.PluralRules(code).resolvedOptions().pluralCategories;
  } catch { return [`${code}: invalid BCP 47 locale`]; }
  if (!catalog || typeof catalog !== "object" || Array.isArray(catalog)) return [`${code}: expected a flat JSON object`];
  for (const key of Object.keys(source)) {
    if (!Object.hasOwn(catalog, key)) errors.push(`${code}: missing ${key}`);
  }
  const bases = new Set(Object.keys(source).map((key) => key.match(plural)?.[1]).filter(Boolean));
  for (const base of bases) for (const category of categories) {
    if (!Object.hasOwn(catalog, `${base}_${category}`)) errors.push(`${code}: missing plural ${base}_${category}`);
  }
  for (const [key, value] of Object.entries(catalog)) {
    const match = key.match(plural);
    const reference = source[key] ?? (match && bases.has(match[1]) ? source[`${match[1]}_other`] : undefined);
    if (reference === undefined) { errors.push(`${code}: unknown key ${key}`); continue; }
    if (typeof value !== "string" || !value.trim()) { errors.push(`${code}: ${key} must be a nonempty string`); continue; }
    if (placeholders(value) !== placeholders(reference)) errors.push(`${code}: placeholders differ for ${key}`);
    if (/[{}]/.test(value.replace(/\{\w+\}/g, ""))) errors.push(`${code}: malformed placeholder in ${key}`);
    if (/<\/?[a-z!][^>]*>|&#?\w+;|!?\[[^\]]*\]\([^)]*\)|\*\*|```/i.test(value)) errors.push(`${code}: markup forbidden in ${key}`);
  }
  return errors;
}
export function validateLocales(target = "all") {
  const codes = readdirSync(localeDirectory).filter((name) => name.endsWith(".json")).map((name) => name.slice(0, -5));
  if (target !== "all" && !codes.includes(target)) return [`Unknown locale: ${target}`];
  return (target === "all" ? codes : [target]).flatMap((code) => {
    try { return validateCatalog(code, readCatalog(code)); }
    catch (error) { return [`${code}: ${error.message}`]; }
  });
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const errors = validateLocales(process.argv[2] ?? "all");
  if (errors.length) { console.error(errors.join("\n")); process.exitCode = 1; }
  else console.log(`Catalog validation passed (${process.argv[2] ?? "all"}).`);
}
