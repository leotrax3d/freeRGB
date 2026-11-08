import { Check, Download, RefreshCw, Search, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { PageHeader } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Select } from "@/components/ui/select";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { cn } from "@/lib/utils";
import { useNavigation } from "@/app/navigation";
import { InstallProgress } from "./InstallProgress";
import { t } from "./messages";
import { ReportIssue } from "./ReportIssue";
import { Stars } from "./Stars";
import { defaults, matches, sortPackages, type SortOrder, type MarketplaceStatus, type Package } from "./types";

function EffectCard({ pkg, installed, status, disabled, run }: {
  pkg: Package; installed?: Package; status: MarketplaceStatus; disabled: boolean;
  run: (method: string, params?: unknown) => Promise<boolean>;
}) {
  const active = status.active.type === "marketplace" && status.active.id === pkg.id;
  const current = active && status.active.type === "marketplace" ? status.active : null;
  const target = installed ?? pkg;
  const packageDisabled = status.packages.disabled?.includes(pkg.id) ?? false;
  const [installStarted, setInstallStarted] = useState<number | null>(null);
  const [installAccepted, setInstallAccepted] = useState(false);
  useEffect(() => {
    if (installAccepted && installStarted && status.packages.failure?.effect_id === pkg.id && !status.packages.job) setInstallStarted(null);
  }, [installAccepted, installStarted, pkg.id, status.packages.failure, status.packages.job]);
  const update = installed && installed.version !== pkg.version;
  const ready = installed?.version === pkg.version;
  const job = status.packages.job?.id === pkg.id ? status.packages.job : null;
  const progress = job && job.total ? Math.min(100, Math.round(job.bytes / job.total * 100)) : 0;
  return (
    <article className={cn("flex flex-col overflow-hidden rounded-card border bg-surface-sunken", active ? "border-accent/60" : "border-border")}>
      <div className="relative h-28" style={{ background: `linear-gradient(125deg, ${pkg.preview[0]}, ${pkg.preview[1]})` }} aria-hidden>
        <div className="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent" />
        <div className="absolute bottom-3 start-4 end-4 flex justify-between text-caption text-white">
          <span>{pkg.category}</span><span>{pkg.version}</span>
        </div>
      </div>
      <div className="flex flex-1 flex-col gap-4 p-4">
        <div>
          <div className="flex items-center justify-between gap-2"><h2 className="text-title font-semibold">{pkg.name}</h2>{active && <span className="flex items-center gap-1 text-caption text-accent"><Check className="size-3" aria-hidden />{t("market.active")}{current?.version !== pkg.version ? ` · ${current?.version}` : ""}</span>}</div>
          <p className="mt-1 text-small text-text-muted">{pkg.description}</p>
          <p className="mt-2 text-caption text-text-subtle">{t("market.by")} <span className="font-medium text-text-muted">{pkg.publisher.name}</span> · @{pkg.publisher.id}</p>
        </div>
        <Stars id={pkg.id} name={pkg.name} ratings={status.packages.ratings} disabled={disabled} rate={(stars) => void run("marketplace.rate", { id: pkg.id, stars })} />
        <p className="flex items-center gap-2 text-caption text-text-subtle"><Download className="size-3" aria-hidden />{status.packages.ratings.updated_at ? t("market.downloads", { n: (status.packages.ratings.by_effect[pkg.id]?.downloads ?? 0).toLocaleString() }) : t("market.downloadsNone")}</p>
        {installStarted !== null && <InstallProgress name={pkg.name} started={installStarted} reported={progress} completed={ready && !job} onFinish={() => setInstallStarted(null)} />}
        {job && installStarted === null && <progress aria-label={t("market.installation", { name: pkg.name })} max={100} value={Math.min(95,progress)} className="h-1.5 w-full accent-accent" />}
        {job && job.stage !== "committing" && <Button variant="ghost" onClick={() => void run("marketplace.cancel")}>{t("market.cancel")}</Button>}
        {update && <p className="text-caption text-accent">{t("market.updateAvailable", { from: installed.version, to: pkg.version })}</p>}
        <div className="mt-auto flex flex-wrap items-center gap-2 pt-1">
          {!ready && <Button disabled={disabled || installStarted !== null} onClick={async () => {setInstallAccepted(false);setInstallStarted(Date.now()); if (await run("marketplace.install", { id: pkg.id })) setInstallAccepted(true); else setInstallStarted(null);}}><Download className="size-4" aria-hidden />{update ? t("market.update") : t("market.install")}</Button>}
          {installed && <Button disabled={disabled || packageDisabled} onClick={async () => {if(await run("effects.set", { type: "marketplace", id: pkg.id, version: target.version, parameters: current?.version === target.version ? current.parameters : defaults(target) })) useNavigation.getState().go("lighting");}}>{t("market.open")}</Button>}
          {installed && <Button variant="secondary" disabled={disabled} aria-label={t(packageDisabled ? "market.enableAria" : "market.disableAria", { name: pkg.name })} onClick={() => void run(packageDisabled ? "marketplace.enable" : "marketplace.disable", { id: pkg.id })}>{packageDisabled ? t("market.enable") : t("market.disable")}</Button>}
          {installed && <Button variant="ghost" aria-label={t("market.uninstall", { name: pkg.name })} title={t("market.uninstallHint")} disabled={disabled} onClick={() => void run("marketplace.uninstall", { id: pkg.id })}><Trash2 className="size-4" aria-hidden /></Button>}
        </div>
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
      return true;
    } catch (e) { if (alive.current) setError(e instanceof Error ? e.message : String(e)); return false; }
    finally { if (alive.current) setPending(false); }
  };
  const disabled = pending || !!status?.packages.job || mode === "mock";
  const installed = new Map(status?.packages.installed.map((p) => [p.id, p]));
  const packages = [...(status?.packages.catalog ?? []), ...(status?.packages.installed.filter((p) => !status.packages.catalog.some((c) => c.id === p.id)) ?? [])];
  const visible = sortPackages(packages.filter((pkg) => matches(pkg, query, installed.has(pkg.id), sortOrder === "installed") && (sortOrder !== "updates" || (installed.has(pkg.id) && installed.get(pkg.id)?.version !== pkg.version))), sortOrder, status?.packages.ratings.by_effect ?? {});
  return <>
    <PageHeader title={t("market.title")} />
    <div className="flex flex-col gap-5">
      <div className="flex justify-end">
        <Button variant="secondary" disabled={disabled || !status} onClick={() => void run("marketplace.refresh")}><RefreshCw className={cn("size-4", status?.packages.job?.action === "refresh" && "animate-spin")} aria-hidden />{t("market.check")}</Button>
      </div>
      {connectionError && <p role="alert" className="text-small text-danger-text">{connectionError}</p>}
      {error && <p role="alert" className="rounded-control border border-danger-text/30 p-3 text-small text-danger-text">{error}</p>}
      {status?.packages.error && <p role="alert" className="rounded-control border border-danger-text/30 p-3 text-small text-danger-text">{status.packages.failure?.code && <strong>{status.packages.failure.code}: </strong>}{status.packages.error} {t("market.err.package")}
        {status.packages.failure && status.packages.failure.code !== "FX_INSTALL_CANCELLED" && <Button variant="secondary" className="mt-2" onClick={() => setReportKind("operation")}>{t("market.err.review")}</Button>}</p>}
      {status?.runtime_error && <p role="alert" className="rounded-control border border-danger-text/30 p-3 text-small text-danger-text">{status.runtime_error_code && <strong>{status.runtime_error_code}: </strong>}{status.runtime_error} {t("market.err.runtime")}
        {status.runtime_error_code && <Button variant="secondary" className="mt-2" onClick={() => setReportKind("runtime")}>{t("market.err.review")}</Button>}</p>}
      {status?.packages.ratings.error && <p role="alert" className="text-small text-danger-text">{status.packages.ratings.error} {t("market.err.ratings")}</p>}
      {status && <section aria-label={t("market.privacy.title")} className="flex flex-col gap-2 rounded-control border border-border p-3">
        <h2 className="text-small font-medium">{t("market.privacy.title")}</h2>
        <p className="text-caption text-text-subtle">{t("market.privacy.body")}</p>
        <Checkbox checked={status.packages.community.online} disabled={disabled} onChange={(online) => void run("marketplace.community.set", { online })}>{t("market.privacy.online")}</Checkbox>
        <Checkbox checked={status.packages.community.count_downloads} disabled={disabled || !status.packages.community.online} onChange={(count_downloads) => void run("marketplace.community.set", { count_downloads })}>{t("market.privacy.count")}</Checkbox>
      </section>}
      <div className="flex flex-wrap items-center gap-4"><label className="relative min-w-48 flex-1"><Search className="absolute start-3 top-2.5 size-4 text-text-subtle" aria-hidden /><input aria-label={t("market.search")} placeholder={t("market.searchPlaceholder")} value={query} onChange={(e) => setQuery(e.target.value)} className="h-9 w-full rounded-control border border-border bg-surface-sunken ps-9 pe-3 text-small" /></label><span className="text-caption text-text-subtle">{t("market.installedCount", { n: installed.size })}</span></div>
      <div className="flex items-center gap-3 text-small"><span>{t("market.sortBy")}</span><Select<SortOrder> label={t("market.sortAria")} value={sortOrder} onChange={setSortOrder} options={[{value:"downloads",label:t("market.sort.downloads")},{value:"rating",label:t("market.sort.rating")},{value:"name",label:t("market.sort.name")},{value:"installed",label:t("market.sort.installed")},{value:"updates",label:t("market.sort.updates")}]} />{status?.packages.ratings.configured && <Button variant="ghost" aria-label={t("market.refreshRatings")} disabled={status.packages.ratings.loading || !!status.packages.ratings.saving} onClick={() => void run("marketplace.ratings.refresh")}><RefreshCw className={cn("size-4",status.packages.ratings.loading && "animate-spin")} /></Button>}</div>
      {!status && mode !== "mock" && !error && <p role="status" className="text-small text-text-muted">{t("market.connecting")}</p>}
      {status && visible.length === 0 && <p className="py-10 text-center text-small text-text-muted">{t("market.noMatch")}</p>}
      <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,280px),1fr))] items-stretch gap-4">{status && visible.map((pkg) => <EffectCard key={pkg.id} pkg={pkg} installed={installed.get(pkg.id)} status={status} disabled={disabled} run={run} />)}</div>
    </div>
    {reportKind && <ReportIssue kind={reportKind} onClose={() => setReportKind(null)} />}
  </>;
}
