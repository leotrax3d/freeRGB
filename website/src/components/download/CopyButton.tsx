import { useEffect, useRef, useState } from "react"
import { Check, Copy } from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

type State = "idle" | "copied" | "failed"

async function writeClipboard(text: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text)
    return
  }
  // Fallback for browsers without the async clipboard API.
  const area = document.createElement("textarea")
  area.value = text
  area.setAttribute("readonly", "")
  area.style.position = "fixed"
  area.style.opacity = "0"
  document.body.appendChild(area)
  area.select()
  const ok = document.execCommand("copy")
  document.body.removeChild(area)
  if (!ok) throw new Error("copy failed")
}

/**
 * Copy button with a polite live region. The visible label keeps a fixed width
 * so switching between "Copy" and "Copied" does not move anything.
 */
export function CopyButton({ text, label, className }: { text: string; label: string; className?: string }) {
  const [state, setState] = useState<State>("idle")
  const timer = useRef<number | undefined>(undefined)
  useEffect(() => () => window.clearTimeout(timer.current), [])

  const copy = async () => {
    try {
      await writeClipboard(text)
      setState("copied")
    } catch {
      setState("failed")
    }
    window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => setState("idle"), 2000)
  }

  return (
    <>
      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={copy}
        aria-label={label}
        className={cn("min-h-10 min-w-[6.5rem] shrink-0 sm:min-h-9", className)}
      >
        {state === "copied" ? <Check aria-hidden className="text-brand" /> : <Copy aria-hidden />}
        <span aria-hidden>{state === "copied" ? "Copied" : state === "failed" ? "Failed" : "Copy"}</span>
      </Button>
      <span role="status" aria-live="polite" className="sr-only">
        {state === "copied" ? "Copied" : state === "failed" ? "Could not copy. Select the text and copy it by hand." : ""}
      </span>
    </>
  )
}
