import data from "virtual:release"

// Measured values from docs/GUI_MODERNIZATION.md ("Memory tuning (release build, 2026-10-20)"
// and the backend measurement above it). Update both together.
const installer = data.release?.assets.find((a) => /windows.*setup\.exe$/i.test(a.name))

type Part = number | string
const stats: { parts: Part[]; unit: string; label: string; decimals?: number }[] = [
  { parts: [38], unit: "MB", label: "Background service (Rust) memory" },
  { parts: ["~", 0.2], unit: "%", label: "CPU while idle", decimals: 1 },
  { parts: [73, "–", 78], unit: "MB", label: "App window open, incl. WebView2" },
  { parts: [7, "–", 8], unit: "MB", label: "Window minimized" },
  ...(installer ? [{ parts: [Math.round((installer.size / 1024 / 1024) * 10) / 10], unit: "MB", label: "Windows installer", decimals: 1 }] : []),
]

/** Numbers render final in the HTML; motion.ts counts them up from 0 ([data-count]). */
export function Footprint() {
  return (
    <section aria-labelledby="footprint-title" className="container pb-8 md:pb-12">
      <div data-reveal="" className="max-w-2xl">
        <p className="text-caption uppercase tracking-wider text-brand-foreground">Built in Rust</p>
        <h2 id="footprint-title" className="mt-3 text-h2">
          Lightweight by design
        </h2>
        <p className="mt-4 text-body text-muted-foreground">
          The device service and effect engine are written in Rust and run in the background. The window only opens when
          you need it.
        </p>
      </div>
      <dl data-stats="" className="mt-10 grid grid-cols-2 gap-4 md:grid-cols-3 lg:grid-cols-5">
        {stats.map((s) => (
          /* dt stays first for valid markup; flex-col-reverse puts the value on top so all numbers line up. */
          <div key={s.label} data-stat="" className="flex flex-col-reverse justify-end rounded-card border bg-card p-5">
            <dt className="mt-3 text-small text-muted-foreground">{s.label}</dt>
            <dd className="flex items-baseline gap-1.5 whitespace-nowrap font-semibold tracking-tight tabular-nums">
              <span className="text-[clamp(30px,3.4vw,44px)] leading-none">
                {s.parts.map((p, i) =>
                  typeof p === "number" ? (
                    <span key={i} data-count={p} data-decimals={s.decimals ?? 0}>
                      {p.toFixed(s.decimals ?? 0)}
                    </span>
                  ) : (
                    <span key={i}>{p}</span>
                  ),
                )}
              </span>
              <span className="text-body text-muted-foreground">{s.unit}</span>
            </dd>
          </div>
        ))}
      </dl>
    </section>
  )
}
