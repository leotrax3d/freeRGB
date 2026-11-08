// Writes the server-rendered markup of every page into its built HTML file, so the
// content is visible before JavaScript runs (fast first paint, no layout shift).
import { readFileSync, rmSync, writeFileSync } from "node:fs"
import { fileURLToPath, pathToFileURL } from "node:url"

const dist = fileURLToPath(new URL("../dist/", import.meta.url))
const ssr = fileURLToPath(new URL("../dist-ssr/", import.meta.url))
const { render } = await import(pathToFileURL(`${ssr}render.js`).href)

const pages = { home: "index.html", download: "download/index.html", devices: "devices/index.html", docs: "docs/index.html", privacy: "privacy/index.html" }
for (const [page, file] of Object.entries(pages)) {
  const path = dist + file
  const html = readFileSync(path, "utf8")
  if (!html.includes("<!--app-->")) throw new Error(`${file}: missing <!--app--> placeholder`)
  writeFileSync(path, html.replace("<!--app-->", render(page)))
}
rmSync(ssr, { recursive: true, force: true })
console.log(`Prerendered ${Object.keys(pages).length} pages`)
