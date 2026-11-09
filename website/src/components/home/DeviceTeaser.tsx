import { useRef, useState, type FormEvent } from "react"
import { ArrowRight, Search } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { deviceTeaser } from "@/content/site"
import { href } from "@/lib/paths"

const examples = ["K70", "Aura", "1b1c:1bfd"]
const inputLabel = "Product name or USB ID"
const submitLabel = "Search devices"

export function DeviceTeaser() {
  const [query, setQuery] = useState("")
  const inputRef = useRef<HTMLInputElement>(null)

  // CSP has form-action 'none', so the form never submits natively.
  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const value = query.trim()
    window.location.assign(value ? `${href("devices/")}?q=${encodeURIComponent(value)}` : href("devices/"))
  }

  return (
    <section aria-labelledby="device-check-title" className="container py-16 md:py-24">
      <div data-reveal="" className="grid gap-8 rounded-card border bg-card p-6 md:p-10 lg:grid-cols-[1fr_1.2fr] lg:items-center lg:gap-12">
        <div>
          <h2 id="device-check-title" className="text-h2">
            {deviceTeaser.title}
          </h2>
          <p className="mt-4 max-w-xl text-body text-muted-foreground">{deviceTeaser.text}</p>
        </div>
        <div>
          <form role="search" aria-label="Device list" onSubmit={onSubmit} noValidate>
            <label htmlFor="device-check-input" className="text-small font-medium">
              {inputLabel}
            </label>
            <div className="mt-2 flex flex-col gap-3 sm:flex-row">
              <div className="relative flex-1">
                <Search aria-hidden className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-subtle-foreground" />
                <Input
                  ref={inputRef}
                  id="device-check-input"
                  type="search"
                  autoComplete="off"
                  spellCheck={false}
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder={deviceTeaser.placeholder}
                  className="h-11 rounded-lg bg-background pl-9 text-base md:text-base"
                />
              </div>
              <Button type="submit" size="lg">
                {submitLabel}
              </Button>
            </div>
          </form>
          <div className="mt-4 flex flex-wrap items-center gap-2">
            <span className="text-small text-subtle-foreground">Try:</span>
            {examples.map((example) => (
              <button
                key={example}
                type="button"
                onClick={() => {
                  setQuery(example)
                  inputRef.current?.focus()
                }}
                className="inline-flex min-h-11 items-center rounded-md border border-border-strong px-3 font-mono md:min-h-9 text-[13px] text-muted-foreground transition-colors duration-fast ease-out hover:bg-accent hover:text-foreground"
              >
                <span className="sr-only">Fill in </span>
                {example}
              </button>
            ))}
          </div>
          <a
            href={href("devices/")}
            className="mt-5 inline-flex min-h-11 items-center gap-1.5 rounded-md text-small font-medium text-brand-foreground underline-offset-4 hover:underline"
          >
            {deviceTeaser.cta}
            <ArrowRight aria-hidden className="size-4" />
          </a>
        </div>
      </div>
    </section>
  )
}
