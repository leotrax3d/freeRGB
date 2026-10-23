/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

// Tauri expects a fixed dev port and must not clear its own Rust output.
export default defineConfig(({ mode }) => ({
  plugins: [react(), tailwindcss(), {
    name: "demo-isolation",
    transformIndexHtml(html) {
      if (mode !== "demo") return html;
      return html.replace("<head>", `<head><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data:; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'">`).replace('<div id="root"></div>', '<div id="root"><p style="color:#eee;background:#111;padding:32px">Loading freeRGB demo…</p></div><noscript>This interactive demo requires JavaScript and a current browser.</noscript>');
    },
  }],
  clearScreen: false,
  base: mode === "demo" ? "./" : "/",
  define: {
    __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "dev"),
    __WEB_DEMO__: mode === "demo",
  },
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    modulePreload: mode === "demo" ? { polyfill: false } : undefined,
    outDir: mode === "demo" ? "dist-demo" : "dist",
    target: "es2022",
    chunkSizeWarningLimit: 300,
  },
  test: {
    environment: "node",
  },
}));
