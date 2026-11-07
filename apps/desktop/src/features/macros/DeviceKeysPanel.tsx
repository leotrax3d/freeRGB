import { t } from "./messages";
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
    try { await rpc("macros.device_keys.save", { bindings }); await refresh(); setMessage(t("macros.deviceKeysPanel.bindingsSavedActivationRequiresAnExplicitSession")); }
    catch (e) { setMessage(String(e)); }
  };
  const keys = [63, 64, 65, 66, 67].filter((key) => state?.devices.find((d) => d.id === device)?.keys[String(key)] !== undefined);
  return <section aria-label={t("macros.deviceKeysPanel.deviceMacroKeys")} className="mb-4 flex flex-col gap-2 rounded-card border border-border p-4">
    <h3>{t("macros.deviceKeysPanel.deviceMacroKeys")}</h3>
    <p>{t("macros.deviceKeysPanel.assignMappedF6F10KeysTypingKeys")}</p>
    {state?.physical_identity_pending && <p>{t("macros.deviceKeysPanel.physicalSourceIdentityAwaitsOwnerVerificationAnonymous")}</p>}
    <div className="flex flex-wrap gap-2">
      <Select label={t("macros.deviceKeysPanel.device")} value={device} onChange={setDevice} options={[{ value: "", label: t("macros.deviceKeysPanel.chooseDevice") }, ...(state?.devices ?? []).map((d) => ({ value: d.id, label: d.name }))]} />
      <Select label={t("macros.deviceKeysPanel.deviceKey")} value={usage} onChange={setUsage} options={keys.map((key) => ({ value: String(key), label: `F${key - 57}` }))} />
      <Select label={t("macros.deviceKeysPanel.savedMacro")} value={name} onChange={setName} options={[{ value: "", label: t("macros.deviceKeysPanel.chooseMacro") }, ...names.map((n) => ({ value: n, label: n }))]} />
      <Button disabled={!device || !name || !keys.includes(Number(usage))} onClick={() => save([...(state?.bindings ?? []), { device_id: device, usage: Number(usage), macro_name: name }])}>{t("macros.deviceKeysPanel.assign")}</Button>
    </div>
    <ul>{state?.bindings.map((binding, i) => <li key={`${binding.device_id}:${binding.usage}`}>
      {state.devices.find((d) => d.id === binding.device_id)?.name ?? binding.device_id} · F{binding.usage - 57} → {binding.macro_name}
      <Button size="sm" variant="ghost" aria-label={t("macros.deviceKeysPanel.removeDeviceBindingValue", { p0: i + 1 })} onClick={() => save(state.bindings.filter((_, index) => index !== i))}>{t("macros.deviceKeysPanel.remove")}</Button>
    </li>)}</ul>
    <label><input type="checkbox" checked={state?.enabled ?? false} disabled={!state?.bindings.length} onChange={async (e) => {
      try { await rpc("macros.device_keys.enable", { enabled: e.target.checked }); await refresh(); } catch (error) { setMessage(String(error)); }
    }} /> {t("macros.deviceKeysPanel.enableDeviceMacroKeysForThisSession")}</label>
    {message && <p role="status">{message}</p>}
  </section>;
}
