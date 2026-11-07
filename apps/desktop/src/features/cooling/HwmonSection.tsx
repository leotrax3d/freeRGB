import { t } from "./messages";
import { stateLabel } from "./labels";
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
  { value: "firmware", get label() { return t("cooling.hwmonSection.firmwareBios"); } },
  { value: "manual", get label() { return t("cooling.hwmonSection.fixedDuty"); } },
  { value: "curve", get label() { return t("cooling.hwmonSection.curve"); } },
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
    <Section title={t("cooling.hwmonSection.motherboardFansLinux")}>
      <p className="text-small text-text-muted">
        {board ? `${board.manufacturer} ${board.product} · BIOS ${board.bios_version} · ` : ""}
        {t("cooling.hwmonSection.usesTheKernelSMotherboardSensorDrivers")} {view.floor_percent}{t("cooling.hwmonSection.minimumUnlessUnrestrictedControlAllowsFanStop")} </p>
      {view.paused && <Banner tone="warn">{t("cooling.hwmonSection.fanControlIsPausedAllHeadersFollow")}</Banner>}
      {headers.length === 0 && (
        <Banner tone="info">{t("cooling.hwmonSection.noFanHeadersFoundLoadYourBoard")} <code>sudo modprobe nct6775</code> {t("cooling.hwmonSection.or")} <code>sudo modprobe it87</code>{t("cooling.hwmonSection.thenReopenThisPage")}</Banner>
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
              {status && <span className={status.state === "faulted" ? "text-caption text-danger-text" : "text-caption text-text-subtle"}>{stateLabel(status.state)}{status.message ? `: ${status.message}` : ""}</span>}
            </div>
            {!pwm.writable ? (
              <p className="text-caption text-text-subtle">{t("cooling.hwmonSection.readOnlyInstall")} <code>packaging/linux/60-freergb.rules</code> {t("cooling.hwmonSection.forWriteAccess")}</p>
            ) : (
              <div className="flex flex-wrap items-center gap-2 text-small">
                <Select label={t("cooling.hwmonSection.valueMode", { p0: pwm.label })} value={d.mode} options={MODES} onChange={(mode) => edit(id, { mode })} />
                {d.mode === "manual" && (
                  <label className="flex items-center gap-2">
                    <input type="range" min={0} max={100} value={d.percent} aria-label={t("cooling.hwmonSection.valueDuty", { p0: pwm.label })} onChange={(e) => edit(id, { percent: Number(e.target.value) })} />
                    <span className="tabular w-10">{d.percent}%</span>
                  </label>
                )}
                {d.mode === "curve" && (
                  <>
                    <Select label={t("cooling.hwmonSection.valueTemperature", { p0: pwm.label })} value={d.sensor} options={temps} onChange={(sensor) => edit(id, { sensor })} placeholder={t("cooling.hwmonSection.temperature")} />
                    <Select label={t("cooling.hwmonSection.valueCurve", { p0: pwm.label })} value={d.curve} options={curveNames.map((n) => ({ value: n, label: n }))} onChange={(curve) => edit(id, { curve })} placeholder={t("cooling.hwmonSection.curve")} />
                  </>
                )}
                {d.mode !== "firmware" && <Checkbox checked={d.pump} onChange={(pump) => edit(id, { pump })}>{t("cooling.hwmonSection.pump")}</Checkbox>}
                {d.mode !== "firmware" && view.unrestricted && <Checkbox checked={d.allowStop} onChange={(allowStop) => edit(id, { allowStop })}>{t("cooling.hwmonSection.allowFanStop")}</Checkbox>}
                <Button size="sm" variant="primary" disabled={!request || pending !== null}
                  onClick={() => request && run(id, async () => { await rpc(request.method, request.params); const next = { ...drafts }; delete next[id]; setDrafts(next); await load(); })}>{t("cooling.hwmonSection.apply")}</Button>
              </div>
            )}
          </div>
        );
      })}
      {view.assignments.length > 0 && (
        <Button size="sm" variant="ghost" onClick={() => run("release", async () => { await rpc("cooling.hwmon.release", {}); setDrafts({}); await load(); })}>{t("cooling.hwmonSection.returnAllHeadersToBios")}</Button>
      )}
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
    </Section>
  );
}
