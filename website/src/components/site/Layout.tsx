import { useEffect, useState, type ReactNode } from "react"
import { Github, Menu, Moon, Sun } from "lucide-react"
import ring from "../../../../assets/brand/ring-dark.svg"
import { Button } from "@/components/ui/button"
import { Sheet, SheetContent, SheetDescription, SheetTitle, SheetTrigger } from "@/components/ui/sheet"
import { blob, footer, nav, repo } from "@/content/site"
import { cn } from "@/lib/utils"
import { href } from "@/lib/paths"

export type PageId = "home" | "download" | "devices" | "docs" | "privacy"

function ThemeToggle() {
  // The real theme is set before paint by public/theme-init.js; React syncs after mount.
  const [dark, setDark] = useState(true)
  useEffect(() => setDark(document.documentElement.classList.contains("dark")), [])
  const toggle = () => {
    const next = !dark
    setDark(next)
    document.documentElement.classList.toggle("dark", next)
    try {
      localStorage.setItem("freergb-theme", next ? "dark" : "light")
    } catch {
      /* storage blocked: the toggle still works for this page view */
    }
  }
  return (
    <Button variant="ghost" size="icon" onClick={toggle} aria-label={dark ? "Switch to light theme" : "Switch to dark theme"}>
      {dark ? <Sun aria-hidden /> : <Moon aria-hidden />}
    </Button>
  )
}

function Logo() {
  return (
    <a href={href("")} className="flex items-center gap-2.5 rounded-md text-[17px] font-semibold tracking-tight">
      <img src={ring} alt="" width={28} height={28} className="size-7 rounded-[7px]" />
      freeRGB
    </a>
  )
}

function Header({ page }: { page: PageId }) {
  const current = (path: string) => (path === `${page}/` ? "page" : undefined)
  return (
    <header className="sticky top-0 z-40 border-b bg-background/80 backdrop-blur supports-[backdrop-filter]:bg-background/70">
      <div className="container flex h-16 items-center gap-6">
        <Logo />
        <nav aria-label="Main" className="ml-4 hidden md:block">
          <ul className="flex items-center gap-1">
            {nav.map((item) => (
              <li key={item.href}>
                <a
                  href={href(item.href)}
                  aria-current={current(item.href)}
                  className={cn(
                    "rounded-md px-3 py-2 text-small text-muted-foreground transition-colors duration-fast hover:text-foreground",
                    current(item.href) && "text-foreground",
                  )}
                >
                  {item.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>
        <div className="ml-auto flex items-center gap-1">
          <Button variant="ghost" size="icon" asChild>
            <a href={repo} aria-label="freeRGB source code on GitHub">
              <Github aria-hidden />
            </a>
          </Button>
          <ThemeToggle />
          <Button size="sm" className="ml-2 hidden sm:inline-flex" asChild>
            <a href={href("download/")}>Download</a>
          </Button>
          <Sheet>
            <SheetTrigger asChild>
              <Button variant="ghost" size="icon" className="md:hidden" aria-label="Open menu">
                <Menu aria-hidden />
              </Button>
            </SheetTrigger>
            <SheetContent side="right" className="w-72">
              <SheetTitle>freeRGB</SheetTitle>
              <SheetDescription className="sr-only">Site navigation</SheetDescription>
              <nav aria-label="Mobile" className="mt-6">
                <ul className="flex flex-col gap-1">
                  {[{ href: "", label: "Home" }, ...nav].map((item) => (
                    <li key={item.href}>
                      <a
                        href={href(item.href)}
                        aria-current={item.href === "" ? (page === "home" ? "page" : undefined) : current(item.href)}
                        className="block rounded-md px-3 py-3 text-body text-muted-foreground hover:bg-accent hover:text-foreground aria-[current=page]:text-foreground"
                      >
                        {item.label}
                      </a>
                    </li>
                  ))}
                  <li>
                    <a href={repo} className="block rounded-md px-3 py-3 text-body text-muted-foreground hover:bg-accent hover:text-foreground">
                      GitHub
                    </a>
                  </li>
                </ul>
              </nav>
            </SheetContent>
          </Sheet>
        </div>
      </div>
    </header>
  )
}

function Footer() {
  const columns = [
    { title: "Product", links: [{ label: "Download", url: href("download/") }, { label: "Devices", url: href("devices/") }, { label: "Web demo", url: href("demo/") }] },
    { title: "Docs", links: [{ label: "User guide", url: href("docs/") }, { label: "Install guide", url: blob("docs/INSTALL.md") }, { label: "Project status", url: blob("docs/STATUS.md") }] },
    { title: "Project", links: [{ label: "Source code", url: repo }, { label: "Report a bug", url: `${repo}/issues` }, { label: "Privacy and legal", url: href("privacy/") }] },
  ]
  return (
    <footer className="mt-24 border-t">
      <div className="container grid gap-10 py-14 md:grid-cols-[1.4fr_repeat(3,1fr)]">
        <div className="space-y-4">
          <Logo />
          <p className="max-w-xs text-small text-muted-foreground">{footer.line}</p>
        </div>
        {columns.map((column) => (
          <div key={column.title}>
            <h2 className="text-caption uppercase tracking-wider text-subtle-foreground">{column.title}</h2>
            <ul className="mt-4 space-y-3">
              {column.links.map((link) => (
                <li key={link.label}>
                  <a href={link.url} className="text-small text-muted-foreground transition-colors duration-fast hover:text-foreground">
                    {link.label}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
      <div className="border-t">
        <div className="container flex flex-col gap-2 py-6 text-small text-subtle-foreground sm:flex-row sm:justify-between">
          <p>MIT or Apache-2.0 · modules GPL-2.0-or-later</p>
          <p>{footer.affiliation}</p>
        </div>
      </div>
    </footer>
  )
}

export function Layout({ page, children }: { page: PageId; children: ReactNode }) {
  return (
    <>
      <a
        href="#main"
        className="sr-only z-50 rounded-md bg-primary px-4 py-3 text-primary-foreground focus:not-sr-only focus:fixed focus:left-4 focus:top-3"
      >
        Skip to content
      </a>
      <Header page={page} />
      <main id="main" tabIndex={-1} className="outline-none">
        {children}
      </main>
      <Footer />
    </>
  )
}
