import { EyeOff, Github, HardDrive, Laptop, Scale, type LucideIcon } from "lucide-react"
import { repo, trust } from "@/content/site"

const icons: LucideIcon[] = [Scale, HardDrive, Laptop, EyeOff]

export function TrustStrip() {
  return (
    <section aria-labelledby="trust-title" className="container py-16 md:py-24">
      <h2 id="trust-title" className="sr-only">
        Project facts
      </h2>
      <ul data-batch="" className="grid gap-px overflow-hidden rounded-card border bg-border sm:grid-cols-2 lg:grid-cols-4">
        {trust.map((item, i) => {
          const Icon = icons[i] ?? Scale
          return (
            <li key={item.title} className="bg-background p-6">
              <Icon aria-hidden className="size-5 text-muted-foreground" strokeWidth={1.75} />
              <h3 className="mt-4 text-body font-semibold">{item.title}</h3>
              <p className="mt-1.5 text-small text-muted-foreground">{item.text}</p>
            </li>
          )
        })}
      </ul>
      <p className="mt-6 text-center">
        <a
          href={repo}
          className="inline-flex min-h-11 items-center gap-2 rounded-md px-2 text-small text-muted-foreground transition-colors duration-fast ease-out hover:text-foreground"
        >
          <Github aria-hidden className="size-4" />
          View source on GitHub
        </a>
      </p>
    </section>
  )
}
