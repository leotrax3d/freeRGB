import { useEffect, type RefObject } from "react";

const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type=hidden])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[role=slider]:not([aria-disabled=true])",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

/** Visible, keyboard-focusable elements inside a container, in DOM order. */
export function focusableIn(container: HTMLElement): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.offsetParent !== null || el === document.activeElement);
}

/**
 * Next index when Tab / Shift+Tab would leave a modal: wraps around so focus
 * never escapes into the (inert) page behind it. Pure, for tests.
 */
export function wrapIndex(current: number, count: number, backwards: boolean): number {
  if (count === 0) return -1;
  if (current < 0) return backwards ? count - 1 : 0;
  return (current + (backwards ? -1 : 1) + count) % count;
}

/**
 * Keeps Tab focus inside a modal dialog and restores focus to the element
 * that opened it when the dialog closes.
 */
export function useFocusTrap(ref: RefObject<HTMLElement | null>, active = true) {
  useEffect(() => {
    if (!active) return;
    const container = ref.current;
    if (!container) return;
    const opener = document.activeElement;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;
      const items = focusableIn(container);
      if (items.length === 0) {
        event.preventDefault();
        return;
      }
      const index = items.indexOf(document.activeElement as HTMLElement);
      const atEdge = event.shiftKey ? index <= 0 : index === items.length - 1;
      if (atEdge || index < 0) {
        event.preventDefault();
        items[wrapIndex(index, items.length, event.shiftKey)]?.focus();
      }
    };
    container.addEventListener("keydown", onKeyDown);
    return () => {
      container.removeEventListener("keydown", onKeyDown);
      if (opener instanceof HTMLElement && document.contains(opener)) opener.focus();
    };
  }, [ref, active]);
}
