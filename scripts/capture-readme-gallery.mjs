// Capture the actual frontend with its built-in browser mocks; never use hardware.
// Start `npm run dev -- --host 127.0.0.1` in apps/desktop first.
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const base = new URL(process.env.FREERGB_BROWSER_URL || 'http://127.0.0.1:1420/');
assert.equal(base.hostname, '127.0.0.1', 'Use the local mock development server');
const output = new URL('../docs/screenshots/', import.meta.url);
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH || undefined,
  headless: true,
  args: ['--no-sandbox'],
});
const errors = [];
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 1, reducedMotion: 'reduce' });
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    const url = new URL(route.request().url());
    return url.origin === base.origin ? route.continue() : route.abort();
  });
  await page.goto(base.href);
  await page.evaluate(async () => {
    const { transport } = await import('/src/lib/api/transport.ts');
    if ((await transport()).kind !== 'mock') throw Error('Mock transport required');
  });
  await page.getByRole('button', { name: 'Get started', exact: true }).click();
  await page.getByRole('button', { name: /Skip — set up lighting only/ }).click();
  await page.getByRole('button', { name: 'Skip devices', exact: true }).click();
  await page.getByRole('button', { name: 'Open freeRGB', exact: true }).click();
  await page.getByRole('button', { name: 'Close tour', exact: true }).click();
  for (const [nav, heading, filename] of [
    ['Lighting', 'Lighting', 'lighting'],
    ['Layout', 'Layout', 'layout'],
    ['Marketplace', 'Effect Marketplace', 'marketplace'],
    ['Monitoring', 'Monitoring', 'monitoring'],
    ['Cooling', 'Cooling', 'cooling'],
    ['Profiles', 'Profiles', 'profiles'],
    ['Macros', 'Macros', 'macros'],
    ['Devices', 'Devices', 'devices'],
    ['Settings', 'Settings', 'settings'],
  ]) {
    await page.getByRole('navigation', { name: 'Main', exact: true }).getByRole('button', { name: nav, exact: true }).click();
    await page.getByRole('main').getByRole('heading', { name: heading, exact: true }).waitFor();
    await page.evaluate(() => document.fonts.ready);
    // Allow the initial mock subscription/chart render to settle.
    await page.waitForTimeout(500);
    await page.screenshot({ path: fileURLToPath(new URL(`${filename}.png`, output)), animations: 'disabled' });
    console.log(`Captured ${filename}.png (browser mock)`);
  }
  assert.deepEqual(errors, [], 'Frontend must render without page errors');
} finally {
  await browser.close();
}
