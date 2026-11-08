// Capture README/website screenshots from the browser demo build (mock backend,
// no hardware, no daemon). The UI has a dark theme only, so no light shots.
//
// Usage (Playwright installed outside the repo):
//   cd apps/desktop && npx vite build --mode demo --outDir <build>/demo
//   PLAYWRIGHT_BROWSERS_PATH=<build>/pw-browsers node scripts/capture-screenshots.mjs <build>/demo <build>/pw-tool
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { readFile } from "node:fs/promises";
import { extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const [dist, toolDir] = process.argv.slice(2);
if (!dist || !toolDir) throw new Error("usage: capture-screenshots.mjs <demo-dist> <playwright-dir>");
const { chromium } = createRequire(join(resolve(toolDir), "x.js"))("playwright");
const out = resolve(fileURLToPath(new URL("../docs/screenshots", import.meta.url)));
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".png": "image/png" };

const server = createServer(async (req, res) => {
  let path = decodeURIComponent((req.url ?? "/").split(/[?#]/)[0]);
  if (path === "/") path = "/index.html";
  try {
    const body = await readFile(join(dist, path));
    res.writeHead(200, { "content-type": types[extname(path)] ?? "application/octet-stream" }).end(body);
  } catch { res.writeHead(404).end(); }
}).listen(0);
await new Promise((r) => server.once("listening", r));
const base = `http://127.0.0.1:${server.address().port}/`;

const SHOTS = [
  ["lighting", "lighting"], ["layout", "layout"], ["marketplace", "marketplace"], ["profiles", "profiles"],
  ["devices", "devices"], ["headers", "headers"], ["cooling", "cooling"], ["monitoring", "monitoring"],
  ["macros", "macros"], ["settings", "settings"],
];
// Sidebar order (after the search button).
const ORDER = ["marketplace", "lighting", "layout", "profiles", "devices", "headers", "cooling", "monitoring", "macros", "settings"];
const browser = await chromium.launch();
// The mock starts in first-run state; click through the onboarding wizard once.
async function finishOnboarding(page) {
  for (let i = 0; i < 12 && (await page.locator("ol[aria-label]").count()) > 0; i++) {
    const wizard = page.locator("ol[aria-label]").first().locator("xpath=ancestor::*[.//button][last()]");
    const buttons = wizard.locator("button:not([disabled])");
    const n = await buttons.count();
    if (!n) break;
    // Safety step: skip (ghost) when present, otherwise the last button advances.
    await buttons.nth(n - 1).click().catch(() => {});
    await page.waitForTimeout(400);
  }
}
async function session(locale, shots) {
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 }, locale: locale ?? "en-US", reducedMotion: "reduce" });
  await ctx.addInitScript((l) => { try { localStorage.setItem("freergb.tour.v1", "1"); if (l) localStorage.setItem("freergb.locale", l); } catch {} }, locale);
  const page = await ctx.newPage();
  await page.goto(base);
  await page.waitForTimeout(1500);
  await finishOnboarding(page);
  for (const [view, name] of shots) {
    await page.locator("nav[aria-label] button").nth(1 + ORDER.indexOf(view)).click();
    await page.waitForTimeout(2500);
    await page.screenshot({ path: join(out, `${name}.png`) });
  }
  await ctx.close();
}
await session(null, SHOTS);
await session("de", [["lighting", "lighting-de"]]);
await browser.close();
server.close();
