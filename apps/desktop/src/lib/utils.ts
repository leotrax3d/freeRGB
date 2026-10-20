import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Like {@link frameThrottle}, but with an independent stream per key (e.g. per device). */
export function keyedFrameThrottle<T>(send: (key: string, value: T) => void) {
  const streams = new Map<string, ReturnType<typeof frameThrottle<T>>>();
  const stream = (key: string) => {
    let s = streams.get(key);
    if (!s) streams.set(key, (s = frameThrottle<T>((value) => send(key, value))));
    return s;
  };
  return {
    push: (key: string, value: T) => stream(key).push(value),
    flush: (key: string, value: T) => stream(key).flush(value),
  };
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
