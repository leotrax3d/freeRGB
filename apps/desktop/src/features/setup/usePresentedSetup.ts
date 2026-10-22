import { useEffect, useRef, useState } from "react";
import type { ModulesStatus } from "@/lib/api/types";
import { advanceSetup } from "./presentation";

export function usePresentedSetup(latest: ModulesStatus | null | undefined) {
  const shown = useRef(latest);
  const [value, setValue] = useState(latest);
  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      shown.current = latest;
      setValue(latest);
      return;
    }
    let frame = 0;
    let time = performance.now();
    const tick = (now: number) => {
      const next = latest && shown.current ? advanceSetup(shown.current, latest, Math.min(100, now - time)) : latest;
      shown.current = next;
      time = now;
      setValue(next);
      if (next !== latest) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [latest]);
  return value;
}
