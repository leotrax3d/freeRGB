import { CopyButton } from "./CopyButton"
import { cn } from "@/lib/utils"

/** Monospace box for commands and hashes, with a copy button. */
export function CodeBlock({
  code,
  copyLabel,
  caption,
  wrap = false,
  className,
}: {
  code: string
  copyLabel: string
  caption?: string
  /** Break long tokens (hashes) instead of scrolling horizontally. */
  wrap?: boolean
  className?: string
}) {
  return (
    <figure className={cn("min-w-0", className)}>
      {caption && <figcaption className="mb-2 text-caption text-subtle-foreground">{caption}</figcaption>}
      <div className="flex items-start gap-3 rounded-lg border bg-background p-3">
        <pre
          className={cn(
            "min-w-0 flex-1 py-2 font-mono text-[13px] leading-5 text-foreground",
            wrap ? "whitespace-pre-wrap break-all" : "overflow-x-auto",
          )}
          tabIndex={wrap ? undefined : 0}
        >
          <code>{code}</code>
        </pre>
        <CopyButton text={code} label={copyLabel} />
      </div>
    </figure>
  )
}
