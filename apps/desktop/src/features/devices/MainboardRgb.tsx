import { useEffect, useState } from "react";
import { Cpu } from "lucide-react";
import { Section } from "@/components/page";
import { Banner } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { mainboardControllers, presetLeds, withHeader, type MainboardRgbSettings, type MainboardRgbView } from "./mainboardRgbModel";

/** LED count and name per ARGB header of each motherboard RGB controller. */
export function MainboardRgb() {
  const devices = useDaemon((s) => s.devices);
  const [view, setView] = useState<MainboardRgbView | null>(null);
  const [draft, setDraft] = useState<MainboardRgbSettings | null>(null);
  const [fans, setFans] = useState(1);
  const { pending, error, run } = useAction();
  const load = () => rpc<MainboardRgbView>("mainboard_rgb.get").then((v) => { setView(v); setDraft(v.settings); }, () => setView(null));
  useEffect(() => { void load(); }, []);
  if (!view || !draft) return null;
  const controllers = mainboardControllers(devices, draft);
  const changed = JSON.stringify(draft) !== JSON.stringify(view.settings);
  return (
    <Section title="Motherboard RGB">
      <Checkbox checked={draft.enabled} onChange={(enabled) => setDraft({ ...draft, enabled })}>Manage motherboard RGB (ARGB/RGB headers)</Checkbox>
      {view.vendor_software.length > 0 && (
        <Banner tone="warn">{view.vendor_software.join(", ")} is running and controls the motherboard lighting. freeRGB leaves the motherboard alone until it is closed.</Banner>
      )}
      {view.error && <Banner tone="error">{view.error}</Banner>}
      {draft.enabled && controllers.length === 0 && <p className="text-small text-text-muted">No supported motherboard RGB controller was found (ASUS Aura USB, Gigabyte RGB Fusion 2 USB).</p>}
      {draft.enabled && controllers.map((c) => (
        <div key={c.key} className="flex flex-col gap-2 rounded-card border border-border p-3">
          <div className="flex items-center gap-2 text-small font-medium"><Cpu className="size-4" aria-hidden />{c.label}<span className="text-caption text-text-subtle">{c.key}{c.device ? "" : " · not detected"}</span></div>
          {Array.from({ length: c.headers }, (_, i) => {
            const header = draft.controllers[c.key]?.[i] ?? { leds: 0 };
            return (
              <div key={i} className="flex flex-wrap items-center gap-2 text-small">
                <span className="w-24 text-text-muted">ARGB {i + 1}</span>
                <input aria-label={`ARGB header ${i + 1} name`} placeholder={`ARGB header ${i + 1}`} maxLength={40} value={header.name ?? ""}
                  onChange={(e) => setDraft(withHeader(draft, c.key, i, { name: e.target.value }, view.max_leds))}
                  className="h-8 w-40 rounded-control border border-border bg-surface-sunken px-2" />
                <input aria-label={`ARGB header ${i + 1} LED count`} type="number" min={0} max={view.max_leds} value={header.leds}
                  onChange={(e) => setDraft(withHeader(draft, c.key, i, { leds: Number(e.target.value) }, view.max_leds))}
                  className="h-8 w-20 rounded-control border border-border bg-surface-sunken px-2 tabular" />
                <span className="text-caption text-text-subtle">LEDs · fan preset:</span>
                {view.fan_presets.map((per) => (
                  <Button key={per} size="sm" variant="ghost" onClick={() => setDraft(withHeader(draft, c.key, i, { leds: presetLeds(per, fans) }, view.max_leds))}>{fans > 1 ? `${fans}×` : ""}{per}</Button>
                ))}
              </div>
            );
          })}
        </div>
      ))}
      {draft.enabled && controllers.length > 0 && (
        <label className="flex items-center gap-2 text-caption text-text-subtle">Fans chained per header for presets
          <input type="number" min={1} max={8} value={fans} onChange={(e) => setFans(Math.max(1, Math.min(8, Number(e.target.value) || 1)))} className="h-7 w-14 rounded-control border border-border bg-surface-sunken px-2" />
        </label>
      )}
      <div className="flex items-center gap-2">
        <Button size="sm" variant="primary" disabled={!changed || pending !== null} onClick={() => run("save", async () => { await rpc("mainboard_rgb.save", { settings: draft }); await load(); })}>Save</Button>
        {changed && <Button size="sm" variant="ghost" onClick={() => setDraft(view.settings)}>Discard</Button>}
        <span className="text-caption text-text-subtle">Headers with 0 LEDs keep their firmware effect. Lighting returns to the firmware effect when freeRGB stops.</span>
      </div>
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
    </Section>
  );
}
