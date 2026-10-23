import { cn } from "@/lib/utils";
import ring from "@brand/ring-dark.svg";
import wordmark from "@brand/wordmark-dark.svg";
import loader from "@brand/loading-ring-monogram.svg";

/** Primary application mark: the segmented RGB Ring (assets/brand). */
export function BrandRing({ size = 24, className }: { size?: number; className?: string }) {
  return <img src={ring} width={size} height={size} alt="" aria-hidden className={cn("shrink-0 rounded-[22%]", className)} />;
}

/** Ring with the outlined freeRGB lettering. */
export function BrandWordmark({ height = 28, className }: { height?: number; className?: string }) {
  return <img src={wordmark} height={height} style={{ height, width: "auto" }} alt="freeRGB" className={className} />;
}

/** Looping Ring → f → Ring animation; static Ring for reduced motion (built into the SVG). */
export function BrandLoader({ size = 96, className, label = "Loading" }: { size?: number; className?: string; label?: string }) {
  return <img src={loader} width={size} height={size} alt={label} className={cn("shrink-0", className)} />;
}

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
