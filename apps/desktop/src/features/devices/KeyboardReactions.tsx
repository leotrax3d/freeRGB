import { useState } from "react";
import { Section } from "@/components/page";
import { InlineError } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import type { ReactiveConfig } from "@/lib/api/types";
import { t } from "@/features/settings/messages";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";

export function KeyboardReactions() {
  useSlices("keyboard_reactive");
  const state = useDaemon((s) => s.keyboard_reactive);
  const connected = useDaemon((s) => s.connection?.connected);
  const [draft, setDraft] = useState<ReactiveConfig | null>(null);
  const { pending, error, run } = useAction();
  const config = draft ?? state?.config;
  const edit = (patch: Partial<ReactiveConfig>) => config && setDraft({ ...config, ...patch });
  return (
    <Section title={t("settings.kbd.title")}>
      {!state || !config ? (
        <p className="text-small text-text-subtle">{t("settings.kbd.unavailable")}</p>
      ) : (
        <>
          <p className="text-small text-text-muted">{t("settings.kbd.intro")}</p>
          <Checkbox checked={config.enabled} onChange={(enabled) => edit({ enabled })}>{t("settings.kbd.enable")}</Checkbox>
          <div className="flex flex-wrap items-center gap-4 text-small">
            <label className="flex items-center gap-2">
              {t("settings.kbd.color")}{" "}
              <input
                type="color"
                aria-label={t("settings.kbd.colorAria")}
                value={`#${[config.color.r, config.color.g, config.color.b].map((v) => v.toString(16).padStart(2, "0")).join("")}`}
                onChange={(e) => {
                  const hex = e.target.value;
                  edit({ color: { r: parseInt(hex.slice(1, 3), 16), g: parseInt(hex.slice(3, 5), 16), b: parseInt(hex.slice(5, 7), 16) } });
                }}
              />
            </label>
            <label>
              {t("settings.kbd.fade")}{" "}
              <input
                className="w-24 rounded-control bg-surface-sunken p-2"
                type="number"
                min={50}
                max={5000}
                value={config.fade_ms}
                onChange={(e) => edit({ fade_ms: e.target.valueAsNumber })}
              />
            </label>
            <Checkbox checked={config.iso} onChange={(iso) => edit({ iso })}>{t("settings.kbd.iso")}</Checkbox>
          </div>
          {state.devices.length === 0 && <p className="text-small text-text-subtle">{t("settings.kbd.none")}</p>}
          {state.devices.map((device) => (
            <Checkbox
              key={device.id}
              disabled={!config.devices.includes(device.id) && (!device.connected || Object.keys(device.keys).length === 0)}
              checked={config.devices.includes(device.id)}
              onChange={(selected) =>
                edit({ devices: selected ? [...config.devices.filter((id) => id !== device.id), device.id] : config.devices.filter((id) => id !== device.id) })
              }
            >
              {t("settings.kbd.device", { name: device.name, n: Object.keys(device.keys).length })}
              {device.connected ? "" : t("settings.kbd.disconnected")}
            </Checkbox>
          ))}
          <p className="text-small text-text-muted">
            {t("settings.kbd.capture", { state: state.status.state })}
            {state.status.error ? ` · ${state.status.error}` : ""}
          </p>
          <div className="flex gap-2">
            <Button
              disabled={!connected || pending !== null || !Number.isInteger(config.fade_ms) || config.fade_ms < 50 || config.fade_ms > 5000}
              onClick={() =>
                run(
                  "save",
                  async () => {
                    await rpc("keyboard.reactive.configure", { config });
                    setDraft(null);
                  },
                  t("settings.kbd.saved"),
                )
              }
            >
              {t("settings.kbd.save")}
            </Button>
            <Button
              variant="secondary"
              disabled={!connected || pending !== null}
              onClick={() =>
                run("disable", async () => {
                  await rpc("keyboard.reactive.disable");
                  setDraft(null);
                })
              }
            >
              {t("settings.kbd.disable")}
            </Button>
            <Button variant="ghost" disabled={!draft} onClick={() => setDraft(null)}>{t("settings.kbd.reset")}</Button>
          </div>
        </>
      )}
      {error && <InlineError>{error}</InlineError>}
    </Section>
  );
}
