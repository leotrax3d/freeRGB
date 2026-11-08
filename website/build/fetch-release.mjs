// Fetches release metadata from GitHub at build time and writes
// src/data/release.generated.json. The published site never calls GitHub itself.
// Offline or rate-limited builds keep the previous file (or build without release data).
import { existsSync, mkdirSync, writeFileSync } from "node:fs"
import { fileURLToPath } from "node:url"

const repo = "leotrax3d/freeRGB"
const out = fileURLToPath(new URL("../src/data/release.generated.json", import.meta.url))
const headers = { Accept: "application/vnd.github+json", "User-Agent": "freergb-website-build" }
if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`

async function get(path) {
  const response = await fetch(`https://api.github.com/${path}`, { headers, signal: AbortSignal.timeout(15000) })
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`)
  return response.json()
}

try {
  const [releases, info] = await Promise.all([get(`repos/${repo}/releases?per_page=20`), get(`repos/${repo}`)])
  const latest = releases.find((r) => !r.draft && /^v\d/.test(r.tag_name)) ?? null
  const data = {
    fetchedAt: new Date().toISOString(),
    stars: typeof info.stargazers_count === "number" ? info.stargazers_count : null,
    release: latest && {
      tag: latest.tag_name,
      name: latest.name || latest.tag_name,
      url: latest.html_url,
      publishedAt: latest.published_at,
      prerelease: latest.prerelease,
      assets: latest.assets.map((a) => ({
        name: a.name,
        size: a.size,
        url: a.browser_download_url,
        sha256: typeof a.digest === "string" && a.digest.startsWith("sha256:") ? a.digest.slice(7) : null,
      })),
    },
  }
  mkdirSync(new URL("../src/data/", import.meta.url), { recursive: true })
  writeFileSync(out, JSON.stringify(data, null, 2) + "\n")
  console.log(`Release data: ${data.release?.tag ?? "no release"} (${data.release?.assets.length ?? 0} assets)`)
} catch (error) {
  console.warn(`Release data not refreshed: ${error.message}.${existsSync(out) ? " Keeping the previous file." : " Building without release data."}`)
}
