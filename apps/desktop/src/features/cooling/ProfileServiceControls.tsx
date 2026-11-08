import { useEffect, useState } from "react";
import { Power } from "lucide-react";
import { Section } from "@/components/page";
import { StatusDot } from "@/components/status";
import { ConfirmButton } from "@/components/confirm-button";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import type { CoolingProfileImport, CoolingServiceState } from "@/lib/api/types";
import { t } from "./messages";
import { profileDocument } from "./profileServiceModel";

export function ProfileServiceControls() {
  const connected = useDaemon((s) => s.connection?.connected);
  const profiles = useDaemon((s) => s.cooling_profiles);
  const [selected, setSelected] = useState("");
  const [text, setText] = useState("");
  const [name, setName] = useState("");
  const [report, setReport] = useState<CoolingProfileImport | null>(null);
  const [service, setService] = useState<CoolingServiceState | null>(null);
  const [mode, setMode] = useState<CoolingServiceState["mode"]>("login");
  const [bootProfile, setBootProfile] = useState("");
  const { pending, error, run } = useAction();
  const load = async () => {
    const state = await rpc<CoolingServiceState>("cooling.service.get");
    setService(state); setMode(state.mode); setBootProfile(state.boot_profile ?? "");
  };
  useEffect(() => {
    let alive = true;
    setService(null);
    if (connected) void rpc<CoolingServiceState>("cooling.service.get").then((state) => {
      if (alive) { setService(state); setMode(state.mode); setBootProfile(state.boot_profile ?? ""); }
    }, () => {});
    return () => { alive = false; };
  }, [connected]);
  const busy = !connected || pending !== null;
  const names = profiles?.profiles.map((p) => p.name) ?? [];
  const stateLabels = {
    pending: t("cooling.profileService.pending"), resumed: t("cooling.profileService.resumed"),
    profile: t("cooling.profileService.profile"), failed: t("cooling.profileService.failed"), unavailable: t("cooling.profileService.unavailable"),
  };
  return <details className="text-small">
    <summary>{t("cooling.profileService.manage")}</summary>
    <div className="mt-2 flex flex-col gap-3">
      <Select label={t("cooling.fanOverview.profile")} value={selected} options={names.map((n) => ({ value: n, label: n }))} onChange={setSelected} />
      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="secondary" disabled={busy || !names.includes(selected)} onClick={() => run("export", async () => {
          const document = await rpc<unknown>("cooling.profiles.export", { name: selected });
          setText(JSON.stringify(document, null, 2)); setReport(null);
        })}>{t("cooling.profileService.export")}</Button>
        <Button size="sm" variant="ghost" disabled={busy || !text} onClick={() => run("copy", () => navigator.clipboard.writeText(text), t("cooling.profileService.copied"))}>{t("cooling.profileService.copy")}</Button>
      </div>
      <label className="flex flex-col gap-1 text-small">{t("cooling.profileService.json")}
        <textarea rows={6} className="w-full rounded-control border border-border bg-surface-sunken px-3 py-2" value={text} onChange={(e) => { setText(e.target.value); setReport(null); }} />
      </label>
      <label className="flex flex-col gap-1 text-small">{t("cooling.profileService.name")}
        <input maxLength={64} className="rounded-control border border-border bg-surface-sunken px-3 py-2" value={name} onChange={(e) => setName(e.target.value)} />
      </label>
      <Button size="sm" variant="secondary" disabled={busy || !text.trim()} onClick={() => run("import", async () => {
        setReport(null);
        setReport(await rpc<CoolingProfileImport>("cooling.profiles.import", { document: profileDocument(text), ...(name.trim() ? { name: name.trim() } : {}) }));
      }, t("cooling.profileService.imported"))}>{t("cooling.profileService.import")}</Button>
      {report && <div role="status" className="text-caption text-text-subtle">
        <p>{t("cooling.profileService.report", { name: report.name, count: report.report.remapped, same: report.report.same_board ? t("cooling.profileService.yes") : t("cooling.profileService.no") })}</p>
        {report.report.needs_confirmation && <p>{t("cooling.profileService.confirmation")}</p>}
        {report.report.unresolved.length > 0 && <p>{t("cooling.profileService.unresolved", { ids: report.report.unresolved.join(", ") })}</p>}
      </div>}
      <Section title={t("cooling.profileService.service")}>
        <p className="text-caption text-text-subtle">{t("cooling.profileService.semantics")}</p>
        <StatusDot tone={service?.error || service?.boot.state === "failed" ? "error" : "idle"} label={service ? (service.headless ? t("cooling.profileService.headless") : t("cooling.profileService.interactive")) : t("cooling.profileService.unavailable")} />
        {service && <>
          <p className="text-caption text-text-subtle">{t("cooling.profileService.boot", { state: stateLabels[service.boot.state] })}</p>
          <Select label={t("cooling.profileService.service")} value={mode} onChange={(v) => setMode(v as CoolingServiceState["mode"])} options={[{ value: "login", label: t("cooling.profileService.login") }, { value: "service", label: t("cooling.profileService.serviceMode") }]} />
          <Select label={t("cooling.profileService.bootProfile")} value={bootProfile ? "profile:" + bootProfile : "remembered"} onChange={(v) => setBootProfile(v === "remembered" ? "" : v.slice(8))} options={[{ value: "remembered", label: t("cooling.profileService.remembered") }, ...Array.from(new Set([...names, ...(bootProfile ? [bootProfile] : [])])).map((n) => ({ value: "profile:" + n, label: n }))]} />
          <ConfirmButton key={`${mode}:${bootProfile}`} icon={<Power />} label={t("cooling.profileService.apply")} confirmLabel={t("cooling.profileService.confirm")} disabled={busy || (mode === service.mode && bootProfile === (service.boot_profile ?? ""))} onConfirm={() => run("service", async () => {
            await rpc("cooling.service.set", { mode, boot_profile: bootProfile || null }); await load();
          }, t("cooling.profileService.saved"))} />
        </>}
        <Button size="sm" variant="ghost" disabled={busy} onClick={() => run("refresh", load)}>{t("cooling.profileService.refresh")}</Button>
        {service?.error && <p role="alert" className="text-small text-danger-text">{service.error}</p>}
      </Section>
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
    </div>
  </details>;
}
