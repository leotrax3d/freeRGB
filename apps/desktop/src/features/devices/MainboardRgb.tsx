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
  useEffect(() => {
    void load();
    const timer = setInterval(() => { void rpc<MainboardRgbView>("mainboard_rgb.get").then(setView, () => {}); }, 2000);
    return () => clearInterval(timer);
  }, []);
  if (!view || !draft) return <Banner tone="warn">{t("mainboardRgb.unavailable")}</Banner>;
  const controllers = mainboardControllers(devices, draft);
  const savedDraft: MainboardRgbSettings = { ...draft, controllers: { ...draft.controllers } };
  for (const [key, fixed] of Object.entries(view.fixed_headers ?? {})) {
    if (fixed.length) savedDraft.controllers[key] = fixed.map((h, i) => ({ ...h, ...draft.controllers[key]?.[i], leds: h.leds, type: h.type }));
  }
  const changed = JSON.stringify(savedDraft) !== JSON.stringify(view.settings);
  return (
    <Section title={t("mainboardRgb.headers")}>
      <Checkbox checked={draft.enabled} onChange={(enabled) => setDraft({ ...draft, enabled })}>{t("mainboardRgb.mainboardRgb.manageMotherboardRgbArgbRgbHeaders")}</Checkbox>
      {view.vendor_software.length > 0 && (
        <Banner tone="warn">{t("mainboardRgb.vendorRunning", { names: view.vendor_software.join(", ") })}</Banner>
      )}
      {view.error && <Banner tone="error">{view.error}</Banner>}
      {draft.enabled && controllers.length === 0 && <p className="text-small text-text-muted">{t("mainboardRgb.mainboardRgb.noSupportedMotherboardRgbControllerWasFound")}</p>}
      {draft.enabled && controllers.map((c) => (
        <div key={c.key} className="flex flex-col gap-2 rounded-card border border-border p-3">
          <div className="flex items-center gap-2 text-small font-medium"><Cpu className="size-4" aria-hidden />{c.label}<span className="text-caption text-text-subtle">{c.key}{c.device ? "" : t("mainboardRgb.mainboardRgb.notDetected")}</span></div>
          {Array.from({ length: view.fixed_headers?.[c.key]?.length ?? c.headers }, (_, i) => {
            const fixed = view.fixed_headers?.[c.key]?.[i];
            const header = { ...(fixed ?? { leds: 0 }), ...draft.controllers[c.key]?.[i], ...(fixed ? { leds: fixed.leds, type: fixed.type } : {}) };
            const edit = (patch: Partial<typeof header>) => setDraft(withHeader(draft, c.key, i, { ...header, ...patch }, view.max_leds));
            return (
              <div key={i} className="flex flex-wrap items-center gap-2 text-small">
                <span className="w-24 text-text-muted">{i + 1}</span>
                <select aria-label={t("mainboardRgb.type")} disabled={!!fixed} className={selectClass} value={header.type ?? "argb"} onChange={(e) => edit({ type: e.target.value as "rgb" | "argb" })}>
                  <option value="argb">{t("mainboardRgb.argb")}</option><option value="rgb">{t("mainboardRgb.rgb")}</option>
                </select>
                <input aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueName", { p0: i + 1 })} placeholder={t("mainboardRgb.mainboardRgb.argbHeaderValue", { p0: i + 1 })} maxLength={40} value={header.name ?? ""}
                  onChange={(e) => edit({ name: e.target.value })}
                  className="h-8 w-40 rounded-control border border-border bg-surface-sunken px-2" />
                <input aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueLedCount", { p0: i + 1 })} disabled={!!fixed} type="number" min={0} max={header.type === "rgb" ? 1 : view.max_leds} value={header.leds}
                  onChange={(e) => edit({ leds: Number(e.target.value) })}
                  className="h-8 w-20 rounded-control border border-border bg-surface-sunken px-2 tabular" />
                <span className="text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.ledsFanPreset")}</span>
                {view.fan_presets.map((per) => (
                  <Button key={per} disabled={!!fixed} size="sm" variant="ghost" onClick={() => edit({ leds: presetLeds(per, fans) })}>{fans > 1 ? `${fans}×` : ""}{per}</Button>
                ))}
                {supportsHeaderOptions(c.key) && (
                  <>
                    <label className="flex items-center gap-1 text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.colourOrder")} <select aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueColourOrder", { p0: i + 1 })} value={header.order ?? "rgb"} className={selectClass}
                        onChange={(e) => edit({ order: e.target.value as ColorOrder })}>
                        {COLOR_ORDERS.map((o) => <option key={o} value={o}>{o.toUpperCase()}</option>)}
                      </select>
                    </label>
                    {c.key.startsWith("0b05:") && <label className="flex items-center gap-1 text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.whenFreergbStops")} <select aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueFallback", { p0: i + 1 })} value={fallbackMode(header.fallback)} className={selectClass}
                        onChange={(e) => {
                          const mode = e.target.value;
                          const fallback: Fallback = mode === "static" ? { mode: "static", color: "#ffffff" } : { mode: mode as "spectrum" | "off" };
                          edit({ fallback });
                        }}>
                        <option value="spectrum">{t("mainboardRgb.mainboardRgb.firmwareSpectrum")}</option>
                        <option value="off">{t("mainboardRgb.mainboardRgb.off")}</option>
                        <option value="static">{t("mainboardRgb.mainboardRgb.staticColour")}</option>
                      </select>
                    </label>}
                    {header.fallback?.mode === "static" && (
                      <input type="color" aria-label={t("mainboardRgb.mainboardRgb.argbHeaderValueFallbackColour", { p0: i + 1 })} value={header.fallback.color}
                        onChange={(e) => edit({ fallback: { mode: "static", color: e.target.value } })} />
                    )}
                    {view.detected?.[c.key]?.headers[i] && (
                      <span className="text-caption text-text-subtle">{view.detected[c.key]!.headers[i]!.present ? t("mainboardRgb.detected") : t("mainboardRgb.mainboardRgb.notReportedByController")}</span>
                    )}
                  </>
                )}
                <fieldset className="flex flex-wrap items-center"><legend className="text-caption">{t("mainboardRgb.canvas")}</legend>
                  {(["x", "y", "w", "h"] as const).map((field) => <label key={field} className="ml-2">{field.toUpperCase()} <input aria-label={`${t("mainboardRgb.canvas")} ${field.toUpperCase()}`} type="number" min={field === "w" || field === "h" ? .01 : 0} max={1} step={.01} className="h-8 w-20 rounded-control border border-border bg-surface-sunken px-2" value={(header.region ?? { x: 0, y: 0, w: 1, h: 1 })[field]} onChange={(e) => edit({ region: { ...(header.region ?? { x: 0, y: 0, w: 1, h: 1 }), [field]: Number(e.target.value) } })} /></label>)}
                </fieldset>
                <Button size="sm" disabled={changed || !c.device || header.leds === 0 || pending !== null || view.vendor_software.length > 0} onClick={() => run("test", () => rpc("mainboard_rgb.test", { key: c.key, index: i }))}>{t("mainboardRgb.test")}</Button>
              </div>
            );
          })}
        </div>
      ))}
      <p className="text-small text-text-muted">{t("mainboardRgb.help")}</p>
      {draft.enabled && controllers.length > 0 && (
        <label className="flex items-center gap-2 text-caption text-text-subtle">{t("mainboardRgb.mainboardRgb.fansChainedPerHeaderForPresets")} <input type="number" min={1} max={8} value={fans} onChange={(e) => setFans(Math.max(1, Math.min(8, Number(e.target.value) || 1)))} className="h-7 w-14 rounded-control border border-border bg-surface-sunken px-2" />
        </label>
      )}
      <div className="flex items-center gap-2">
        <Button size="sm" variant="primary" disabled={!changed || pending !== null} onClick={() => run("save", async () => { await rpc("mainboard_rgb.save", { settings: savedDraft }); await load(); })}>{t("mainboardRgb.mainboardRgb.save")}</Button>
        {changed && <Button size="sm" variant="ghost" onClick={() => setDraft(view.settings)}>{t("mainboardRgb.mainboardRgb.discard")}</Button>}
        <Button size="sm" variant="ghost" onClick={() => {
          const url = URL.createObjectURL(new Blob([JSON.stringify(savedDraft, null, 2)], { type: "application/json" }));
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
