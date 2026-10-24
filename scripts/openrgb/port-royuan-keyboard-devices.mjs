// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";
const [, , root="openrgb", commit="unknown"] = process.argv;
const dir = path.join(root, "Controllers", "RoyuanKeyboardController");
const files=[]; const walk=d=>{for(const e of fs.readdirSync(d,{withFileTypes:true})){const p=path.join(d,e.name);if(e.isDirectory())walk(p);else if(/\.(cpp|h)$/.test(e.name))files.push(p);}}; walk(dir);
const source=files.map(f=>fs.readFileSync(f,"utf8")).join("\n");
const defs=new Map([...source.matchAll(/^\s*#define\s+([A-Za-z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)].map(m=>[m[1],m[2]]));
const rows=[]; for(const m of source.matchAll(/REGISTER_HID_DETECTOR[^\(]*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)/g)){if(defs.has(m[3])&&defs.has(m[4])){const n=m[1]; rows.push({name:n,vid:defs.get(m[3]),pid:defs.get(m[4]),protocol:n.includes("GK68HE")?"Skyloong":n.includes("Akko")?"Akko":"Legacy",leds:n.includes("GK68HE")?67:1});}}
if(rows.length!==5) throw new Error(`expected 5 detectors, found ${rows.length}`);
const out=["// SPDX-License-Identifier: GPL-2.0-or-later",`// Generated from OpenRGB ${commit}.`,`use super::table::{Detector,Device,Protocol};`,`pub static DETECTORS:&[Detector]= &[`,...rows.map(d=>`Detector{name:${JSON.stringify(d.name)},vid:${d.vid},pid:${d.pid}},`),`];`,`pub static DEVICES:&[Device]= &[`,...rows.map(d=>`Device{vid:${d.vid},pid:${d.pid},protocol:Protocol::${d.protocol},leds:${d.leds}},`),`];`,""];
fs.mkdirSync("modules/royuan-keyboard/src",{recursive:true}); fs.writeFileSync("modules/royuan-keyboard/src/devices_table.rs",out.join("\n")); console.log(`generated ${rows.length} Royuan detectors`);
