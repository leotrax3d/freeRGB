// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";
const [, , root="openrgb", commit="unknown"] = process.argv;
const dir=path.join(root,"Controllers","LianLiController"), files=[];
const walk=x=>{for(const e of fs.readdirSync(x,{withFileTypes:true})){const p=path.join(x,e.name);if(e.isDirectory())walk(p);else if(/\.(cpp|h)$/.test(e.name))files.push(p);}}; walk(dir);
const s=files.map(x=>fs.readFileSync(x,"utf8")).join("\n"), vals=new Map();
for(const m of s.matchAll(/^\s*#define\s+([A-Za-z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm))vals.set(m[1],m[2]);
const rows=[]; for(const m of s.matchAll(/REGISTER_HID_DETECTOR[^\(]*\(\s*"([^"]+)"\s*,[^,]+,\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)/g))if(vals.has(m[2])&&vals.has(m[3]))rows.push({name:m[1],vid:vals.get(m[2]),pid:vals.get(m[3])});
const ds=Array.from(new Map(rows.map(d=>[d.pid,d])).values()); if(ds.length!==9)throw new Error(`expected 9 HID detectors, found ${ds.length}`);
const leds=d=>d.pid==="0xA200"?60:d.pid.startsWith("0x737")?16:120;
const out=["// SPDX-License-Identifier: GPL-2.0-or-later",`// Generated from OpenRGB ${commit}.`,"use super::table::{Detector,Device};","pub static DETECTORS:&[Detector]= &[",...ds.map(d=>`Detector{name:${JSON.stringify(d.name)},vid:${d.vid},pid:${d.pid},leds:${leds(d)}},`),"];","pub static DEVICES:&[Device]= &[",...ds.map(d=>`Device{vid:${d.vid},pid:${d.pid},leds:${leds(d)}},`),"];",""];
fs.mkdirSync("modules/lianli/src",{recursive:true}); fs.writeFileSync("modules/lianli/src/devices_table.rs",out.join("\n")); console.log(`generated ${ds.length} Lian Li HID detectors`);
