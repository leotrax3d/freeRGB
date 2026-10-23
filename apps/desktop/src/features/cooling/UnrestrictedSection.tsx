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
    <Section title="Unrestricted control">
      <Banner tone="warn">For hardware freeRGB has not tested. An unlocked channel needs no channel test, may run down to 0 % (fan stop) and only warns about implausible RPM; a wrong LibreHardwareMonitor mapping can address the wrong header. Still always active: return to firmware on stop, crash, lost telemetry and watchdog expiry, 100 % at critical temperature, no takeover of a header another program controls.</Banner>
      <Checkbox checked={support?.unrestricted_enabled ?? false} onChange={(v) => run("switch", () => rpc("cooling.support.settings", { unrestricted_enabled: v }))}>Allow unrestricted control (off by default; turning it off disables every unlock and re-validates background channels)</Checkbox>
      {support?.unrestricted_enabled && <>
        {!support.board_identity_available && <p className="text-small text-text-subtle">The protected service reports no board identity; unlocks are unavailable until it is updated.</p>}
        <div className="grid gap-3 md:grid-cols-2">
          <Select label="Channel" placeholder="Channel" value={control} options={controls.map((c) => ({ value: c.reading.id, label: `${c.name} · ${support.channels[c.reading.id]?.unlock ?? "none"}` }))} onChange={(v) => { setControl(v); setTyped(""); setUnderstood(false); }} />
          <label className="flex flex-col gap-1 text-small">
            <span>{state === "unlocked" ? `Pump confirmation: type "PUMP ${selected?.name ?? ""}"` : `Type the channel name "${selected?.name ?? ""}" to confirm`}</span>
            <input className="rounded-control border border-border bg-surface-sunken px-3 py-2" value={typed} onChange={(e) => setTyped(e.target.value)} aria-label="Confirmation" />
          </label>
        </div>
        <Checkbox checked={understood} onChange={setUnderstood}>I understand the risks for this channel.</Checkbox>
        <div className="flex flex-wrap gap-2">
          <Button disabled={busy || !selected || !understood || typed.trim() !== selected.name.trim() || state === "unlocked" || state === "unlocked_pump"} onClick={() => run("unlock", () => rpc("cooling.channels.unlock", { control_id: control, confirmation: typed }))}>Unlock channel</Button>
          <Button variant="secondary" disabled={busy || !selected || !understood || state !== "unlocked" || typed.trim() !== `PUMP ${selected.name.trim()}`} onClick={() => run("pump", () => rpc("cooling.channels.confirm_pump", { control_id: control, confirmation: typed }))}>Confirm pump control</Button>
          <Button variant="secondary" disabled={busy || !selected || state === "none"} onClick={() => run("revoke", () => rpc("cooling.channels.revoke", { control_id: control }))}>Revoke</Button>
        </div>
        <p className="text-caption text-text-subtle">Unlocks are bound to this board, its BIOS version and the channel; any change invalidates them. Pump confirmation is only accepted for channels bound as pump.</p>
      </>}
      {(error || support?.error) && <p role="alert" className="text-small text-danger-text">{error || support?.error}</p>}
    </Section>
  );
}
