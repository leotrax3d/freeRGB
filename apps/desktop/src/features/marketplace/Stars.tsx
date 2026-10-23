import { Star } from "lucide-react";
import { useId } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { Ratings } from "./types";
export function Stars({ id, name, ratings, disabled, rate }: { id: string; name: string; ratings: Ratings; disabled: boolean; rate: (stars: number | null) => void }) {
  const group = useId();
  const summary = ratings.by_effect[id];
  const mine = ratings.mine[id];
  const busy = ratings.loading || !!ratings.saving;
  return <div className="flex flex-col gap-1.5 border-t border-border pt-3">
    <p className="text-small text-text-muted" aria-live="polite">{summary ? <><span className="font-medium text-text">{summary.average.toFixed(1)} / 5</span> · {summary.count} {summary.count === 1 ? "rating" : "ratings"}</> : !ratings.configured ? "Community ratings not connected" : ratings.updated_at ? "No ratings yet" : "Community score not loaded"}</p>
    <fieldset disabled={disabled || busy || !ratings.configured} className="flex items-center gap-1">
      <legend className="sr-only">Rate {name}</legend>
      {[1, 2, 3, 4, 5].map((value) => <label key={value} className="cursor-pointer rounded-control p-1 has-disabled:cursor-default has-disabled:opacity-40 has-focus-visible:ring-2 has-focus-visible:ring-accent">
        <input type="radio" className="sr-only" name={group} value={value} checked={mine === value} aria-label={`${value} ${value === 1 ? "star" : "stars"} for ${name}`} onChange={() => rate(value)} />
        <Star aria-hidden className={cn("size-5", mine && value <= mine ? "fill-amber-400 text-amber-400" : "text-text-subtle")} />
      </label>)}
    </fieldset>
    <div className="flex min-h-5 items-center justify-between text-caption text-text-subtle"><span>{ratings.saving === id ? "Saving rating…" : mine ? `Your rating: ${mine}/5 · no account needed` : "Rate without registration"}</span>{mine && <Button variant="ghost" size="sm" disabled={disabled || busy} onClick={() => rate(null)}>Remove</Button>}</div>
  </div>;
}
