import { t } from "./messages";
import { stateLabel } from "./labels";
import { useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { Banner } from "@/components/status";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";

/** Explicit, revocable full control for untested hardware (DECISIONS.md #14). */
export function UnrestrictedSection() {
  useSlices("cooling_support", "cooling_channels");
  const support = useDaemon((s) => s.cooling_support);
  const channels = useDaemon((s) => s.cooling_channels);
  const connected = useDaemon((s) => s.connection?.connected);
  const [control, setControl] = useState("");
  const [typed, setTyped] = useState("");
  const [understood, setUnderstood] = useState(false);
  const { pending, error, run } = useAction();
  const controls = (channels ?? []).filter((c) => c.kind === "control" && c.control_available);
  const selected = controls.find((c) => c.reading.id === control);
  const state = support?.channels[control]?.unlock ?? "none";
  const busy = !connected || pending !== null;
  return (
    <Section title={t("cooling.unrestrictedSection.unrestrictedControl")}>
      <Banner tone="warn">{t("cooling.unrestrictedSection.forHardwareFreergbHasNotTestedAn")}</Banner>
      <Checkbox checked={support?.unrestricted_enabled ?? false} onChange={(v) => run("switch", () => rpc("cooling.support.settings", { unrestricted_enabled: v }))}>{t("cooling.unrestrictedSection.allowUnrestrictedControlOffByDefaultTurning")}</Checkbox>
      {support?.unrestricted_enabled && <>
        {!support.board_identity_available && <p className="text-small text-text-subtle">{t("cooling.unrestrictedSection.theProtectedServiceReportsNoBoardIdentity")}</p>}
        <div className="grid gap-3 md:grid-cols-2">
          <Select label={t("cooling.unrestrictedSection.channel")} placeholder={t("cooling.unrestrictedSection.channel")} value={control} options={controls.map((c) => ({ value: c.reading.id, label: `${c.name} · ${stateLabel(support.channels[c.reading.id]?.unlock ?? "none")}` }))} onChange={(v) => { setControl(v); setTyped(""); setUnderstood(false); }} />
          <label className="flex flex-col gap-1 text-small">
            <span>{state === "unlocked" ? t("cooling.unrestrictedSection.pumpConfirmationTypePumpValue", { p0: selected?.name ?? "" }) : t("cooling.unrestrictedSection.typeTheChannelNameValueToConfirm", { p0: selected?.name ?? "" })}</span>
            <input className="rounded-control border border-border bg-surface-sunken px-3 py-2" value={typed} onChange={(e) => setTyped(e.target.value)} aria-label={t("cooling.unrestrictedSection.confirmation")} />
          </label>
        </div>
        <Checkbox checked={understood} onChange={setUnderstood}>{t("cooling.unrestrictedSection.iUnderstandTheRisksForThisChannel")}</Checkbox>
        <div className="flex flex-wrap gap-2">
          <Button disabled={busy || !selected || !understood || typed.trim() !== selected.name.trim() || state === "unlocked" || state === "unlocked_pump"} onClick={() => run("unlock", () => rpc("cooling.channels.unlock", { control_id: control, confirmation: typed }))}>{t("cooling.unrestrictedSection.unlockChannel")}</Button>
          <Button variant="secondary" disabled={busy || !selected || !understood || state !== "unlocked" || typed.trim() !== `PUMP ${selected.name.trim()}`} onClick={() => run("pump", () => rpc("cooling.channels.confirm_pump", { control_id: control, confirmation: typed }))}>{t("cooling.unrestrictedSection.confirmPumpControl")}</Button>
          <Button variant="secondary" disabled={busy || !selected || state === "none"} onClick={() => run("revoke", () => rpc("cooling.channels.revoke", { control_id: control }))}>{t("cooling.unrestrictedSection.revoke")}</Button>
        </div>
        <p className="text-caption text-text-subtle">{t("cooling.unrestrictedSection.unlocksAreBoundToThisBoardIts")}</p>
      </>}
      {(error || support?.error) && <p role="alert" className="text-small text-danger-text">{error || support?.error}</p>}
    </Section>
  );
}
