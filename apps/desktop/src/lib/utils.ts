import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Wraps a sender so it fires at most once per animation frame while a control
 * is dragged; `flush` sends the last value immediately (use on release).
 */
export function frameThrottle<T>(send: (value: T) => void) {
  let pending: { value: T } | null = null;
  let frame = 0;
  const run = () => {
    frame = 0;
    if (pending) send(pending.value);
    pending = null;
  };
  return {
    push(value: T) {
      pending = { value };
      frame ||= requestAnimationFrame(run);
    },
    flush(value: T) {
      if (frame) cancelAnimationFrame(frame);
      frame = 0;
      pending = null;
      send(value);
    },
  };
}
