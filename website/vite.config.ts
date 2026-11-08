import { defineConfig, type Plugin } from "vite"
import react from "@vitejs/plugin-react"
import { fileURLToPath } from "node:url"
import { siteData } from "./build/site-data"

const here = (path: string) => fileURLToPath(new URL(path, import.meta.url))

// Multi-page build: every page is a real HTML file, so GitHub Pages serves it
// without a SPA fallback. build/prerender.mjs fills each page with server-rendered markup.
const pages = ["index", "download", "devices", "docs", "privacy"]

// The site loads nothing from other origins. connect-src 'self': the pages make no fetches; 'self' only lets tools such as Lighthouse read robots.txt.
// Only added to the production build, because the dev server injects inline scripts.
const csp = [
  "default-src 'self'",
  "script-src 'self'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data:",
  "font-src 'self'",
  "connect-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'none'",
].join("; ")

function contentSecurityPolicy(): Plugin {
  return {
    name: "freergb-csp",
    apply: "build",
    transformIndexHtml: (html) =>
      html.replace("<meta charset=\"UTF-8\" />", `<meta charset="UTF-8" />\n    <meta http-equiv="Content-Security-Policy" content="${csp}" />`),
  }
}

// Preload the Latin subset of Inter so the first paint already uses the web font.
function preloadFont(): Plugin {
  return {
    name: "freergb-preload-font",
    apply: "build",
    transformIndexHtml: {
      order: "post",
      handler(html, ctx) {
        const font = Object.keys(ctx.bundle ?? {}).find((file) => /inter-latin-wght-normal-[\w-]+\.woff2$/.test(file))
        if (!font) return html
        const base = process.env.BASE_PATH || "/"
        return html.replace("</title>", `</title>
    <link rel="preload" href="${base}${font}" as="font" type="font/woff2" crossorigin />`)
      },
    },
  }
}

export default defineConfig(({ isSsrBuild }) => ({
  base: process.env.BASE_PATH || "/",
  plugins: [react(), siteData(), contentSecurityPolicy(), preloadFont()],
  resolve: { alias: { "@": here("./src") } },
  build: isSsrBuild
    ? { outDir: "dist-ssr", emptyOutDir: true, copyPublicDir: false }
    : {
        target: "es2020",
        cssCodeSplit: false,
        rollupOptions: {
          input: Object.fromEntries(pages.map((page) => [page, here(page === "index" ? "./index.html" : `./${page}/index.html`)])),
        },
      },
}))
