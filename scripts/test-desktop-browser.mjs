import assert from 'node:assert/strict';
// Run against `npm run dev` only. Install Playwright outside the checkout or
// set PLAYWRIGHT_MODULE to its module URL; CHROMIUM_PATH selects a system browser.
import { mkdir } from 'node:fs/promises';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH || undefined,headless:true,args:['--no-sandbox']});
const page=await browser.newPage({viewport:{width:1440,height:1000}});
const errors=[];page.on('pageerror',error=>errors.push(error.message));
try {
 const base = new URL(process.env.FREERGB_BROWSER_URL || 'http://127.0.0.1:1420/');
 assert.equal(base.hostname, '127.0.0.1', 'Use a local mock development server');
 await page.goto(base.href);
 await page.evaluate(async()=>{
   const {transport}=await import('/src/lib/api/transport.ts');
   if((await transport()).kind !== 'mock')throw Error('Mock transport required');
 });
 await page.getByRole('button',{name:'Get started',exact:true}).click();
 await page.getByRole('button',{name:/Skip — set up lighting only/}).click();
 await page.getByRole('heading',{name:'Set up your devices',exact:true}).waitFor();
 await page.evaluate(async()=>{
   const {transport}=await import('/src/lib/api/transport.ts');
   const t=await transport(); if(t.kind!=='mock')throw Error('Mock transport required');
   const original=t.rpc.bind(t);window.failMethod='modules.setup.complete';
   t.rpc=(method,params)=>method===window.failMethod?Promise.reject(Error('Simulated save failure')):original(method,params);
 });
 await page.getByRole('button',{name:'Skip devices',exact:true}).click();
 await page.getByRole('alert').filter({hasText:'Simulated save failure'}).waitFor();
 assert(await page.getByRole('heading',{name:'Set up your devices',exact:true}).isVisible());
 await page.evaluate(()=>window.failMethod='');
 await page.getByRole('button',{name:'Skip devices',exact:true}).click();
 await page.getByRole('heading',{name:"You're set",exact:true}).waitFor();
 await page.evaluate(()=>window.failMethod='onboarding.complete');
 await page.getByRole('button',{name:'Open freeRGB',exact:true}).click();
 await page.getByRole('alert').filter({hasText:'Simulated save failure'}).waitFor();
 assert(await page.getByRole('dialog',{name:'freeRGB setup',exact:true}).isVisible());
 await mkdir(new URL('../target/quality-audit/', import.meta.url), {recursive:true});
 await page.screenshot({path:new URL('../target/quality-audit/setup-error.png', import.meta.url).pathname,fullPage:true});
 await page.evaluate(()=>window.failMethod='');
 await page.getByRole('button',{name:'Open freeRGB',exact:true}).click();
 await page.getByRole('dialog',{name:'freeRGB setup',exact:true}).waitFor({state:'hidden'});
 console.log('Setup failure/retry: PASS');
 await page.getByRole('button',{name:'Close tour',exact:true}).click();
 for (const [nav, heading] of [['Marketplace','Effect Marketplace'],['Lighting','Lighting'],['Layout','Layout'],['Profiles','Profiles'],['Devices','Devices'],['Cooling','Cooling'],['Monitoring','Monitoring'],['Macros','Macros'],['Settings','Settings']]) {
   await page.getByRole('navigation',{name:'Main',exact:true}).getByRole('button',{name:nav,exact:true}).click();
   await page.getByRole('main').getByRole('heading',{name:heading,exact:true}).waitFor();
 }
 console.log('All nine frontend views: PASS');
 assert.deepEqual(errors,[]);
} finally {await browser.close();}
