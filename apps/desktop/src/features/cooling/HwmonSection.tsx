import { useEffect, useState } from "react";
import { Fan } from "lucide-react";
import { Section } from "@/components/page";
import { Banner } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Select } from "@/components/ui/select";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { hwmonRequest, hwmonTemperatures, type HwmonMode, type HwmonView } from "./hwmonModel";

interface Draft { mode: HwmonMode; percent: number; sensor: string; curve: string; pump: boolean; allowStop: boolean }

const MODES = [
  { value: "firmware", label: "Firmware (BIOS)" },
  { value: "manual", label: "Fixed duty" },
  { value: "curve", label: "Curve" },
] as const;

/** Linux: every motherboard fan header the kernel hwmon drivers expose. */
export function HwmonSection() {
  const curves = useDaemon((s) => s.curves);
  const [view, setView] = useState<HwmonView | null>(null);
  const [drafts, setDrafts] = useState<Record<string, Draft>>({});
  const { pending, error, run } = useAction();
  const load = () => rpc<HwmonView>("cooling.hwmon.get").then(setView, () => setView(null));
  useEffect(() => {
    void load();
    const timer = setInterval(() => void load(), 2000);
    return () => clearInterval(timer);
  }, []);
  if (!view?.started) return null;
  const temps = hwmonTemperatures(view);
  const curveNames = Object.keys(curves ?? {});
  const headers = view.chips.flatMap((c) => c.pwms.map((p) => ({ chip: c, pwm: p })));
  const draftFor = (id: string): Draft => {
    if (drafts[id]) return drafts[id];
    const a = view.assignments.find((x) => x.channel_id === id);
    if (a?.mode === "manual") return { mode: "manual", percent: a.percent, sensor: temps[0]?.value ?? "", curve: curveNames[0] ?? "", pump: a.pump, allowStop: a.allow_stop };
    if (a?.mode === "curve") return { mode: "curve", percent: 60, sensor: a.sensor_id, curve: curveNames.find((n) => JSON.stringify(curves?.[n]) === JSON.stringify(a.curve)) ?? curveNames[0] ?? "", pump: a.pump, allowStop: a.allow_stop };
    return { mode: "firmware", percent: 60, sensor: temps[0]?.value ?? "", curve: curveNames[0] ?? "", pump: false, allowStop: false };
  };
  const edit = (id: string, patch: Partial<Draft>) => setDrafts({ ...drafts, [id]: { ...draftFor(id), ...patch } });
  const board = view.board;
  return (
    <Section title="Motherboard fans (Linux)">
      <p className="text-small text-text-muted">
        {board ? `${board.manufacturer} ${board.product} · BIOS ${board.bios_version} · ` : ""}
        Uses the kernel's motherboard sensor drivers (nct6775, it87, nct6683, f71882fg, …). Fans keep a {view.floor_percent}% minimum unless unrestricted control allows fan stop; missing temperatures drive 100%.
      </p>
      {view.paused && <Banner tone="warn">Fan control is paused; all headers follow the BIOS.</Banner>}
      {headers.length === 0 && (
        <Banner tone="info">No fan headers found. Load your board's sensor driver, e.g. <code>sudo modprobe nct6775</code> or <code>sudo modprobe it87</code>, then reopen this page.</Banner>
      )}
      {headers.map(({ chip, pwm }) => {
        const id = pwm.channel_id;
        const d = draftFor(id);
        const status = view.status.find((s) => s.channel_id === id);
        const rpm = chip.fans.find((f) => f.index === pwm.paired_fan)?.rpm;
        const request = hwmonRequest(id, d.mode, d.percent, d.sensor, curves?.[d.curve], d.pump, d.allowStop);
        return (
          <div key={id} className="flex flex-col gap-2 rounded-card border border-border p-3">
            <div className="flex flex-wrap items-center gap-2 text-small font-medium">
              <Fan className="size-4" aria-hidden />{pwm.label}
              <span className="text-caption text-text-subtle">{chip.name} · {pwm.percent ?? "–"}%{rpm != null ? ` · ${Math.round(rpm)} RPM` : ""}</span>
              {status && <span className={status.state === "faulted" ? "text-caption text-danger-text" : "text-caption text-text-subtle"}>{status.state}{status.message ? `: ${status.message}` : ""}</span>}
            </div>
            {!pwm.writable ? (
              <p className="text-caption text-text-subtle">Read-only: install <code>packaging/linux/60-freergb.rules</code> for write access.</p>
            ) : (
              <div className="flex flex-wrap items-center gap-2 text-small">
                <Select label={`${pwm.label} mode`} value={d.mode} options={MODES} onChange={(mode) => edit(id, { mode })} />
                {d.mode === "manual" && (
                  <label className="flex items-center gap-2">
                    <input type="range" min={0} max={100} value={d.percent} aria-label={`${pwm.label} duty`} onChange={(e) => edit(id, { percent: Number(e.target.value) })} />
                    <span className="tabular w-10">{d.percent}%</span>
                  </label>
                )}
                {d.mode === "curve" && (
                  <>
                    <Select label={`${pwm.label} temperature`} value={d.sensor} options={temps} onChange={(sensor) => edit(id, { sensor })} placeholder="Temperature" />
                    <Select label={`${pwm.label} curve`} value={d.curve} options={curveNames.map((n) => ({ value: n, label: n }))} onChange={(curve) => edit(id, { curve })} placeholder="Curve" />
                  </>
                )}
                {d.mode !== "firmware" && <Checkbox checked={d.pump} onChange={(pump) => edit(id, { pump })}>Pump</Checkbox>}
                {d.mode !== "firmware" && view.unrestricted && <Checkbox checked={d.allowStop} onChange={(allowStop) => edit(id, { allowStop })}>Allow fan stop</Checkbox>}
                <Button size="sm" variant="primary" disabled={!request || pending !== null}
                  onClick={() => request && run(id, async () => { await rpc(request.method, request.params); const next = { ...drafts }; delete next[id]; setDrafts(next); await load(); })}>Apply</Button>
              </div>
            )}
          </div>
        );
      })}
      {view.assignments.length > 0 && (
        <Button size="sm" variant="ghost" onClick={() => run("release", async () => { await rpc("cooling.hwmon.release", {}); setDrafts({}); await load(); })}>Return all headers to BIOS</Button>
      )}
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
    </Section>
  );
}
