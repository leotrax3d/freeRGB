import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { rpc } from "@/lib/store/daemon";

interface Binding { device_id: string; usage: number; macro_name: string }
interface State { bindings: Binding[]; enabled: boolean; devices: { id: string; name: string; keys: Record<string, number> }[]; physical_identity_pending: boolean }

export function DeviceKeysPanel({ names }: { names: string[] }) {
  const [state, setState] = useState<State | null>(null);
  const [device, setDevice] = useState("");
  const [name, setName] = useState("");
  const [usage, setUsage] = useState("63");
  const [message, setMessage] = useState("");
  const refresh = () => rpc<State>("macros.device_keys.get").then(setState).catch((e: unknown) => setMessage(String(e)));
  useEffect(() => { void refresh(); return () => { void rpc("macros.device_keys.enable", { enabled: false }).catch(() => {}); }; }, []);
  const save = async (bindings: Binding[]) => {
    try { await rpc("macros.device_keys.save", { bindings }); await refresh(); setMessage("Bindings saved; activation requires an explicit session opt-in."); }
    catch (e) { setMessage(String(e)); }
  };
  const keys = [63, 64, 65, 66, 67].filter((key) => state?.devices.find((d) => d.id === device)?.keys[String(key)] !== undefined);
  return <section aria-label="Device macro keys" className="mb-4 flex flex-col gap-2 rounded-card border border-border p-4">
    <h3>Device macro keys</h3>
    <p>Assign mapped F6–F10 keys. Typing keys, modifiers and cancellation F11 are reserved.</p>
    {state?.physical_identity_pending && <p>Physical source identity awaits owner verification. Anonymous keyboard events never trigger these bindings.</p>}
    <div className="flex flex-wrap gap-2">
      <Select label="Device" value={device} onChange={setDevice} options={[{ value: "", label: "Choose device" }, ...(state?.devices ?? []).map((d) => ({ value: d.id, label: d.name }))]} />
      <Select label="Device key" value={usage} onChange={setUsage} options={keys.map((key) => ({ value: String(key), label: `F${key - 57}` }))} />
      <Select label="Saved macro" value={name} onChange={setName} options={[{ value: "", label: "Choose macro" }, ...names.map((n) => ({ value: n, label: n }))]} />
      <Button disabled={!device || !name || !keys.includes(Number(usage))} onClick={() => save([...(state?.bindings ?? []), { device_id: device, usage: Number(usage), macro_name: name }])}>Assign</Button>
    </div>
    <ul>{state?.bindings.map((binding, i) => <li key={`${binding.device_id}:${binding.usage}`}>
      {state.devices.find((d) => d.id === binding.device_id)?.name ?? binding.device_id} · F{binding.usage - 57} → {binding.macro_name}
      <Button size="sm" variant="ghost" aria-label={`Remove device binding ${i + 1}`} onClick={() => save(state.bindings.filter((_, index) => index !== i))}>Remove</Button>
    </li>)}</ul>
    <label><input type="checkbox" checked={state?.enabled ?? false} disabled={!state?.bindings.length} onChange={async (e) => {
      try { await rpc("macros.device_keys.enable", { enabled: e.target.checked }); await refresh(); } catch (error) { setMessage(String(error)); }
    }} /> Enable device macro keys for this session</label>
    {message && <p role="status">{message}</p>}
  </section>;
}
