import { LayoutDashboard, RotateCcw } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { PageHeader } from "@/components/page";
import { RegionCanvas, type CanvasItem } from "@/components/region-canvas";
import { EmptyState, StatusDot } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { effectPreview } from "@/lib/api/effects";
import { RpcError } from "@/lib/api/transport";
import { deviceStatus, type DeviceSummary, type Region } from "@/lib/api/types";
import { FULL, normalize } from "@/lib/region";
import { approxEqual } from "@/lib/store/approxEqual";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { keyedFrameThrottle } from "@/lib/utils";
import { useCurrentEffect } from "../lighting/useEffectDraft";
import { InlineError } from "@/components/status";
import { t, type LayoutKey } from "./messages";

/** Device colours on the canvas: the brand accents, then two extra hues. */
const COLORS = ["#3b82f6", "#10a37f", "#f59e0b", "#a855f7", "#ef4444", "#22d3ee", "#f472b6"];

const FIELDS: { key: keyof Region; label: LayoutKey }[] = [
  { key: "x", label: "layout.left" },
  { key: "y", label: "layout.top" },
  { key: "w", label: "layout.width" },
  { key: "h", label: "layout.height" },
];

function RegionFields({ region, onChange }: { region: Region; onChange: (r: Region, commit: boolean) => void }) {
  return (
    <div className="flex flex-col gap-3">
      {FIELDS.map(({ key, label }) => (
        <div key={key} className="flex flex-col gap-1.5">
          <div className="flex justify-between text-small">
            <span>{t(label)}</span>
            <span className="text-text-muted tabular">{Math.round(region[key] * 100)}%</span>
          </div>
          <Slider
            aria-label={t(label)}
            min={key === "w" || key === "h" ? 0.05 : 0}
            max={1}
            step={0.01}
            value={[region[key]]}
            onValueChange={([v]) => onChange(normalize({ ...region, [key]: v! }), false)}
            onValueCommit={([v]) => onChange(normalize({ ...region, [key]: v! }), true)}
          />
        </div>
      ))}
    </div>
  );
}

export default function LayoutView() {
  const devices = useDaemon((s) => s.devices);
  const effect = useCurrentEffect();
  const [drafts, setDrafts] = useState<Record<string, Region>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const sender = useMemo(
    () =>
      keyedFrameThrottle<Region>((id, region) => {
        rpc("devices.set_region", { id, ...region }).then(
          () => setError(null),
          (e: unknown) => {
            if (e instanceof RpcError && e.detail.kind === "superseded") return;
            setError(e instanceof Error ? e.message : String(e));
            setDrafts(({ [id]: _, ...rest }) => rest);
          },
        );
      }),
    [],
  );

  // Drop drafts the daemon has confirmed (f32 round-trips tolerated).
  useEffect(() => {
    setDrafts((current) => {
      const remaining = Object.fromEntries(
        Object.entries(current).filter(([id, r]) => {
          const remote = devices?.find((d) => d.info.id === id)?.region;
          return !remote || !approxEqual(remote, r);
        }),
      );
      return Object.keys(remaining).length === Object.keys(current).length ? current : remaining;
    });
  }, [devices]);

  if (!devices) return null;
  if (devices.length === 0) {
    return (
      <>
        <PageHeader title={t("layout.title")} />
        <EmptyState icon={<LayoutDashboard />} title={t("layout.empty.title")}>
          {t("layout.empty.text")}
        </EmptyState>
      </>
    );
  }

  const regionOf = (d: DeviceSummary) => drafts[d.info.id] ?? d.region;
  const change = (id: string, region: Region, commit: boolean) => {
    setDrafts((current) => ({ ...current, [id]: region }));
    if (commit) sender.flush(id, region);
    else sender.push(id, region);
  };
  const items: CanvasItem[] = devices.map((d, i) => ({
    id: d.info.id,
    label: d.info.name,
    color: COLORS[i % COLORS.length]!,
    region: regionOf(d),
    muted: deviceStatus(d.status) !== "connected",
  }));
  const current = devices.find((d) => d.info.id === selected) ?? null;

  return (
    <>
      <PageHeader
        title={t("layout.title")}
        description={t("layout.description")}
      />
      <div className="@container">
        <div className="grid items-start gap-6 @3xl:grid-cols-[minmax(0,1fr)_280px]">
          <div className="flex flex-col gap-3">
            <RegionCanvas
              items={items}
              selected={selected}
              background={effect ? effectPreview(effect) : undefined}
              onSelect={setSelected}
              onChange={change}
            />
            <p className="text-caption text-text-subtle">
              {t("layout.help")}
            </p>
            {error && <InlineError>{error}</InlineError>}
          </div>

          <aside aria-label={t("layout.aside")} className="flex flex-col gap-4 rounded-card bg-surface-sunken p-4">
            {current ? (
              <>
                <header className="flex flex-col gap-1">
                  <h2 className="text-title font-semibold">{current.info.name}</h2>
                  <StatusDot
                    tone={deviceStatus(current.status) === "connected" ? "ok" : "idle"}
                    label={t("layout.leds", { n: current.leds, vendor: current.info.vendor })}
                  />
                </header>
                <RegionFields region={regionOf(current)} onChange={(r, commit) => change(current.info.id, r, commit)} />
                <Button variant="ghost" size="sm" onClick={() => change(current.info.id, FULL, true)}>
                  <RotateCcw aria-hidden /> {t("layout.whole")}
                </Button>
              </>
            ) : (
              <ul className="flex flex-col gap-1">
                {items.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => setSelected(item.id)}
                      className="flex h-9 w-full items-center gap-3 rounded-control px-2 text-left text-body hover:bg-surface-raised"
                    >
                      <span className="size-3 rounded-full" style={{ background: item.color }} />
                      <span className="truncate">{item.label}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </aside>
        </div>
      </div>
    </>
  );
}
