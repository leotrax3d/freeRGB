import { useEffect, useState } from "react"

// The rolling word at the end of the hero headline. All words share one grid cell, so the
// drum is as wide as the longest word and never shifts the layout. The prerendered HTML
// shows the first word; JS rolls on every 2.6 s (src/index.css: [data-drum]).
const words = [
  { id: "rgb", label: "RGB" },
  { id: "lighting", label: "lighting" },
  { id: "fans", label: "fans" },
] as const

function Word({ id, label }: { id: (typeof words)[number]["id"]; label: string }) {
  if (id === "rgb")
    return (
      <span data-fx="rgb">
        <span>R</span>
        <span>G</span>
        <span>B</span>
      </span>
    )
  return <span data-fx={id}>{label}</span>
}

export function WordDrum() {
  const [active, setActive] = useState(0)
  useEffect(() => {
    const id = window.setInterval(() => setActive((i) => (i + 1) % words.length), 2600)
    return () => window.clearInterval(id)
  }, [])
  return (
    <span data-drum="" aria-hidden className="inline-grid">
      {words.map((w, i) => (
        <span key={w.id} data-state={i === active ? "in" : i === (active + words.length - 1) % words.length ? "out" : "wait"}>
          <Word id={w.id} label={w.label} />
        </span>
      ))}
    </span>
  )
}
