import { ArrowUpRight } from "lucide-react"
import guide from "virtual:guide"
import { PageHeader } from "@/components/site/Section"
import { GuideTocDesktop, GuideTocMobile, useActiveHeading } from "@/components/docs/GuideToc"
import { blob, docsPage, repo } from "@/content/site"
import { cn } from "@/lib/utils"

const GUIDE_PATH = "docs/USER_GUIDE.md"
const headingIds = guide.toc.map((entry) => entry.id)

/**
 * Extras on top of the .prose-guide styles in index.css: elements the shared styles do
 * not cover (h4, hr, images, nested and loose lists) and wrapping of long inline code
 * such as file names and pipe paths on narrow screens.
 */
const articleExtras = cn(
  "[&>:first-child]:mt-0",
  "[&_h2]:scroll-mt-24 [&_h3]:scroll-mt-24",
  "[&_h4]:mt-8 [&_h4]:scroll-mt-24 [&_h4]:text-body [&_h4]:font-semibold [&_h4]:text-foreground",
  "[&_hr]:my-12 [&_hr]:border-border",
  "[&_img]:h-auto [&_img]:max-w-full [&_img]:rounded-card [&_img]:border",
  "[&_li::marker]:text-subtle-foreground [&_li]:pl-1",
  "[&_li>p:first-child]:mt-0 [&_li>ol]:mt-2 [&_li>ul]:mt-2",
  "[&_:not(pre)>code]:[overflow-wrap:anywhere]",
  "[&_pre]:max-w-full",
)

function SourceNote() {
  const [before, after] = docsPage.source.split(GUIDE_PATH)
  const linkClass =
    "text-brand-foreground underline underline-offset-4 transition-colors duration-fast ease-out hover:text-foreground"
  return (
    <div className="mt-16 border-t pt-8 text-small text-muted-foreground">
      <p>
        {before}
        <a href={blob(GUIDE_PATH)} rel="noopener" className={cn(linkClass, "font-mono text-[0.875em]")}>
          {GUIDE_PATH}
        </a>
        {after}
      </p>
      <p className="mt-3">
        <a href={`${repo}/tree/main/docs`} rel="noopener" className={cn(linkClass, "inline-flex min-h-11 items-center gap-1 md:min-h-0")}>
          {docsPage.more}
          <ArrowUpRight aria-hidden="true" className="h-4 w-4" />
        </a>
      </p>
    </div>
  )
}

export function DocsPage() {
  const active = useActiveHeading(headingIds)
  const hasToc = guide.toc.length > 0

  return (
    <>
      <PageHeader title={docsPage.title} lead={docsPage.lead} />
      <div className="container pb-16 md:pb-24">
        <div className="lg:grid lg:grid-cols-[220px_minmax(0,1fr)] lg:gap-16">
          {hasToc && (
            <aside className="hidden lg:block">
              <GuideTocDesktop toc={guide.toc} active={active} />
            </aside>
          )}
          <div className="min-w-0">
            {hasToc && (
              <div className="mb-10 lg:hidden">
                <GuideTocMobile toc={guide.toc} active={active} />
              </div>
            )}
            <article
              className={cn("prose-guide max-w-3xl", articleExtras)}
              dangerouslySetInnerHTML={{ __html: guide.html }}
            />
            <div className="max-w-3xl">
              <SourceNote />
            </div>
          </div>
        </div>
      </div>
    </>
  )
}
