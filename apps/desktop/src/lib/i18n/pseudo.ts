import type { Catalog } from "./index";

/** Visible brackets and 40% expansion reveal untranslated UI and tight layouts. */
export function pseudoCatalog(source: Catalog): Record<string, string> {
  return Object.fromEntries(Object.entries(source).map(([key, text]) => [key,
    `⟦${text.replace(/\{\w+\}|[aeiouAEIOU]/g, (part) => part.startsWith("{") ? part : ({ a: "á", e: "ë", i: "ï", o: "ö", u: "ü", A: "Á", E: "Ë", I: "Ï", O: "Ö", U: "Ü" }[part] ?? part))}${"~".repeat(Math.ceil(text.length * 0.4))}⟧`,
  ]));
}
