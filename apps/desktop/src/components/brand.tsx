import { cn } from "@/lib/utils";

const TILES = [
  { letter: "R", color: "bg-danger" },
  { letter: "G", color: "bg-accent" },
  { letter: "B", color: "bg-info" },
] as const;

/** The logo's three R/G/B tiles; `letters` shows the glyphs at larger sizes. */
export function BrandTiles({ size = 10, letters = false, className }: { size?: number; letters?: boolean; className?: string }) {
  return (
    <span className={cn("inline-flex items-end gap-[3px]", className)} aria-hidden>
      {TILES.map((t) => (
        <span
          key={t.letter}
          className={cn(t.color, "grid place-items-center font-semibold text-text")}
          style={{
            width: size,
            height: size * 1.55,
            borderRadius: size * 0.28,
            fontSize: size * 0.8,
          }}
        >
          {letters ? t.letter : null}
        </span>
      ))}
    </span>
  );
}
