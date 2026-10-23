import { useEffect, useState } from "react";

export function installationProgress(elapsed: number, reported: number, completed: boolean): number {
  const visual = Math.min(90, elapsed / 12);
  return completed ? Math.min(100, elapsed / 9) : Math.min(95, Math.max(visual, reported));
}

export function InstallProgress({ name, started, reported, completed, onFinish }: {
  name: string; started: number; reported: number; completed: boolean; onFinish: () => void;
}) {
  const [now, setNow] = useState(Date.now());
  const progress = installationProgress(now - started, reported, completed);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 40);
    return () => clearInterval(timer);
  }, []);
  useEffect(() => {
    if (progress < 100) return;
    const timer = setTimeout(onFinish, 600);
    return () => clearTimeout(timer);
  }, [progress, onFinish]);
  return <div className="flex flex-col gap-2" role="status">
    <span className="text-caption text-text-muted">{progress === 100 ? "Installed" : completed ? "Finishing…" : "Installing…"}</span>
    <div role="progressbar" aria-label={`${name} installation`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(progress)} className="h-1.5 overflow-hidden rounded-full bg-surface-raised"><div className="h-full rounded-full bg-accent transition-[width] duration-100 motion-reduce:transition-none" style={{width:`${progress}%`}} /></div>
  </div>;
}
