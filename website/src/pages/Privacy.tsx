import { ArrowUpRight } from "lucide-react"
import { PageHeader } from "@/components/site/Section"
import { blob, legal, privacyPage } from "@/content/site"

const LINKS_TITLE = "Documents on GitHub"
const GITHUB_PRIVACY = {
  label: "GitHub privacy statement",
  url: "https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement",
}

const sectionId = (title: string) =>
  title
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-")

const linkClass =
  "inline-flex min-h-11 items-center gap-1 text-small text-brand-foreground underline-offset-4 transition-colors duration-fast ease-out hover:text-foreground hover:underline lg:min-h-0 lg:py-1"

export function PrivacyPage() {
  return (
    <>
      <PageHeader title={privacyPage.title} lead={privacyPage.lead} />
      <div className="container pb-16 md:pb-24">
        <div className="lg:grid lg:grid-cols-[minmax(0,1fr)_280px] lg:gap-16">
          <div className="max-w-3xl">
            {privacyPage.sections.map((section) => {
              const id = sectionId(section.title)
              return (
                <section key={section.title} aria-labelledby={id} className="border-t py-10 first:border-t-0 first:pt-0">
                  <h2 id={id} className="scroll-mt-24 text-h3 text-foreground">
                    {section.title}
                  </h2>
                  {section.body.map((paragraph) => (
                    <p key={paragraph} className="mt-4 text-body text-muted-foreground">
                      {paragraph}
                    </p>
                  ))}
                </section>
              )
            })}
          </div>

          <aside aria-labelledby="privacy-links" className="mt-6 lg:mt-0">
            <div className="rounded-card border bg-card p-5 lg:sticky lg:top-24">
              <h2 id="privacy-links" className="text-caption uppercase tracking-wider text-subtle-foreground">
                {LINKS_TITLE}
              </h2>
              <ul className="mt-3 space-y-1 lg:space-y-0.5">
                {privacyPage.links.map((link) => (
                  <li key={link.path}>
                    <a href={blob(link.path)} rel="noopener" className={linkClass}>
                      {link.label}
                      <ArrowUpRight aria-hidden="true" className="h-3.5 w-3.5 shrink-0" />
                    </a>
                  </li>
                ))}
                <li className="mt-2 border-t pt-2">
                  <a href={legal.imprint} rel="noopener" className={linkClass}>
                    Impressum
                    <ArrowUpRight aria-hidden="true" className="h-3.5 w-3.5 shrink-0" />
                  </a>
                </li>
                <li>
                  <a href={legal.privacy} rel="noopener" className={linkClass}>
                    Datenschutzerklärung
                    <ArrowUpRight aria-hidden="true" className="h-3.5 w-3.5 shrink-0" />
                  </a>
                </li>
                <li>
                  <a href={GITHUB_PRIVACY.url} rel="noopener" className={linkClass}>
                    {GITHUB_PRIVACY.label}
                    <ArrowUpRight aria-hidden="true" className="h-3.5 w-3.5 shrink-0" />
                  </a>
                </li>
              </ul>
            </div>
          </aside>
        </div>
      </div>
    </>
  )
}
