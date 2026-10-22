import type { ModulesStatus } from "@/lib/api/types";

// Presentation may trail measured bytes, but never predicts progress.
const FILL_MS = 1600;
export function advanceSetup(previous: ModulesStatus, latest: ModulesStatus, elapsed: number): ModulesStatus {
  if (latest.job.cancelled) return latest;
  let pending = false;
  const advance = (from: number, to: number, total: number) => {
    const value = Math.min(to, Math.max(0, from) + total * Math.max(0, elapsed) / FILL_MS);
    if (value < to) pending = true;
    return value;
  };
  const cards = latest.cards.map((card) => {
    const old = previous.cards.find((item) => item.key === card.key);
    if (card.phase === "failed" || card.phase === "detected" || card.phase === "unsupported") return card;
    const progress = card.phase === "downloading" ? card.progress : old?.phase === "downloading" ? old.progress : null;
    if (!progress || progress.total <= 0) return card;
    const from = old?.phase === "downloading" && old.progress?.total === progress.total ? old.progress.done : 0;
    const target = card.phase === "downloading" ? progress.done : progress.total;
    const done = advance(from, target, progress.total);
    if (done < target || card.phase === "downloading") return { ...card, phase: "downloading" as const, progress: { done, total: progress.total } };
    return card;
  });
  const total = latest.job.bytes_total;
  const done = total > 0 ? advance(previous.job.bytes_total === total ? previous.job.bytes_done : 0, latest.job.bytes_done, total) : latest.job.bytes_done;
  const holding = cards.some((card, index) => card.phase === "downloading" && latest.cards[index]?.phase !== "downloading");
  if (!pending && !holding) return latest;
  return { ...latest, cards, job: { ...latest.job, bytes_done: done, running: latest.job.running || holding || pending } };
}
