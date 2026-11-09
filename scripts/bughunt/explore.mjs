// Isolated browser-demo exploration. Dependencies and every artifact stay external.
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, join, extname, sep } from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

if (process.argv.includes('--help')) {
  console.log('Usage: node scripts/bughunt/explore.mjs [demo-dist] [external-playwright-dir] [external-run-output]\nDefaults: E:/freergb-build/bughunt/demo E:/freergb-build/pw-tool E:/freergb-build/bughunt/cycle-1');
  process.exit(0);
}

const [distArg = 'E:/freergb-build/bughunt/demo', toolsArg = 'E:/freergb-build/pw-tool', outArg = 'E:/freergb-build/bughunt/cycle-1'] = process.argv.slice(2);
const dist = resolve(distArg), out = resolve(outArg);
if (out.startsWith(resolve('.') + sep)) throw new Error('Output must be outside the worktree');
process.env.PLAYWRIGHT_BROWSERS_PATH ??= 'E:/freergb-build/pw-browsers';
const require = createRequire(join(resolve(toolsArg), 'loader.cjs'));
const { chromium } = require('playwright');
const axe = await readFile(require.resolve('axe-core/axe.min.js'), 'utf8');
const manifest = JSON.parse((await readFile(new URL('./coverage-manifest.json', import.meta.url), 'utf8')).replace(/^\uFEFF/, ''));
const hash = s => createHash('sha256').update(String(s)).digest('hex').slice(0, 16);
await mkdir(join(out, 'findings'), { recursive: true });
await mkdir(join(out, 'screenshots'), { recursive: true });
const coverage = { started: new Date().toISOString(), baseCommit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), manifest, sessions: [], omissions: ['Native OS/file-picker behavior and real hardware are outside this demo run.', 'Bounded DOM crawl cannot prove all combinations or pointer drag interactions.', 'Controls may navigate away from the starting view. Inventories describe the final active DOM layer; controls left behind after navigation are not fully inventoried. Failed and omitted actions are retained individually.'], findings: 0 };
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json', '.png': 'image/png' };
const server = createServer(async (req, res) => {
  try {
    const pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
    const file = resolve(dist, '.' + (pathname === '/' ? '/index.html' : pathname));
    if (!file.startsWith(dist + sep)) { res.writeHead(403).end(); return; }
    const body = await readFile(file);
    res.writeHead(200, { 'content-type': types[extname(file)] ?? 'application/octet-stream' }).end(body);
  } catch { res.writeHead(404).end(); }
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}/`;
const seen = new Set();
let browser;
try {
  for (const locale of manifest.locales) for (const viewport of manifest.viewports) {
    browser = await chromium.launch();
    const session = { locale, viewport, views: [], events: [], actions: [], omissions: [] };
    coverage.sessions.push(session);
    const ctx = await browser.newContext({ locale, viewport: { width: viewport.width, height: viewport.height }, reducedMotion: 'reduce' });
    await ctx.addInitScript(l => { localStorage.setItem('freergb.locale', l); localStorage.setItem('freergb.tour.v1', '1'); }, locale);
    const page = await ctx.newPage();
    page.setDefaultTimeout(1800);
    let area = 'onboarding', steps = ['Open fresh demo session'];
    const pending = [];
    async function finding(rule, selector, text, extra = {}) {
      const fingerprint = `${area}|${rule}|${selector}|${hash(text)}`;
      if (seen.has(fingerprint)) return;
      seen.add(fingerprint);
      const id = hash(fingerprint), screenshot = join(out, 'screenshots', `${id}.png`);
      await page.screenshot({ path: screenshot }).catch(() => {});
      await writeFile(join(out, 'findings', `${id}.json`), JSON.stringify({ fingerprint, area, rule, selector, text, status: 'suspect', repro: [...steps], repro_steps: [...steps], expected: 'Usable, correctly rendered and accessible demo UI', actual: text, locale, viewport, screenshot, baseCommit: coverage.baseCommit, commit: coverage.baseCommit, ...extra }, null, 2));
      coverage.findings++;
    }
    for (const event of ['console', 'pageerror', 'requestfailed', 'response']) page.on(event, value => {
      const message = event === 'console' ? (value.type() === 'error' ? value.text() : null) : event === 'response' ? (value.status() >= 400 ? `${value.status()} ${value.url()}` : null) : event === 'requestfailed' ? `${value.url()} ${value.failure()?.errorText}` : String(value);
      if (message) { session.events.push({ area, event, message }); pending.push({ area, event, message, steps: [...steps] }); }
    });
    page.on('dialog', d => { session.events.push({ area, event: 'dialog', message: d.message() }); d.dismiss().catch(() => {}); });
    async function scan(full = false) {
      const suspects = await page.evaluate(() => {
        const result = [], visible = e => e.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true, contentVisibilityAuto: true });
        if (document.documentElement.scrollWidth > innerWidth + 2) result.push(['horizontal-overflow', 'html', `${document.documentElement.scrollWidth} > ${innerWidth}`]);
        for (const e of document.querySelectorAll('main, section, button, label, h1, h2, h3, p, span, [role=tab]')) {
          if (!visible(e)) continue;
          const text = (e.innerText ?? '').trim();
          const selector = e.id ? `#${CSS.escape(e.id)}` : `${e.tagName.toLowerCase()}${e.className && typeof e.className === 'string' ? '.' + e.className.trim().split(/\s+/).map(CSS.escape).join('.') : ''}`;
          if (e.children.length === 0 && /\b(?:undefined|NaN)\b|\[object Object\]|\b[a-z]+(?:\.[a-zA-Z_]+){2,}\b/.test(text)) result.push(['raw-text', selector, text]);
          if (text && e.children.length === 0 && e.clientWidth > 0 && e.scrollWidth > e.clientWidth + 3 && getComputedStyle(e).overflowX !== 'visible') result.push(['clipping', selector, text]);
          if (e.matches('main,section') && !text && !e.querySelector('canvas,svg,img,input')) result.push(['empty-section', selector, 'Visible section has no text or content']);
        }
        return result.slice(0, 35);
      });
      for (const [rule, selector, text] of suspects) await finding(rule, selector, text);
      if (full) {
        await page.evaluate(axe);
        const results = await page.evaluate(async () => (await window.axe.run(document, { resultTypes: ['violations'] })).violations);
        for (const v of results) for (const n of v.nodes.slice(0, 5)) await finding(`axe:${v.id}`, n.target.join(' '), v.help, { impact: v.impact, evidence: n.failureSummary });
      }
    }
    async function controls() {
      return page.locator(manifest.interactiveSelector).evaluateAll(elements => {
        const visible = e => e.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true, contentVisibilityAuto: true });
        const layer = [...document.querySelectorAll('[role=listbox], [role=menu], [role=dialog][aria-modal=true]')].filter(visible).at(-1);
        return elements.filter(e => visible(e) && !e.closest('nav') && (!layer || layer.contains(e))).map((e, i) => {
        e.setAttribute('data-bughunt-id', String(i));
        const parts = []; let node = e;
        while (node && node !== document.body) { const tag = node.tagName.toLowerCase(); parts.unshift(`${tag}:nth-of-type(${[...node.parentElement.children].filter(s => s.tagName === node.tagName).indexOf(node) + 1})`); node = node.parentElement; }
        return { id: i, selector: 'body > ' + parts.join(' > '), tag: e.tagName, type: e.getAttribute('type'), role: e.getAttribute('role'), name: e.getAttribute('aria-label') || e.getAttribute('title') || e.innerText || e.getAttribute('placeholder') || e.getAttribute('name') || '', disabled: e.matches(':disabled,[aria-disabled=true]'), keyboard: e.tabIndex >= 0, html: e.outerHTML.slice(0, 800), value: e.value, min: e.min, max: e.max, options: e.tagName === 'SELECT' ? [...e.options].map(o => o.value) : [] };
      }); });
    }
    async function crawl(budget) {
      const visited = new Set(); let count = 0;
      const identity = c => `${c.selector}|${c.tag}|${c.type}|${c.role}|${c.name}`;
      while (count < budget) {
        const list = (await controls()).filter(c => area !== 'onboarding' || ['INPUT', 'SELECT', 'TEXTAREA'].includes(c.tag));
        const c = list.find(c => !visited.has(identity(c)));
        if (!c) break;
        const key = identity(c);
        visited.add(key); count++;
        const selector = c.selector, el = page.locator(selector);
        const action = { area, control: c, result: 'pending' }; session.actions.push(action);
        if (c.html.includes('id="language-select"')) { action.result = 'omitted'; action.reason = 'Locale selector excluded to preserve EN/DE scenario; both locales have dedicated sessions'; continue; }
        if (c.disabled || c.type === 'file') { action.result = 'omitted'; action.reason = c.disabled ? 'disabled' : 'native file picker'; continue; }
        if (!c.name.trim() && !['INPUT', 'TEXTAREA', 'SELECT'].includes(c.tag)) await finding('accessible-name', selector, c.html);
        if (!c.keyboard) await finding('keyboard-suspect', selector, c.html);
        steps.push(`Activate ${c.tag} ${c.name.slice(0, 100)} (${selector})`);
        try {
          const before = await page.locator('body').innerText();
          if (c.tag === 'SELECT') {
            for (const value of c.options.slice(0, 16)) await el.selectOption(value);
            if (c.options.length > 16) session.omissions.push({ area, selector, reason: 'Select option budget', options: c.options.slice(16) });
          }
          else if (['INPUT', 'TEXTAREA'].includes(c.tag) && !['checkbox', 'radio', 'range', 'color', 'button', 'submit'].includes(c.type)) {
            const samples = c.type === 'number' ? ['', c.min || '0', c.max || '100', String(Number(c.min || 0) - 1), String(Number(c.max || 100) + 1)] : manifest.inputSamples;
            action.samples = [];
            for (const sample of samples) {
              try { await el.fill(sample); await el.press('Tab'); action.samples.push({ sample, result: 'exercised', validity: await el.evaluate(e => e.validity?.valid) }); }
              catch (error) { action.samples.push({ sample, result: 'failed', reason: String(error).slice(0, 250) }); }
            }
          } else if (c.type === 'range' || c.role === 'slider') { await el.focus(); await el.press('ArrowRight'); }
          else if (c.type === 'color') await el.fill('#ff0033');
          else await el.click();
          await page.waitForTimeout(120);
          action.result = 'exercised';
          if (before === await page.locator('body').innerText() && c.tag === 'BUTTON') await finding('no-visible-effect', selector, c.name || c.html, { caveat: 'Text-only heuristic; visual, mock backend, or delayed effect may be intentional.' });
          // Capture transient mock errors before the next control clears them.
          await scan();
        } catch (error) { action.result = 'failed'; action.reason = String(error).slice(0, 500); }
      }
      const remaining = (await controls()).filter(c => !visited.has(identity(c)));
      session.inventories ??= [];
      session.inventories.push({ area, attempted: count, budget, remaining: remaining.map(({ selector, tag, type, role, name, disabled }) => ({ selector, tag, type, role, name, disabled })) });
      if (count === budget && remaining.length) session.omissions.push({ area, reason: 'interaction budget reached', budget, remainingVisible: remaining.length });
    }
    await page.goto(base); await page.waitForTimeout(1000);
    await scan(true);
    // Wizard uses an ordered step list; advancing its last enabled button mirrors screenshot capture.
    for (let i = 0; i < 12 && await page.locator('ol[aria-label]').count(); i++) {
      await crawl(8);
      if (!(await page.locator('ol[aria-label]').count())) break;
      const buttons = page.locator('ol[aria-label]').first().locator('xpath=ancestor::*[.//button][last()]').locator('button:not([disabled])');
      if (!(await buttons.count())) { session.omissions.push({ area, reason: 'wizard has no enabled advance control' }); break; }
      steps.push(`Advance onboarding step ${i + 1}`);
      session.actions.push({ area, control: { name: await buttons.last().innerText() }, result: 'advance-attempt' });
      await buttons.last().click().catch(e => session.omissions.push({ area, reason: String(e) }));
      await page.waitForTimeout(200);
    }
    session.onboardingRemaining = await page.locator('ol[aria-label]').count();
    await writeFile(join(out, 'coverage.json'), JSON.stringify(coverage, null, 2));
    for (const [index, view] of manifest.views.entries()) {
      area = view.id; steps = [`Open fresh ${locale} demo`, 'Complete onboarding', `Open navigation view ${view.id}`];
      try {
        // Reset mock state between views so a deliberately opened modal cannot mask later navigation.
        await page.goto(base); await page.waitForTimeout(500);
        for (let i = 0; i < 12 && await page.locator('ol[aria-label]').count(); i++) {
          const advance = page.locator('ol[aria-label]').first().locator('xpath=ancestor::*[.//button][last()]').locator('button:not([disabled])').last();
          await advance.click(); await page.waitForTimeout(150);
        }
        if (await page.locator('ol[aria-label]').count()) throw new Error('Onboarding overlay remains; view was not covered');
        await page.keyboard.press('Escape');
        const nav = page.locator('nav[aria-label] button').nth(index + 1);
        const navName = (await nav.innerText()).trim();
        await nav.click();
        await page.waitForTimeout(350);
        const headings = await page.locator('main h1, main h2').allTextContents();
        const active = await nav.getAttribute('class');
        session.viewEvidence ??= [];
        session.viewEvidence.push({ view: view.id, navName, headings, activeClass: active, onboardingRemaining: await page.locator('ol[aria-label]').count() });
        if (!headings.some(h => h.trim().toLowerCase().includes(navName.toLowerCase()))) session.omissions.push({ area, reason: 'Heading differs from navigation label; inspect view evidence', navName, headings });
        session.views.push(view.id);
        await scan(true);
        await crawl(manifest.interactionBudgetPerView ?? Math.floor(manifest.interactionBudgetPerSession / manifest.views.length));
        await scan();
        await writeFile(join(out, 'coverage.json'), JSON.stringify(coverage, null, 2));
      } catch (e) { session.omissions.push({ area, reason: String(e).slice(0, 500) }); }
    }
    for (const event of pending) { area = event.area; steps = event.steps; await finding(event.event, 'runtime', event.message); }
    await ctx.close();
    await browser.close();
    await writeFile(join(out, 'coverage.json'), JSON.stringify(coverage, null, 2));
    console.log(`${locale} ${viewport.name}: ${session.views.length} views, ${session.actions.length} actions, ${session.omissions.length} omissions`);
  }
} finally {
  coverage.finished = new Date().toISOString();
  await writeFile(join(out, 'coverage.json'), JSON.stringify(coverage, null, 2));
  await browser?.close();
  await new Promise(r => server.close(r));
}
console.log(`Findings: ${coverage.findings}; artifacts: ${out}`);
