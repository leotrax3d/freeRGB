import { t } from "./mainboardRgbMessages";
import { useEffect, useState } from "react";
import { Cpu } from "lucide-react";
import { Section } from "@/components/page";
import { Banner } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { COLOR_ORDERS, mainboardControllers, parseSettings, presetLeds, supportsHeaderOptions, withHeader, type ColorOrder, type Fallback, type MainboardRgbSettings, type MainboardRgbView } from "./mainboardRgbModel";

const fallbackMode = (f: Fallback | undefined) => f?.mode ?? "spectrum";
const selectClass = "h-8 rounded-control border border-border bg-surface-sunken px-2";

/** LED count and name per ARGB header of each motherboard RGB controller. */
export function MainboardRgb() {
  const devices = useDaemon((s) => s.devices);
  const [view, setView] = useState<MainboardRgbView | null>(null);
  const [draft, setDraft] = useState<MainboardRgbSettings | null>(null);
  const [fans, setFans] = useState(1);
  const [importError, setImportError] = useState<string | null>(null);
  const { pending, error, run } = useAction();
  const load = () => rpc<MainboardRgbView>("mainboard_rgb.get").then((v) => { setView(v); setDraft(v.settings); }, () => setView(null));
  useEffect(() => { void load(); }, []);
  if (!view || !draft) return null;
  const controllers = mainboardControllers(devices, draft);
  const changed = JSON.stringify(draft) !== JSON.stringify(view.settings);
  return (
    <Section title={t("mainboardRgb.mainboardRgb.motherboardRgb")}>
      <Checkbox checked={draft.enabled} onChange={(enabled) => setDraft({ ...draft, enabled })}>{t("mainboardRgb.mainboardRgb.manageMotherboardRgbArgbRgbHeaders")}</Checkbox>
      {view.vendor_software.length > 0 && (
        <Banner tone="warn">{t("mainboardRgb.vendorRunning", { names: view.vendor_software.join(", ") })}</Banner>
      )}
      {view.error && <Banner tone="error">{view.error}</Banner>}
      {draft.enabled && controllers.length === 0 && <p className="text-small text-text-muted">{t("mainboardRgb.mainboardRgb.noSupportedMotherboardRgbControllerWasFound")}</p>}
      {draft.enabled && controllers.map((c) => (
        <div key={c.key} className="flex flex-col gap-2 rounded-card border border-border p-3">
          <div className="flex items-center gap-2 text-small font-medium"><Cpu className="size-4" aria-hidden />{c.label}<span className="text-caption text-text-subtle">{c.key}{c.device ? "" : t("mainboardRgb.mainboardRgb.notDetected")}</span></div>
          {Array.from({ length: c.headers }, (_, i) => {
            const header = draft.controllers[c.key]?.[i] ?? { leds: 0 };
            return (
              <div key={i} className="flex flex-wrap items-center gap-2 text-small">
                <span className="w-24 text-text-muted">ARGB {i + 1}</span>
                <input aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueName", { p0: i + 1 })} placeholder={t("mainboardRgb.mainboardRgb.argbHeaderValue", { p0: i + 1 })} maxLength={40} value={header.name ?? ""}
                  onChange={(e) => setDraft(withHeader(draft, c.key, i, { name: e.target.value }, view.max_leds))}
                  className="h-8 w-40 rounded-control border border-border bg-surface-sunken px-2" />
                <input aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueLedCount", { p0: i + 1 })} type="number" min={0} max={view.max_leds} value={header.leds}
                  onChange={(e) => setDraft(withHeader(draft, c.key, i, { leds: Number(e.target.value) }, view.max_leds))}
                  className="h-8 w-20 rounded-control border border-border bg-surface-sunken px-2 tabular" />
                <span className="text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.ledsFanPreset")}</span>
                {view.fan_presets.map((per) => (
                  <Button key={per} size="sm" variant="ghost" onClick={() => setDraft(withHeader(draft, c.key, i, { leds: presetLeds(per, fans) }, view.max_leds))}>{fans > 1 ? `${fans}×` : ""}{per}</Button>
                ))}
                {supportsHeaderOptions(c.key) && (
                  <>
                    <label className="flex items-center gap-1 text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.colourOrder")} <select aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueColourOrder", { p0: i + 1 })} value={header.order ?? "rgb"} className={selectClass}
                        onChange={(e) => setDraft(withHeader(draft, c.key, i, { order: e.target.value as ColorOrder }, view.max_leds))}>
                        {COLOR_ORDERS.map((o) => <option key={o} value={o}>{o.toUpperCase()}</option>)}
                      </select>
                    </label>
                    <label className="flex items-center gap-1 text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.whenFreergbStops")} <select aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueFallback", { p0: i + 1 })} value={fallbackMode(header.fallback)} className={selectClass}
                        onChange={(e) => {
                          const mode = e.target.value;
                          const fallback: Fallback = mode === "static" ? { mode: "static", color: "#ffffff" } : { mode: mode as "spectrum" | "off" };
                          setDraft(withHeader(draft, c.key, i, { fallback }, view.max_leds));
                        }}>
                        <option value="spectrum">{t("mainboardRgb.mainboardRgb.firmwareSpectrum")}</option>
                        <option value="off">{t("mainboardRgb.mainboardRgb.off")}</option>
                        <option value="static">{t("mainboardRgb.mainboardRgb.staticColour")}</option>
                      </select>
                    </label>
                    {header.fallback?.mode === "static" && (
                      <input type="color" aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueFallbackColour", { p0: i + 1 })} value={header.fallback.color}
                        onChange={(e) => setDraft(withHeader(draft, c.key, i, { fallback: { mode: "static", color: e.target.value } }, view.max_leds))} />
                    )}
                    {view.detected?.[c.key]?.headers[i] && (
                      <span className="text-caption text-text-subtle">{view.detected[c.key]!.headers[i]!.present ? t("mainboardRgb.detected") : t("mainboardRgb.mainboardRgb.notReportedByController")}</span>
                    )}
                  </>
                )}
              </div>
            );
          })}
        </div>
      ))}
      {draft.enabled && controllers.length > 0 && (
        <label className="flex items-center gap-2 text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.fansChainedPerHeaderForPresets")} <input type="number" min={1} max={8} value={fans} onChange={(e) => setFans(Math.max(1, Math.min(8, Number(e.target.value) || 1)))} className="h-7 w-14 rounded-control border border-border bg-surface-sunken px-2" />
        </label>
      )}
      <div className="flex items-center gap-2">
        <Button size="sm" variant="primary" disabled={!changed || pending !== null} onClick={() => run("save", async () => { await rpc("mainboard_rgb.save", { settings: draft }); await load(); })}>{t("mainboardRgb.mainboardRgb.save")}</Button>
        {changed && <Button size="sm" variant="ghost" onClick={() => setDraft(view.settings)}>{t("mainboardRgb.mainboardRgb.discard")}</Button>}
        <Button size="sm" variant="ghost" onClick={() => {
          const url = URL.createObjectURL(new Blob([JSON.stringify(draft, null, 2)], { type: "application/json" }));
          const a = document.createElement("a");
          a.href = url; a.download = "mainboard-rgb.json"; a.click();
          URL.revokeObjectURL(url);
        }}>{t("mainboardRgb.mainboardRgb.export")}</Button>
        <label className="inline-flex cursor-pointer items-center text-small text-text-muted">
          {t("mainboardRgb.mainboardRgb.import")} <input type="file" accept="application/json,.json" aria-label={t("mainboardRgb.mainboardRgb.importMotherboardRgbSettings")} className="sr-only"
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.target.value = "";
              if (!file || file.size > 64 * 1024) { setImportError(t("mainboardRgb.mainboardRgb.fileTooLargeOrMissing")); return; }
              void file.text().then((text) => {
                const parsed = parseSettings(text, view.max_leds);
                if (parsed) { setDraft(parsed); setImportError(null); } else setImportError(t("mainboardRgb.mainboardRgb.notAValidFreergbMotherboardRgbFile"));
              });
            }} />
        </label>
        <span className="text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.headersWith0LedsKeepTheirFirmware")}</span>
      </div>
      {(error || importError) && <p role="alert" className="text-small text-danger-text">{error ?? importError}</p>}
    </Section>
  );
}
