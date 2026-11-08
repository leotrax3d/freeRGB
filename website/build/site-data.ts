// Build-time data for the website. Everything is resolved when the site is built,
// so the published pages make no requests to other services.
import { existsSync, readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"
import { Marked, type Tokens } from "marked"
import type { Plugin } from "vite"

const root = (path: string) => fileURLToPath(new URL(`../../${path}`, import.meta.url))
const repoBlob = "https://github.com/leotrax3d/freeRGB/blob/main/"

/** Same slugs GitHub uses for heading anchors, so links like #7-troubleshooting keep working. */
export function slug(text: string) {
  return text
    .toLowerCase()
    .trim()
    .replace(/<[^>]+>/g, "")
    .replace(/[^\p{L}\p{N}\s_-]/gu, "")
    .replace(/\s/g, "-")
}

const escapeAttr = (value: string) => value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;")

/** Resolves a guide link: fragments stay, repo-relative paths go to GitHub, other schemes are dropped. */
function guideHref(href: string): string | null {
  if (href.startsWith("#")) return href
  try {
    const url = new URL(href, `${repoBlob}docs/`)
    return ["http:", "https:", "mailto:"].includes(url.protocol) ? url.href : null
  } catch {
    return null
  }
}

function readJson(path: string) {
  return JSON.parse(readFileSync(root(path), "utf8"))
}

// Device rows: [name, family, transport, ids, status, module]. Status codes:
// s = supported (implemented or hardware-verified; one label, see content/site.ts), b = blocked, n = not ported.
function devices() {
  const matrix = readJson("docs/openrgb-compatibility.json") as {
    devices: { name: string; family: string; transport: string; ids: string | null; implementation: string; status: string }[]
  }
  const code = (status: string) => (status === "hardware-verified" || status === "implemented" ? "s" : status.startsWith("blocked") ? "b" : "n")
  const rows = matrix.devices.map((d) => [
    d.name,
    d.family.replace(/Controller$/, ""),
    d.transport,
    d.ids ?? "",
    code(d.status),
    d.implementation.replace(/^(module|native):/, ""),
  ])
  return { rows }
}

function fans() {
  const coverage = readJson("docs/fan-coverage.json") as {
    lhm_version: string
    chips: { chip: string; name: string; vendor: string; typical_boards: string[]; fans: number; controls: number; status: string }[]
  }
  return {
    lhmVersion: coverage.lhm_version,
    chips: coverage.chips.map((c) => ({
      chip: c.chip,
      name: c.name,
      vendor: c.vendor,
      boards: c.typical_boards.slice(0, 4),
      fans: c.fans,
      controls: c.controls,
      status: c.status,
    })),
  }
}

function guide() {
  const source = readFileSync(root("docs/USER_GUIDE.md"), "utf8")
  const toc: { id: string; text: string; level: number }[] = []
  const marked = new Marked({
    gfm: true,
    renderer: {
      heading({ tokens, depth, text }: Tokens.Heading) {
        if (depth === 1) return "" // The page renders its own title.
        const id = slug(text)
        if (depth <= 3) toc.push({ id, text: text.replace(/`/g, ""), level: depth })
        const tag = `h${Math.min(depth, 4)}`
        return `<${tag} id="${escapeAttr(id)}">${this.parser.parseInline(tokens)}</${tag}>\n`
      },
      link({ href, tokens }: Tokens.Link) {
        const label = this.parser.parseInline(tokens)
        const target = guideHref(href)
        if (target === null) return label
        const rel = target.startsWith("#") ? "" : ' rel="noopener"'
        return `<a href="${escapeAttr(target)}"${rel}>${label}</a>`
      },
    },
  })
  const html = marked.parse(source, { async: false }) as string
  return { html, toc }
}

type Release = {
  tag: string
  name: string
  url: string
  publishedAt: string
  prerelease: boolean
  assets: { name: string; size: number; url: string; sha256: string | null }[]
}

/** Written by build/fetch-release.mjs. Missing file (offline build) means "no release known". */
function release(): { release: Release | null; stars: number | null; fetchedAt: string | null } {
  const path = fileURLToPath(new URL("../src/data/release.generated.json", import.meta.url))
  if (!existsSync(path)) return { release: null, stars: null, fetchedAt: null }
  return JSON.parse(readFileSync(path, "utf8"))
}

const modules: Record<string, () => unknown> = {
  "virtual:devices": devices,
  "virtual:fans": fans,
  "virtual:guide": guide,
  "virtual:release": release,
}

export function siteData(): Plugin {
  return {
    name: "freergb-site-data",
    resolveId(id) {
      return id in modules ? `\0${id}` : null
    },
    load(id) {
      if (!id.startsWith("\0virtual:")) return null
      const name = id.slice(1)
      const value = modules[name]()
      return `export default ${JSON.stringify(value)}`
    },
    configureServer(server) {
      const watched = ["docs/openrgb-compatibility.json", "docs/fan-coverage.json", "docs/USER_GUIDE.md"].map(root)
      server.watcher.add(watched)
    },
  }
}
