/** Injected by Vite from package.json. */
declare const __APP_VERSION__: string;
/** Compile-time boundary: a public demo can never select native IPC. */
declare const __WEB_DEMO__: boolean;
/** Brand SVGs imported as URLs (Vite asset handling). */
declare module "*.svg" {
  const src: string;
  export default src;
}
