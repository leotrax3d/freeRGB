import { t } from "./messages";
import { stateLabel } from "./labels";
import { useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import type { ChannelBinding } from "@/lib/api/types";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";

export function ChannelBindings() {
  useSlices("cooling_bindings", "cooling_channels");
  const saved = useDaemon((s) => s.cooling_bindings);
  const channels = useDaemon((s) => s.cooling_channels) ?? [];
  const connected = useDaemon((s) => s.connection?.connected);
  const [draft, setDraft] = useState<ChannelBinding[] | null>(null);
  const bindings = draft ?? saved?.config.bindings ?? [];
  const { pending, error, run } = useAction();
  const edit = (index: number, patch: Partial<ChannelBinding>) => setDraft(bindings.map((b, i) => i === index ? { ...b, ...patch } : b));
  return <Section title={t("cooling.channelBindings.physicalChannelAnnotations")}>
    <p className="text-small text-text-muted">{t("cooling.channelBindings.nameAPhysicalHeaderAndAssignIts")}</p>
    {saved == null && <p className="text-small text-text-subtle">{t("cooling.channelBindings.channelAssignmentsUnavailable")}</p>}
    {bindings.map((b, index) => <div key={b.control_id} className="flex flex-col gap-3 rounded-card bg-surface-sunken p-4">
      <div className="flex items-center justify-between gap-3 text-small"><span className="break-all">{b.control_id}</span><Button variant="ghost" onClick={() => setDraft(bindings.filter((_, i) => i !== index))}>{t("cooling.channelBindings.remove")}</Button></div>
      <div className="grid gap-3 md:grid-cols-2">
        <label className="text-small">{t("cooling.channelBindings.headerName")} <input aria-label={t("cooling.channelBindings.headerNameValue", { p0: index + 1 })} className="w-full rounded-control border border-border bg-surface p-2" maxLength={64} value={b.header} onChange={(e) => edit(index, { header: e.target.value })} /></label>
        <Select label={t("cooling.channelBindings.channelRole")} value={b.role} options={[{ value: "fan", label: t("cooling.channelBindings.fan") }, { value: "pump", label: t("cooling.channelBindings.pump") }]} onChange={(role) => edit(index, { role })} />
        <Select label={t("cooling.channelBindings.electricalModeAnnotation")} value={b.electrical_mode} options={[{ value: "unknown", label: t("cooling.channelBindings.unknownMode") }, { value: "pwm", label: t("cooling.channelBindings.pwmAnnotationOnly") }, { value: "dc", label: t("cooling.channelBindings.dcAnnotationOnly") }]} onChange={(electrical_mode) => edit(index, { electrical_mode })} />
        <Select label={t("cooling.channelBindings.assignedRpmSensor")} value={b.tachometer_id ?? "none"} options={[{ value: "none", label: t("cooling.channelBindings.noRpmAssigned") }, ...channels.filter((c) => c.kind === "fan" && c.provider === b.provider && c.device_id === b.device_id).map((c) => ({ value: c.id, label: c.name }))]} onChange={(id) => edit(index, { tachometer_id: id === "none" ? null : id })} />
      </div>
      <p className="text-caption text-text-subtle">{t("cooling.channelBindings.savedStatus")} {stateLabel(saved?.assignments.find((a) => a.control_id === b.control_id)?.state ?? "unsaved")} {t("cooling.channelBindings.hardwareUnverified")}</p>
    </div>)}
    <div className="flex flex-wrap gap-2">
      {channels.filter((c) => c.kind === "control" && c.control_available && !bindings.some((b) => b.control_id === c.id)).map((c) => <Button key={c.id} variant="secondary" disabled={!saved || bindings.length >= 128} onClick={() => setDraft([...bindings, { provider: c.provider, device_id: c.device_id, control_id: c.id, tachometer_id: null, header: "", role: "fan", electrical_mode: "unknown" }])}>{t("cooling.channelBindings.assign")} {c.name}</Button>)}
    </div>
    <div className="flex gap-2">
      <Button disabled={!connected || !saved || !draft || pending !== null || bindings.some((b) => !b.header.trim())} onClick={() => run("save", async () => { await rpc("cooling.bindings.save", { config: { version: 1, bindings } }); setDraft(null); }, t("cooling.channelBindings.channelAnnotationsSaved"))}>{t("cooling.channelBindings.saveAssignments")}</Button>
      <Button variant="ghost" disabled={!draft} onClick={() => setDraft(null)}>{t("cooling.channelBindings.discardEdits")}</Button>
    </div>
    {(error || saved?.error) && <p role="alert" className="text-small text-danger-text">{error || saved?.error}</p>}
    {saved && !saved.persistence_enabled && <p className="text-caption text-text-subtle">{t("cooling.channelBindings.theDaemonIsRunningWithoutPersistentStorage")}</p>}
  </Section>;
}
