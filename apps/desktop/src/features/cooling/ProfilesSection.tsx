import { useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";

/** Named sets of background channel curves; activating one asks for UAC. */
export function ProfilesSection() {
  useSlices("cooling_profiles", "cooling_background");
  const profiles = useDaemon((s) => s.cooling_profiles);
  const background = useDaemon((s) => s.cooling_background);
  const connected = useDaemon((s) => s.connection?.connected);
  const [selected, setSelected] = useState("");
  const [name, setName] = useState("");
  const { pending, error, run } = useAction();
  const busy = !connected || pending !== null || background?.apply.state === "applying";
  const trimmed = name.trim();
  return (
    <Section title="Cooling profiles">
      <p className="text-small text-text-muted">Save the current background channels as a named profile and switch between profiles (for example Quiet, Gaming, Full). Every channel is checked again against its support tier, channel test and unlock when a profile is activated.</p>
      <div className="grid gap-3 md:grid-cols-2">
        <Select label="Profile" placeholder="Profile" value={selected} options={(profiles?.profiles ?? []).map((p) => ({ value: p.name, label: `${p.name} · ${p.channels} channel${p.channels === 1 ? "" : "s"}${p.name === profiles?.active ? " · active" : ""}` }))} onChange={setSelected} />
        <label className="flex flex-col gap-1 text-small">
          <span>Save current background as</span>
          <input className="rounded-control border border-border bg-surface-sunken px-3 py-2" value={name} maxLength={64} onChange={(e) => setName(e.target.value)} aria-label="Profile name" />
        </label>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button disabled={busy || !selected} onClick={() => run("activate", () => rpc("cooling.profiles.activate", { name: selected }))}>Activate (UAC)</Button>
        <Button variant="secondary" disabled={busy || !trimmed} onClick={() => run("save", async () => { await rpc("cooling.profiles.save", { name: trimmed }); setSelected(trimmed); setName(""); })}>Save profile</Button>
        <Button variant="secondary" disabled={busy || !selected} onClick={() => run("delete", async () => { await rpc("cooling.profiles.delete", { name: selected }); setSelected(""); })}>Delete</Button>
      </div>
      <p className="text-caption text-text-subtle">Active: {profiles?.active ?? "none"}</p>
      {(error || profiles?.error) && <p role="alert" className="text-small text-danger-text">{error || profiles?.error}</p>}
    </Section>
  );
}
