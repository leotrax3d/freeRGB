import { cn } from "@/lib/utils";
import ring from "../../../../assets/brand/ring-dark.svg";
import wordmark from "../../../../assets/brand/wordmark-dark.svg";

/** Shared project artwork, also used by the website. */
export function BrandWordmark({ className }: { className?: string }) {
  return <img src={wordmark} alt="freeRGB" className={cn("h-8 w-auto shrink-0", className)} draggable={false} />;
}

/** Compatibility name for existing icon placements; renders the current ring logo. */
export function BrandTiles({ size = 10, className }: { size?: number; letters?: boolean; className?: string }) {
  return <img src={ring} alt="" aria-hidden className={cn("inline-block shrink-0", className)} style={{ width: size * 3 + 6, height: size * 3 + 6 }} draggable={false} />;
}
