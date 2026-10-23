import { useEffect, useState } from "react";
import { Section } from "@/components/page";
import { rpc } from "@/lib/store/daemon";
import { defaults, type MarketplaceStatus, type Package } from "@/features/marketplace/types";
import { useCurrentEffect } from "./useEffectDraft";
import { cn } from "@/lib/utils";

export function useInstalledEffects() {
  const [packages,setPackages]=useState<Package[]>([]);
  useEffect(() => {
    let stopped=false;
    const refresh=()=>rpc<MarketplaceStatus>("marketplace.status").then(status=>{if(!stopped)setPackages(status.packages.installed);}).catch(()=>undefined);
    void refresh();
    const timer=setInterval(()=>void refresh(),2000);
    return ()=>{stopped=true;clearInterval(timer);};
  },[]);
  return packages;
}

export function InstalledEffects({packages}:{packages:Package[]}) {
  const current=useCurrentEffect();
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState<string|null>(null);
  if(!packages.length)return null;
  return <Section title="Installed effects">
    <div className="grid grid-cols-[repeat(auto-fill,minmax(128px,1fr))] gap-2">{packages.map(pkg=>{
      const active=current?.type==="marketplace" && current.id===pkg.id && current.version===pkg.version;
      return <button key={pkg.id} aria-pressed={active} disabled={busy} className={cn("flex flex-col gap-2 rounded-card p-1.5 text-left",active?"bg-surface-raised ring-1 ring-accent/60":"hover:bg-surface-raised")} onClick={async()=>{
        if(active)return;
        setBusy(true);setError(null);
        try {await rpc("effects.set",{type:"marketplace",id:pkg.id,version:pkg.version,parameters:defaults(pkg)});} catch(e){setError(e instanceof Error?e.message:String(e));} finally{setBusy(false);}
      }}><div className="h-14 w-full rounded-control" style={{background:`linear-gradient(125deg,${pkg.preview[0]},${pkg.preview[1]})`}} aria-hidden /><span className="px-1 text-small font-medium">{pkg.name}</span></button>;
    })}</div>
    {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
  </Section>;
}
