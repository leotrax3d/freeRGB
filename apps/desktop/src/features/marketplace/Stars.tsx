import { Star } from "lucide-react";
import { useId } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { t } from "./messages";
import type { Ratings } from "./types";
export function Stars({ id, name, ratings, disabled, rate }: { id: string; name: string; ratings: Ratings; disabled: boolean; rate: (stars: number | null) => void }) {
  const group = useId();
  const summary = ratings.by_effect[id];
  const mine = ratings.mine[id];
  const busy = ratings.loading || !!ratings.saving;
  return <div className="flex flex-col gap-1.5 border-t border-border pt-3">
    <div className="flex items-center gap-2 text-small" aria-label={t("market.stars.overall")} aria-live="polite"><span className="flex gap-0.5" aria-hidden>{[1,2,3,4,5].map(value => <Star key={value} className={cn("size-3.5",summary && value <= Math.round(summary.average) ? "fill-amber-400 text-amber-400" : "text-text-subtle")} />)}</span><span className="text-amber-400">{summary && summary.count > 0 ? summary.average.toFixed(1) : "—"}</span><span className="text-text-subtle">({summary?.count ?? 0})</span></div>
    <fieldset disabled={disabled || busy || !ratings.configured} className="flex items-center gap-1">
      <legend className="sr-only">{t("market.stars.rate", { name })}</legend>
      {[1, 2, 3, 4, 5].map((value) => <label key={value} className="cursor-pointer rounded-control p-1 has-disabled:cursor-default has-disabled:opacity-40 has-focus-visible:ring-2 has-focus-visible:ring-accent">
        <input type="radio" className="sr-only" name={group} value={value} checked={mine === value} aria-label={value === 1 ? t("market.stars.one", { name }) : t("market.stars.many", { n: value, name })} onChange={() => rate(value)} />
        <Star aria-hidden className={cn("size-5", mine && value <= mine ? "fill-amber-400 text-amber-400" : "text-text-subtle")} />
      </label>)}
    </fieldset>
    <div className="flex min-h-5 items-center justify-between text-caption text-text-subtle"><span>{ratings.saving === id ? t("market.stars.saving") : mine ? t("market.stars.mine", { n: mine }) : t("market.stars.yours")}</span>{mine && <Button variant="ghost" size="sm" disabled={disabled || busy} onClick={() => rate(null)}>{t("market.stars.remove")}</Button>}</div>
  </div>;
}
