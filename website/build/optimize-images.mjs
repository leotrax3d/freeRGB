// Converts the README screenshots (docs/screenshots) into AVIF and WebP at several
// widths for the website. Run after the screenshots are refreshed:
//   npm install --no-save sharp && node build/optimize-images.mjs
// Output is committed to src/assets/screens so normal builds need no image tooling.
import { mkdirSync, readdirSync } from "node:fs"
import { fileURLToPath } from "node:url"
import { createRequire } from "node:module"

const require = createRequire(import.meta.url)
const sharp = require(process.env.SHARP_PATH || "sharp")
const src = fileURLToPath(new URL("../../docs/screenshots/", import.meta.url))
const out = fileURLToPath(new URL("../src/assets/screens/", import.meta.url))
const widths = [640, 960, 1600]
const names = ["lighting", "layout", "cooling", "monitoring", "profiles", "devices", "macros", "marketplace", "settings"]

mkdirSync(out, { recursive: true })
for (const name of names) {
  const file = readdirSync(src).find((f) => f === `${name}.png`)
  if (!file) throw new Error(`missing docs/screenshots/${name}.png`)
  for (const width of widths) {
    const image = sharp(src + file).resize({ width, withoutEnlargement: true })
    await image.clone().avif({ quality: 55, effort: 6 }).toFile(`${out}${name}-${width}.avif`)
    await image.clone().webp({ quality: 78 }).toFile(`${out}${name}-${width}.webp`)
  }
  console.log(`${name}: ${widths.join(", ")} px`)
}
