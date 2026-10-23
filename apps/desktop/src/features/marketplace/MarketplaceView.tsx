import { Check, Download, RefreshCw, Search, Store, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { PageHeader } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { cn } from "@/lib/utils";
import { Contact } from "@/components/Contact";
import { ReportIssue } from "./ReportIssue";
import { Stars } from "./Stars";
import { defaults, matches, sortPackages, type SortOrder, type MarketplaceStatus, type Package } from "./types";

function EffectCard({ pkg, installed, status, disabled, run }: {
  pkg: Package; installed?: Package; status: MarketplaceStatus; disabled: boolean;
  run: (method: string, params?: unknown) => Promise<void>;
}) {
  const active = status.active.type === "marketplace" && status.active.id === pkg.id;
  const current = active && status.active.type === "marketplace" ? status.active : null;
  const target = installed ?? pkg;
  const [params, setParams] = useState<Record<string, number>>(defaults(target));
  const previous = useRef("");
  const selected = current?.version === target.version ? current.parameters : null;
  const key = JSON.stringify([target.version, selected]);
  useEffect(() => {
    if (previous.current !== key) { setParams({ ...defaults(target), ...selected }); previous.current = key; }
  }, [key, target, selected]);
  const update = installed && installed.version !== pkg.version;
  const ready = installed?.version === pkg.version;
  const job = status.packages.job?.id === pkg.id ? status.packages.job : null;
  const progress = job && job.total ? Math.min(100, Math.round(job.bytes / job.total * 100)) : 0;
  return (
    <article className={cn("flex flex-col overflow-hidden rounded-card border bg-surface-sunken", active ? "border-accent/60" : "border-border")}>
      <div className="relative h-28" style={{ background: `linear-gradient(125deg, ${pkg.preview[0]}, ${pkg.preview[1]})` }} aria-hidden>
        <div className="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent" />
        <div className="absolute bottom-3 left-4 right-4 flex justify-between text-caption text-white">
          <span>{pkg.category}</span><span>{pkg.version}</span>
        </div>
      </div>
      <div className="flex flex-1 flex-col gap-4 p-4">
        <div>
          <div className="flex items-center justify-between gap-2"><h2 className="text-title font-semibold">{pkg.name}</h2>{active && <span className="flex items-center gap-1 text-caption text-accent"><Check className="size-3" />Active{current?.version !== pkg.version ? ` · ${current?.version}` : ""}</span>}</div>
          <p className="mt-1 text-small text-text-muted">{pkg.description}</p>
          <p className="mt-2 text-caption text-text-subtle">By <span className="font-medium text-text-muted">{pkg.publisher.name}</span> · @{pkg.publisher.id}</p>
          <p className="mt-1 text-caption text-text-subtle">From the official catalog · No capture permissions</p>
        </div>
        <Stars id={pkg.id} name={pkg.name} ratings={status.packages.ratings} disabled={disabled} rate={(stars) => void run("marketplace.rate", { id: pkg.id, stars })} />
        <p className="flex items-center gap-2 text-caption text-text-subtle"><Download className="size-3" aria-hidden />{status.packages.ratings.updated_at ? `${(status.packages.ratings.by_effect[pkg.id]?.downloads ?? 0).toLocaleString()} downloads` : "Download count not loaded"}</p>
        {installed && <fieldset className="flex flex-col gap-3" disabled={disabled}>
          <legend className="sr-only">{pkg.name} parameters</legend>
          {target.parameters.map((p) => <label key={p.key} className="flex flex-col gap-1 text-small">
            <span className="flex justify-between gap-3">{p.label}<output className="tabular text-text-muted">{(params[p.key] ?? p.default).toFixed(2)}</output></span>
            <input aria-label={`${pkg.name} ${p.label}`} type="range" min={p.min} max={p.max} step={p.step} value={params[p.key] ?? p.default} onChange={(e) => setParams((old) => ({ ...old, [p.key]: Number(e.target.value) }))} className="w-full accent-accent" />
          </label>)}
        </fieldset>}
        {job && <div role="status" className="flex flex-col gap-1 text-caption text-text-muted">
          <span>{job.stage === "validating" ? "Verifying and testing effect…" : `Downloading… ${progress}%`}</span>
          <progress aria-label={`${pkg.name} installation`} max={100} value={progress} className="h-1.5 w-full accent-accent" />
        </div>}
        <div className="mt-auto flex flex-wrap items-center gap-2 pt-1">
          {!ready && <Button disabled={disabled} onClick={() => void run("marketplace.install", { id: pkg.id })}><Download className="size-4" />{update ? `Update from ${installed.version}` : "Install effect"}</Button>}
          {installed && <Button disabled={disabled} onClick={() => void run("effects.set", { type: "marketplace", id: pkg.id, version: target.version, parameters: params })}>{active && current?.version === target.version ? "Apply settings" : active ? "Activate update" : "Activate"}</Button>}
          {installed && <Button variant="ghost" aria-label={`Uninstall ${pkg.name}`} title={active ? "Switch to another effect before uninstalling" : "Uninstall effect"} disabled={disabled || active} onClick={() => void run("marketplace.uninstall", { id: pkg.id })}><Trash2 className="size-4" /></Button>}
        </div>
        {update && <p className="text-caption text-text-subtle">Your running version stays active until you activate the update.</p>}
      </div>
    </article>
  );
}

export default function MarketplaceView() {
  const mode = useDaemon((s) => s.mode);
  const [reportKind, setReportKind] = useState<"runtime" | "operation" | null>(null);
  const [status, setStatus] = useState<MarketplaceStatus | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [query, setQuery] = useState("");
  const [sortOrder, setSortOrder] = useState<SortOrder>("downloads");
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    if (mode === null) return;
    if (mode === "tauri") void rpc("marketplace.ratings.refresh").catch(() => undefined);
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const value = await rpc<MarketplaceStatus>("marketplace.status");
        if (!stopped) { setStatus(value); setConnectionError(null); }
      } catch (e) { if (!stopped) setConnectionError(e instanceof Error ? e.message : String(e)); }
      if (!stopped) timer = setTimeout(poll, 750);
    };
    void poll();
    return () => { stopped = true; alive.current = false; clearTimeout(timer); };
  }, [mode]);
  const run = async (method: string, params?: unknown) => {
    setPending(true); setError(null);
    try {
      await rpc(method, params);
      const value = await rpc<MarketplaceStatus>("marketplace.status");
      if (alive.current) setStatus(value);
    } catch (e) { if (alive.current) setError(e instanceof Error ? e.message : String(e)); }
    finally { if (alive.current) setPending(false); }
  };
  const disabled = pending || !!status?.packages.job || mode === "mock";
  const installed = new Map(status?.packages.installed.map((p) => [p.id, p]));
  const packages = [...(status?.packages.catalog ?? []), ...(status?.packages.installed.filter((p) => !status.packages.catalog.some((c) => c.id === p.id)) ?? [])];
  const visible = sortPackages(packages.filter((pkg) => matches(pkg, query, installed.has(pkg.id), false)), sortOrder, status?.packages.ratings.by_effect ?? {});
  return <>
    <PageHeader title="Effect Marketplace" />
    <div className="flex flex-col gap-5">
      <div className="flex flex-wrap items-start justify-between gap-4 rounded-card border border-border bg-surface-sunken p-5">
        <div className="flex gap-3"><Store className="mt-1 size-5 text-accent" /><div><h2 className="text-title font-semibold">A new look for every setup</h2><p className="mt-1 max-w-xl text-small text-text-muted">Discover official effects. Run them across your devices, even with this window closed.</p><p className="mt-2 text-caption text-text-subtle">Preview colours are illustrative. Lighting is rendered by the background service.</p></div></div>
        <Button variant="secondary" disabled={disabled || !status} onClick={() => void run("marketplace.refresh")}><RefreshCw className={cn("size-4", status?.packages.job?.action === "refresh" && "animate-spin")} />Check for updates</Button>
      </div>
      {mode === "mock" && <p role="status" className="rounded-card border border-border p-4 text-small text-text-muted">Installation and execution require the desktop app and background service. Browser demo mode cannot manage effects.</p>}
      {connectionError && <p role="alert" className="text-small text-danger-text">{connectionError}</p>}
      {error && <p role="alert" className="rounded-control border border-danger-text/30 p-3 text-small text-danger-text">{error}</p>}
      {status?.packages.error && <p role="alert" className="rounded-control border border-danger-text/30 p-3 text-small text-danger-text">{status.packages.failure?.code && <strong>{status.packages.failure.code}: </strong>}{status.packages.error} Your installed effects remain available. Check the connection or retry the operation.
        {status.packages.failure && <Button variant="secondary" className="mt-2" onClick={() => setReportKind("operation")}>Review error report</Button>}</p>}
      {status?.runtime_error && <p role="alert" className="rounded-control border border-danger-text/30 p-3 text-small text-danger-text">{status.runtime_error_code && <strong>{status.runtime_error_code}: </strong>}{status.runtime_error} Select another effect in Lighting, or reapply settings to retry.
        {status.runtime_error_code && <Button variant="secondary" className="mt-2" onClick={() => setReportKind("runtime")}>Review error report</Button>}</p>}
      {status && !status.packages.ratings.configured && <p className="text-small text-text-muted">Shared star ratings will be available when the freeRGB team connects the rating service. No registration or login will be needed.</p>}
      {status?.packages.ratings.error && <p role="alert" className="text-small text-danger-text">{status.packages.ratings.error} Previously loaded scores may be out of date.</p>}
      {status?.packages.ratings.configured && <div className="flex items-center gap-3 text-caption text-text-subtle"><Button variant="ghost" disabled={status.packages.ratings.loading || !!status.packages.ratings.saving} onClick={() => void run("marketplace.ratings.refresh")}>Refresh ratings</Button>{status.packages.ratings.loading ? "Loading community ratings…" : status.packages.ratings.updated_at ? `Scores updated ${new Date(status.packages.ratings.updated_at * 1000).toLocaleTimeString()}` : "Community scores"}</div>}
      <div className="flex flex-wrap items-center gap-4"><label className="relative min-w-48 flex-1"><Search className="absolute left-3 top-2.5 size-4 text-text-subtle" aria-hidden /><input aria-label="Search effects" placeholder="Search effects…" value={query} onChange={(e) => setQuery(e.target.value)} className="h-9 w-full rounded-control border border-border bg-surface-sunken pl-9 pr-3 text-small" /></label><span className="text-caption text-text-subtle">{installed.size} installed</span></div>
      <div className="flex items-center gap-3 text-small"><span>Sort by</span><Select<SortOrder> label="Sort effects" value={sortOrder} onChange={setSortOrder} options={[{value:"downloads",label:"Most downloaded"},{value:"rating",label:"Highest rated"},{value:"name",label:"Name A–Z"}]} /></div>
      {!status && mode !== "mock" && !error && <p role="status" className="text-small text-text-muted">Connecting to the effect service…</p>}
      {status && visible.length === 0 && <p className="py-10 text-center text-small text-text-muted">No effects match your search.</p>}
      <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,280px),1fr))] items-stretch gap-4">{status && visible.map((pkg) => <EffectCard key={pkg.id} pkg={pkg} installed={installed.get(pkg.id)} status={status} disabled={disabled} run={run} />)}</div>
      <details className="rounded-card border border-border p-4"><summary className="cursor-pointer text-small font-medium">Contact · freeRGB Team</summary><div className="pt-3"><Contact /></div></details>
    </div>
    {reportKind && <ReportIssue kind={reportKind} onClose={() => setReportKind(null)} />}
  </>;
}
