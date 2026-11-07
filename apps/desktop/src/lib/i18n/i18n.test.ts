import { beforeEach, describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { renderToStaticMarkup } from "react-dom/server";
import { createElement } from "react";
import { Button } from "@/components/ui/button";
import en from "./locales/en.json";
import de from "./locales/de.json";
import metadata from "./locale-metadata.json";
import { defineMessages, direction, LOCALES, pickLocale, pluralCategory, registeredKeys, resolveLocale, t, tn, useLocale } from "./index";
import { pseudoCatalog } from "./pseudo";
import { wrapIndex } from "../a11y";

// The CLI is also exercised in-process, including future locale fixtures.
const require = createRequire(import.meta.url);
const { validateCatalog, validateLocales } = require("../../../../../scripts/validate-i18n.mjs") as {
  validateCatalog: (code: string, catalog: Record<string, unknown>, source?: Record<string, string>) => string[];
  validateLocales: (target: string) => string[];
};
const { scanHardcodedUi, hardcodedUi } = require("../../../../../scripts/check-i18n-ui.mjs") as {
  scanHardcodedUi: () => string[];
  hardcodedUi: (source: string) => string[];
};

beforeEach(() => useLocale.getState().setPreference("en"));
describe("locale selection", () => {
  it("matches exact tags, base language and English fallback", () => {
    expect(pickLocale(["de-AT", "en"])).toBe("de");
    expect(pickLocale(["fr-FR", "en-GB"])).toBe("fr");
    expect(pickLocale(["zz-ZZ", "en-GB"])).toBe("en");
    expect(pickLocale(["pt-BR", "pt-PT"])).toBe("pt-BR");
    expect(pickLocale(["zh-Hant", "zh-Hans"])).toBe("zh-Hant");
    expect(pickLocale([])).toBe("en");
    expect(resolveLocale("de", ["en-US"])).toBe("de");
    expect(resolveLocale("missing")).toBe("en");
    expect(LOCALES.find((l) => l.id === "de")?.label).toBe("Deutsch");
    expect(LOCALES.find((l) => l.id === "de")?.reviewed).toBe(false);
  });
  it("updates document language and direction on each switch", () => {
    const html = { lang: "", dir: "" };
    vi.stubGlobal("document", { documentElement: html });
    try {
      useLocale.getState().setPreference("de");
      expect(html).toEqual({ lang: "de", dir: "ltr" });
      for (const locale of ["ar", "ar-EG", "he", "fa", "ur-PK"]) expect(direction(locale)).toBe("rtl");
      expect(direction("en")).toBe("ltr");
    } finally { vi.unstubAllGlobals(); }
  });
});
describe("translation", () => {
  it("interpolates, switches language and keeps unknown placeholders", () => {
    expect(t("palette.running", { name: "Rainbow" })).toBe("Rainbow is running");
    expect(t("palette.running")).toBe("{name} is running");
    useLocale.getState().setPreference("de");
    expect(t("common.back")).toBe("Zurück");
    expect(t("nope" as never)).toBe("nope");
    expect(defineMessages("settings").t("settings.language")).toBe("Sprache");
    expect(() => defineMessages("unknown")).toThrow();
  });
  it("uses English when a locale entry is missing", () => {
    const catalog: Record<string, string> = de;
    const saved = catalog["common.back"]!;
    delete catalog["common.back"];
    try {
      useLocale.getState().setPreference("de");
      expect(t("common.back")).toBe(en["common.back"]);
    } finally { catalog["common.back"] = saved; }
  });
  it("selects all six CLDR categories, including decimals", () => {
    expect([0, 1, 2, 3, 11, 100].map((n) => pluralCategory(n, "ar"))).toEqual(["zero", "one", "two", "few", "many", "other"]);
    expect(pluralCategory(1.5, "ru")).toBe("other");
    expect(tn("shell.conflict", 1, { names: "A" })).toMatch(/is running/);
    expect(tn("shell.conflict", 2, { names: "A, B" })).toMatch(/are running/);
  });
  it("keeps safety warnings bilingual until explicitly reviewed", () => {
    useLocale.getState().setPreference("de");
    const key = "cooling.fanCard.pumpsStayAt100";
    expect(t(key)).toBe(`${de[key]}\n\nEnglish: ${en[key]}`);
    const locale = LOCALES.find((l) => l.id === "de")!;
    locale.reviewed = true;
    try { expect(t(key)).toBe(de[key]); }
    finally { locale.reviewed = false; }
    useLocale.getState().setPreference("en");
    expect(t(key)).toBe(en[key]);
  });
});
describe("catalog validation", () => {
  it("explicitly marks generated catalogs as machine-translated and unreviewed", () => {
    const provenance: Record<string, { machineTranslated: boolean; reviewed: boolean }> = metadata;
    for (const locale of LOCALES.filter((l) => l.id !== "en" && l.id !== "en-XA")) {
      expect(provenance[locale.id], locale.id).toEqual({ machineTranslated: true, reviewed: false });
      expect(locale.machineTranslated, locale.id).toBe(true);
      expect(locale.reviewed, locale.id).toBe(false);
    }
  });
  it("validates all shipped catalogs without importing feature views", () => {
    expect(validateLocales("all")).toEqual([]);
    expect(validateLocales("de")).toEqual([]);
    expect(validateLocales("../../invalid")).toEqual(["Unknown locale: ../../invalid"]);
    expect(registeredKeys("en").sort()).toEqual(Object.keys(en).sort());
  });
  it("rejects missing keys, changed placeholders, markup and incomplete plurals", () => {
    const source = { "n_one": "{count} item", "n_other": "{count} items", "title": "Hi {name}" };
    const arabic = { ...source, n_zero: "{count}", n_two: "{count}", n_few: "{count}", n_many: "{count}" };
    expect(validateCatalog("ar", arabic, source)).toEqual([]);
    expect(validateCatalog("ar", source, source)).toContain("ar: missing plural n_few");
    expect(validateCatalog("ja", { ...source, title: "{wrong}" }, source)).toContain("ja: placeholders differ for title");
    for (const markup of ["<script>alert(1)</script>", "[link](javascript:alert)", "&lt;img&gt;", "**bold**"]) {
      expect(validateCatalog("en", { ...source, title: markup }, source)).toContain("en: markup forbidden in title");
    }
    expect(validateCatalog("en", {}, source)).toContain("en: missing title");
  });
});
describe("pseudo-locale regression guard", () => {
  it("expands every source message while preserving placeholders", () => {
    const pseudo = pseudoCatalog(en);
    for (const [key, value] of Object.entries(en)) {
      expect(pseudo[key], key).toMatch(/^⟦.*⟧$/s);
      expect(pseudo[key]!.length, key).toBeGreaterThan(value.length);
      expect(pseudo[key]!.match(/\{\w+\}/g), key).toEqual(value.match(/\{\w+\}/g));
    }
    expect(validateCatalog("en-XA", pseudo)).toEqual([]);
    useLocale.getState().setPreference("en-XA");
    const html = renderToStaticMarkup(createElement(Button, {}, t("common.back")));
    expect(html).toContain("⟦");
    expect(html).toContain("whitespace-normal");
  });
  it("catches hardcoded JSX text and accessible labels", () => {
    expect(hardcodedUi('<button title="Delete">Continue</button>')).toHaveLength(2);
    expect(scanHardcodedUi()).toEqual([]);
  });
  it("keeps the versioned English safety notice unchanged", () => {
    const safety = readFileSync(new URL("../../features/onboarding/safety.ts", import.meta.url), "utf8");
    expect(safety).toContain('SAFETY_TEXT_VERSION = 1');
    expect(safety).toContain('Hardware damage and overheating');
  });
});
it("wraps focus around the ends", () => {
  expect(wrapIndex(2, 3, false)).toBe(0);
  expect(wrapIndex(0, 3, true)).toBe(2);
  expect(wrapIndex(0, 0, false)).toBe(-1);
});
