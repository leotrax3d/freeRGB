import { X } from "lucide-react";
import { useEffect, useLayoutEffect, useState } from "react";
import { create } from "zustand";
import { BrandTiles } from "@/components/brand";
import { Button } from "@/components/ui/button";
import { useNavigation } from "./navigation";
import type { ViewId } from "./views";

const SEEN_KEY = "freergb.tour.v1";

interface Step {
  /** `data-tour` value of the element to highlight; none = centered card. */
  target?: string;
  view?: ViewId;
  title: string;
  text: string;
}

const STEPS: Step[] = [
  {
    title: "Welcome to freeRGB",
    text: "Your lighting runs in a small background service. This window is only the remote control — close it whenever you like, the lights keep running.",
  },
  { target: "nav", title: "Everything in one place", text: "Light, hardware and automation, grouped on the left. Arrow keys move through the list." },
  { target: "search", title: "Quick search", text: "Press Ctrl+K anywhere to jump to a view, start an effect or load a profile." },
  { target: "now", view: "lighting", title: "What is running", text: "The current effect and the overall brightness for all devices." },
  { target: "effects", view: "lighting", title: "Pick an effect", text: "One click switches every device. Your own saved effects appear below the list." },
  { target: "inspector", view: "lighting", title: "Fine-tune it", text: "Colours, speed and more for the running effect. Changes apply live and are remembered by the service." },
  { target: "status", title: "Service status", text: "Shows whether the background service is running and how smoothly it renders." },
  {
    title: "You're set",
    text: "Place your devices under Layout so effects line up with your desk. You can start this tour again any time in Settings.",
  },
];

export const useTour = create<{ step: number | null; start: () => void; stop: () => void }>((set) => ({
  step: null,
  start: () => set({ step: 0 }),
  stop: () => {
    try {
      localStorage.setItem(SEEN_KEY, "1");
    } catch {
      // Not remembering only means the tour shows again next time.
    }
    set({ step: null });
  },
}));

/** Starts the tour on first launch. */
export function useFirstRunTour() {
  useEffect(() => {
    let seen = true;
    try {
      seen = localStorage.getItem(SEEN_KEY) === "1";
    } catch {
      // Storage unavailable: never nag.
    }
    if (!seen) setTimeout(() => useTour.getState().start(), 600);
  }, []);
}

function useTargetRect(target: string | undefined, step: number) {
  const [rect, setRect] = useState<DOMRect | null>(null);
  useLayoutEffect(() => {
    if (!target) return setRect(null);
    let frame = 0;
    // Views load lazily; look again for a short moment until the target exists.
    const started = performance.now();
    const find = () => {
      const el = document.querySelector(`[data-tour="${target}"]`);
      if (el) {
        el.scrollIntoView({ block: "nearest" });
        setRect(el.getBoundingClientRect());
      } else if (performance.now() - started < 1500) frame = requestAnimationFrame(find);
      else setRect(null);
    };
    find();
    const onResize = () => find();
    window.addEventListener("resize", onResize);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", onResize);
    };
  }, [target, step]);
  return rect;
}

export function Tour() {
  const { step, stop } = useTour();
  const go = useNavigation((s) => s.go);
  const current = step === null ? undefined : STEPS[step];
  useEffect(() => {
    if (current?.view) go(current.view);
  }, [current, go]);
  const rect = useTargetRect(current?.target, step ?? -1);

  useEffect(() => {
    if (step === null) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") stop();
      if (e.key === "ArrowRight") next();
      if (e.key === "ArrowLeft") back();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  if (step === null || !current) return null;
  const last = step === STEPS.length - 1;
  const next = () => (last ? stop() : useTour.setState({ step: step + 1 }));
  const back = () => step > 0 && useTour.setState({ step: step - 1 });

  const pad = 6;
  // Card next to the highlighted element when there is room, else centered.
  const card: React.CSSProperties = rect
    ? rect.right + 340 < window.innerWidth
      ? { left: rect.right + 16, top: Math.min(rect.top, window.innerHeight - 220) }
      : { left: Math.max(16, rect.left), top: rect.bottom + 16 < window.innerHeight - 200 ? rect.bottom + 16 : Math.max(16, rect.top - 216) }
    : { left: "50%", top: "50%", transform: "translate(-50%, -50%)" };

  return (
    <div className="fixed inset-0 z-[60]" role="dialog" aria-modal="true" aria-labelledby="tour-title">
      {rect ? (
        <div
          className="pointer-events-none absolute rounded-card transition-all duration-(--duration-slow) ease-(--ease-out)"
          style={{
            left: rect.left - pad,
            top: rect.top - pad,
            width: rect.width + pad * 2,
            height: rect.height + pad * 2,
            boxShadow: "0 0 0 2px rgb(16 163 127 / 0.8), 0 0 0 9999px rgb(0 0 0 / 0.6)",
          }}
        />
      ) : (
        <div className="absolute inset-0 bg-black/60" />
      )}
      <div
        className="absolute w-80 rounded-panel bg-surface-raised p-5 shadow-popover animate-[pop-in_var(--duration-base)_var(--ease-out)]"
        style={card}
        key={step}
      >
        <button type="button" onClick={stop} aria-label="Close tour" className="absolute right-3 top-3 rounded-chip p-1 text-text-subtle hover:text-text">
          <X className="size-4" />
        </button>
        {!rect && <BrandTiles size={10} className="mb-3" />}
        <h2 id="tour-title" className="pr-6 text-title font-semibold">
          {current.title}
        </h2>
        <p className="mt-1.5 text-small text-text-muted">{current.text}</p>
        <div className="mt-5 flex items-center gap-2">
          <div className="flex gap-1" aria-label={`Step ${step + 1} of ${STEPS.length}`}>
            {STEPS.map((_, i) => (
              <span key={i} className={i === step ? "h-1.5 w-4 rounded-full bg-text" : "size-1.5 rounded-full bg-white/20"} />
            ))}
          </div>
          <div className="ml-auto flex gap-2">
            {step > 0 && (
              <Button size="sm" variant="ghost" onClick={back}>
                Back
              </Button>
            )}
            <Button size="sm" variant="primary" onClick={next} autoFocus>
              {last ? "Done" : step === 0 ? "Show me" : "Next"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
