import en from "@/lib/i18n/locales/en.json";
import de from "@/lib/i18n/locales/de.json";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import type { DaemonState } from "@/lib/store/daemon";
import { useLocale } from "@/lib/i18n";
import { mockTransport } from "@/lib/mock/transport";
import { curveProblem, DEFAULT_CURVE, macroProblem } from "@/lib/api/control";
import CoolingView from "./cooling/CoolingView";
import MacrosView from "./macros/MacrosView";
import { roleUnknownLabel, SETUP_STEPS } from "./cooling/setupModel";
import { stateLabel } from "./cooling/labels";
import { liveChannels } from "./cooling/liveModel";
import { t as mainboard } from "./devices/mainboardRgbMessages";
import { tn } from "./cooling/messages";
import { STEP_TYPES } from "./macros/steps";

const { state, rpc } = vi.hoisted(() => ({ state: { loading: false, mode: "mock" } as DaemonState, rpc: vi.fn() }));
vi.mock("@/lib/store/daemon", () => ({
  useDaemon: (select: (value: DaemonState) => unknown) => select(state),
  useSlices: vi.fn(),
  rpc,
}));

afterEach(() => { useLocale.getState().setPreference("en"); vi.clearAllMocks(); });

describe("cooling, macros and motherboard RGB localization", () => {
  it("renders the cooling and macro views in both languages using simulated state only", async () => {
    Object.assign(state, await mockTransport().snapshot());
    for (const locale of ["en", "de"] as const) {
      useLocale.getState().setPreference(locale);
      const cooling = renderToStaticMarkup(<CoolingView />);
      const macros = renderToStaticMarkup(<MacrosView />);
      expect(cooling).toContain(locale === "en" ? "Live motherboard fan control" : "Live-Steuerung der Mainboard-Lüfter");
      expect(cooling).toContain(locale === "en" ? "2 of 16 channels under live control" : "2 von 16 Anschlüssen unter Live-Steuerung");
      expect(macros).toContain(locale === "en" ? "Step sequences and keyboard shortcuts." : "Schrittfolgen und Tastenkürzel.");
      expect(macros).toContain(locale === "en" ? "New macro" : "Neues Makro");
      expect(macros).toContain(locale === "en" ? 'aria-label="Brightness"' : 'aria-label="Helligkeit"');
      expect(STEP_TYPES.find((s) => s.type === "brightness")!.label).toBe(locale === "en" ? "Set brightness" : "Helligkeit setzen");
      expect(cooling + macros).not.toMatch(/\{p\d+\}|cooling\.[a-z]|macros\.[a-z]/);
    }
    expect(rpc).not.toHaveBeenCalled();
  });

  it("renders full simulated views with expanded pseudo messages", async () => {
    Object.assign(state, await mockTransport().snapshot());
    useLocale.getState().setPreference("en-XA");
    for (const view of [<CoolingView />, <MacrosView />]) {
      const html = renderToStaticMarkup(view);
      expect(html).toContain("\u27e6");
      expect(html).toContain("~");
      expect(html).not.toMatch(/\{p\d+\}|cooling\.[a-z]|macros\.[a-z]/);
    }
    expect(rpc).not.toHaveBeenCalled();
  });

  it("updates helper labels and plurals after the feature modules are already loaded", () => {
    useLocale.getState().setPreference("en");
    expect(roleUnknownLabel()).toBe("100 % (role unknown)");
    expect(SETUP_STEPS[0]!.label).toBe("Detect");
    useLocale.getState().setPreference("de");
    expect(roleUnknownLabel()).toBe("100 % (Rolle unbekannt)");
    expect(SETUP_STEPS[0]!.label).toBe("Erkennen");
    expect(tn("cooling.setup.controls", 1)).toBe("1 steuerbarer Anschluss");
    expect(tn("cooling.setup.controls", 2)).toBe("2 steuerbare Anschlüsse");
    expect(tn("cooling.setup.sensors", 2)).toBe("2 Drehzahlsensoren");
    expect(stateLabel("stopping")).toBe("Wird gestoppt");
    expect(stateLabel("future_state")).toBe("future_state");
    const rows = liveChannels({ available: true, job_id: 1, control_id: "mock-fan", state: "executing", mode: "calibration", report: null, error: null }, null);
    expect(rows[0]!.label).toBe("Kalibrierung läuft");
  });

  it("localizes validation and ARGB labels without changing user or device identifiers", () => {
    useLocale.getState().setPreference("de");
    expect(curveProblem({ ...DEFAULT_CURVE, minimum_percent: -1 })).toBe([de["validation.minimumDuty"], "", `English: ${en["validation.minimumDuty"]}`].join("\n"));
    expect(macroProblem({ repeat: 1, steps: [{ type: "key_up", key: "a" }] })).toBe("Schritt 1: Loslassen ohne vorheriges Drücken.");
    expect(mainboard("mainboardRgb.mainboardRgb.argbHeaderValueName", { p0: 2 })).toBe("Name von ARGB-Anschluss 2");
    expect(mainboard("mainboardRgb.vendorRunning", { names: "Vendor App" })).toContain("Vendor App läuft");
  });
});
