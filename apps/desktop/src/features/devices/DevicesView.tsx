import { Box, Cable, Cpu, Fan, Headphones, Keyboard, MemoryStick, Mouse, PlugZap, Snowflake, Sun, type LucideIcon } from "lucide-react";
import { memo } from "react";
import { PageHeader } from "@/components/page";
import { EmptyState, InlineError, StatusDot, type Tone } from "@/components/status";
import { t, type DevicesKey } from "./messages";
import { Slider } from "@/components/ui/slider";
import { deviceStatus, faultMessage, type DeviceKind, type DeviceSummary } from "@/lib/api/types";
import { useDaemon } from "@/lib/store/daemon";
import { num } from "@/lib/format";
import { useDaemonValue } from "@/lib/store/useDaemonValue";
import { NewDevicesPanel, SetupExtras } from "@/features/setup/NewDevicesPanel";
import { CoolingPluginsManager } from "@/features/cooling/PluginsSection";

const KINDS: Record<DeviceKind, { label: DevicesKey; icon: LucideIcon }> = {
  Keyboard: { label: "devices.kindOne.Keyboard", icon: Keyboard },
  Mouse: { label: "devices.kindOne.Mouse", icon: Mouse },
  Headset: { label: "devices.kindOne.Headset", icon: Headphones },
  LightingController: { label: "devices.kindOne.LightingController", icon: Cable },
  FanHub: { label: "devices.kindOne.FanHub", icon: Fan },
  Cooler: { label: "devices.kindOne.Cooler", icon: Snowflake },
  Memory: { label: "devices.kindOne.Memory", icon: MemoryStick },
  Virtual: { label: "devices.kindOne.Virtual", icon: Box },
  Other: { label: "devices.kindOne.Other", icon: Cpu },
};

const STATUS: Record<ReturnType<typeof deviceStatus>, { tone: Tone; label: DevicesKey }> = {
  connected: { tone: "ok", label: "devices.status.connected" },
  disconnected: { tone: "idle", label: "devices.status.disconnected" },
  faulted: { tone: "error", label: "devices.status.faulted" },
};

function DeviceBrightness({ device }: { device: DeviceSummary }) {
  const id = device.info.id;
  const { value, change, commit, error } = useDaemonValue(device.brightness, (v) => [
    "devices.set_brightness",
    { id, value: v },
  ]);
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-3">
        <Sun className="size-3.5 text-text-subtle" aria-hidden />
        <Slider
          aria-label={t("devices.brightness", { name: device.info.name })}
          min={0}
          max={1}
          step={0.01}
          value={[value]}
          onValueChange={([v]) => change(v!)}
          onValueCommit={([v]) => commit(v!)}
        />
        <span className="w-10 text-end text-caption text-text-muted tabular">{Math.round(value * 100)}%</span>
      </div>
      {error && <InlineError>{error}</InlineError>}
    </div>
  );
}

// Memoised: a device card re-renders only when its own summary changed.
const DeviceCard = memo(function DeviceCard({ device }: { device: DeviceSummary }) {
  const kind = KINDS[device.info.kind] ?? KINDS.Other;
  const Icon = kind.icon;
  const state = deviceStatus(device.status);
  const fault = faultMessage(device.status) ?? device.last_error;
  return (
    <article className="flex flex-col gap-4 rounded-card border border-border bg-surface-sunken p-4">
      <header className="flex items-start gap-3">
        <div className="grid size-10 shrink-0 place-items-center rounded-control bg-surface-raised text-text-muted">
          <Icon className="size-5" aria-hidden />
        </div>
        <div className="flex min-w-0 flex-1 flex-col">
          <h3 className="truncate text-body font-medium" title={device.info.name}>
            {device.info.name}
          </h3>
          <span className="text-caption text-text-subtle">{device.info.vendor} · {t(kind.label)}</span>
        </div>
        <StatusDot tone={STATUS[state].tone} label={t(STATUS[state].label)} />
      </header>
      <dl className="grid grid-cols-3 gap-2 text-caption">
        {[
          [t("devices.leds"), num(device.leds)],
          [t("devices.frames"), num(device.frames)],
          [t("devices.dropped"), num(device.dropped)],
        ].map(([label, value]) => (
          <div key={label} className="flex flex-col gap-0.5 rounded-chip bg-surface px-2.5 py-1.5">
            <dt className="text-text-subtle">{label}</dt>
            <dd className="text-small text-text tabular">{value}</dd>
          </div>
        ))}
      </dl>
      {device.sensors.length > 0 && (
        <ul className="flex flex-col gap-1 text-small">
          {device.sensors.map((s) => (
            <li key={s.id} className="flex justify-between gap-3">
              <span className="truncate text-text-muted">{s.name}</span>
              <span className="text-text tabular">
                {s.value === null ? "—" : `${num(s.value, 1)} ${s.unit}`}
              </span>
            </li>
          ))}
        </ul>
      )}
      {fault && <InlineError className="rounded-chip bg-danger/10 px-2.5 py-1.5">{fault}</InlineError>}
      <DeviceBrightness device={device} />
    </article>
  );
});

export default function DevicesView() {
  const devices = useDaemon((s) => s.devices);
  const groups = new Map<DeviceKind, DeviceSummary[]>();
  for (const d of devices ?? []) groups.set(d.info.kind, [...(groups.get(d.info.kind) ?? []), d]);

  return (
    <>
      <PageHeader
        title={t("devices.title")}
        description={devices ? t("devices.summary", { connected: devices.filter((d) => d.status === "Connected").length, total: devices.length }) : t("devices.summaryNone")}
      />
      <NewDevicesPanel />
      {devices && devices.length === 0 && (
        <EmptyState icon={<PlugZap />} title={t("devices.empty.title")}>
          {t("devices.empty.text")}
        </EmptyState>
      )}
      {devices && devices.length > 0 && (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
          {[...groups.values()].flat().map((d) => (
            <DeviceCard key={d.info.id} device={d} />
          ))}
        </div>
      )}
      <SetupExtras />
      <CoolingPluginsManager />
    </>
  );
}
