import { useId, useState, type CSSProperties } from "react"
import { cn } from "@/lib/utils"

// The app's SPECTRUM (apps/desktop/src/lib/api/effects.ts).
const colours = [
  { id: "red", label: "Red", hex: "#ef4444" },
  { id: "amber", label: "Amber", hex: "#f59e0b" },
  { id: "green", label: "Green", hex: "#10a37f" },
  { id: "blue", label: "Blue", hex: "#3b82f6" },
  { id: "violet", label: "Violet", hex: "#a855f7" },
] as const

const effects = [
  { id: "rainbow", label: "Rainbow wave" },
  { id: "breathing", label: "Breathing" },
  { id: "static", label: "Static" },
] as const

type EffectId = (typeof effects)[number]["id"]
type ColourId = (typeof colours)[number]["id"]

const LEDS = 12
const WAVE_SECONDS = 4

const copy = {
  title: "Preview an effect",
  text: "A simplified preview that runs in your browser. In the app, the same effect runs across all your devices at once.",
  effectLegend: "Effect",
  colourLegend: "Colour",
  colourHint: "Rainbow wave uses the full spectrum.",
}

// Keyframes live here because shared CSS is out of scope for this component.
// The global reduced-motion rule in index.css clamps these to a single 0.01 ms run,
// after which each LED shows its static base colour.
const keyframes = `
@keyframes frgb-wave {
  0%, 100% { color: ${colours[0].hex}; }
  20% { color: ${colours[1].hex}; }
  40% { color: ${colours[2].hex}; }
  60% { color: ${colours[3].hex}; }
  80% { color: ${colours[4].hex}; }
}
@keyframes frgb-breathe {
  0%, 100% { opacity: 0.2; }
  50% { opacity: 1; }
}
`

function ledStyle(effect: EffectId, colour: string, index: number): CSSProperties {
  if (effect === "rainbow") {
    // Static fallback: the spectrum spread across the strip.
    const base = colours[Math.floor((index / LEDS) * colours.length)].hex
    return {
      color: base,
      animation: `frgb-wave ${WAVE_SECONDS}s linear infinite`,
      animationDelay: `${-((LEDS - index) / LEDS) * WAVE_SECONDS}s`,
    }
  }
  if (effect === "breathing") {
    return { color: colour, animation: "frgb-breathe 3s ease-in-out infinite" }
  }
  return { color: colour }
}

export function ColorDemo({ className }: { className?: string }) {
  const [effect, setEffect] = useState<EffectId>("rainbow")
  const [colourId, setColourId] = useState<ColourId>("green")
  const uid = useId()
  const colour = colours.find((c) => c.id === colourId) ?? colours[2]
  const effectLabel = effects.find((e) => e.id === effect)?.label ?? ""
  const description = effect === "rainbow" ? `${effectLabel} across ${LEDS} LEDs` : `${effectLabel} in ${colour.label.toLowerCase()} on ${LEDS} LEDs`

  return (
    <div className={cn("rounded-card border bg-card p-5", className)}>
      <style>{keyframes}</style>
      <h3 className="text-body font-semibold">{copy.title}</h3>
      <p className="mt-1 text-small text-muted-foreground">{copy.text}</p>

      {/* The strip sits on a dark solid surface in both themes so the LEDs read as light. */}
      <div role="img" aria-label={`Preview: ${description}`} className="mt-4 rounded-lg border bg-neutral-950 px-3 py-4">
        <div className="flex items-center justify-between gap-1">
          {Array.from({ length: LEDS }, (_, i) => (
            <span
              key={i}
              aria-hidden
              className="block size-3 shrink-0 rounded-full bg-current motion-reduce:!animate-none sm:size-3.5"
              style={{ ...ledStyle(effect, colour.hex, i), boxShadow: "0 0 10px 1px currentColor" }}
            />
          ))}
        </div>
      </div>
      <p className="sr-only" aria-live="polite">
        {`Showing ${description}.`}
      </p>

      <div className="mt-4 flex flex-col gap-4 sm:flex-row sm:flex-wrap sm:items-start sm:justify-between">
        <fieldset>
          <legend className="text-caption text-subtle-foreground">{copy.effectLegend}</legend>
          <div className="mt-2 inline-flex rounded-lg bg-muted p-1">
            {effects.map((e) => (
              <label key={e.id} className="relative">
                <input
                  type="radio"
                  name={`${uid}-effect`}
                  value={e.id}
                  checked={effect === e.id}
                  onChange={() => setEffect(e.id)}
                  className="peer sr-only"
                />
                <span className="flex min-h-9 cursor-pointer items-center rounded-md px-3 text-small text-muted-foreground transition-colors duration-fast ease-out hover:text-foreground peer-checked:bg-background peer-checked:text-foreground peer-checked:shadow-sm peer-focus-visible:outline peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-ring">
                  {e.label}
                </span>
              </label>
            ))}
          </div>
        </fieldset>

        <fieldset disabled={effect === "rainbow"} aria-describedby={effect === "rainbow" ? `${uid}-hint` : undefined} className="group">
          <legend className="text-caption text-subtle-foreground">{copy.colourLegend}</legend>
          <div className="mt-2 flex gap-1 group-disabled:opacity-40">
            {colours.map((c) => (
              <label key={c.id} className="relative" title={c.label}>
                <input
                  type="radio"
                  name={`${uid}-colour`}
                  value={c.id}
                  checked={colourId === c.id}
                  onChange={() => setColourId(c.id)}
                  className="peer sr-only"
                />
                <span className="sr-only">{c.label}</span>
                <span
                  aria-hidden
                  className="flex size-11 cursor-pointer items-center justify-center rounded-md group-disabled:cursor-not-allowed peer-focus-visible:outline peer-focus-visible:outline-2 peer-focus-visible:outline-offset-1 peer-focus-visible:outline-ring peer-checked:ring-2 peer-checked:ring-inset peer-checked:ring-foreground/70 sm:size-9"
                >
                  <span className="block size-5 rounded-full" style={{ backgroundColor: c.hex }} />
                </span>
              </label>
            ))}
          </div>
          <p id={`${uid}-hint`} className={cn("mt-1 text-caption text-subtle-foreground", effect !== "rainbow" && "invisible")}>
            {copy.colourHint}
          </p>
        </fieldset>
      </div>
    </div>
  )
}
