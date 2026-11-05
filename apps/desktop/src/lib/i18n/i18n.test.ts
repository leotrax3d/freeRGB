import { beforeEach, describe, expect, it } from "vitest";
// Feature catalogs register themselves on import.
import "@/features/devices/messages";
import "@/features/layout/messages";
import "@/features/lighting/messages";
import "@/features/marketplace/messages";
import "@/features/monitoring/messages";
import "@/features/onboarding/messages";
import "@/features/profiles/messages";
import "@/features/settings/messages";
import "@/features/setup/messages";
import { defineMessages, pickLocale, registeredKeys, resolveLocale, t, tn, useLocale } from "./index";
import { wrapIndex } from "../a11y";

describe("locale selection", () => {
  it("maps browser language tags", () => {
    expect(pickLocale(["de-AT", "en"])).toBe("de");
    expect(pickLocale(["fr-FR", "en-GB"])).toBe("en");
    expect(pickLocale(["fr"])).toBe("en");
    expect(pickLocale([])).toBe("en");
  });

  it("an explicit preference beats the browser", () => {
    expect(resolveLocale("de", ["en-US"])).toBe("de");
    expect(resolveLocale("auto", ["de-DE"])).toBe("de");
  });
});

describe("translation", () => {
  beforeEach(() => useLocale.getState().setPreference("en"));

  it("interpolates parameters and keeps unknown placeholders", () => {
    expect(t("palette.running", { name: "Rainbow" })).toBe("Rainbow is running");
    expect(t("palette.running")).toBe("{name} is running");
  });

  it("switches language at runtime", () => {
    useLocale.getState().setPreference("de");
    expect(t("common.back")).toBe("Zurück");
  });

  it("picks plural forms", () => {
    expect(tn("shell.conflict", 1, { names: "Armoury Crate" })).toMatch(/is running/);
    expect(tn("shell.conflict", 2, { names: "A, B" })).toMatch(/are running/);
    useLocale.getState().setPreference("de");
    expect(tn("shell.conflict", 1, { names: "X" })).toMatch(/läuft/);
  });

  it("falls back to English, then to the key", () => {
    const m = defineMessages("fallbacktest", { en: { "fallbacktest.a": "A" }, de: { "fallbacktest.a": "Ä" } });
    useLocale.getState().setPreference("de");
    expect(m.t("fallbacktest.a")).toBe("Ä");
    expect(t("nope" as never)).toBe("nope");
  });

  it("rejects feature keys outside their prefix", () => {
    expect(() => defineMessages("abc", { en: { "xyz.a": "A" }, de: { "xyz.a": "A" } })).toThrow();
  });
});

describe("catalog parity", () => {
  it("English and German define exactly the same keys", () => {
    expect(registeredKeys("de").sort()).toEqual(registeredKeys("en").sort());
  });

  it("every message keeps its placeholders in the translation", () => {
    const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
    useLocale.getState().setPreference("en");
    const en = Object.fromEntries(registeredKeys("en").map((k) => [k, t(k as never, {})]));
    useLocale.getState().setPreference("de");
    for (const key of registeredKeys("de")) {
      const german = t(key as never, {});
      expect(placeholders(german), key).toEqual(placeholders(en[key]!));
    }
  });
});

describe("focus helpers", () => {
  it("wraps focus around the ends", () => {
    expect(wrapIndex(2, 3, false)).toBe(0);
    expect(wrapIndex(0, 3, true)).toBe(2);
    expect(wrapIndex(-1, 3, false)).toBe(0);
    expect(wrapIndex(-1, 3, true)).toBe(2);
    expect(wrapIndex(0, 0, false)).toBe(-1);
  });
});
