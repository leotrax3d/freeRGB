// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs"; import path from "node:path";
const [, , root="openrgb", commit="unknown"]=process.argv; const family=path.join(root,"Controllers","CoolerMasterController"); const files=[];
function walk(d){for(const e of fs.readdirSync(d,{withFileTypes:true})){const f=path.join(d,e.name);if(e.isDirectory())walk(f);else if(/\.(cpp|h)$/.test(e.name))files.push(f);}}
walk(family); const source=files.map(f=>fs.readFileSync(f,"utf8")).join("\n"); const constants=new Map();
for(const m of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm))constants.set(m[1],m[2]);
const detectors=[]; const re=/REGISTER_HID_DETECTOR(?:_[A-Z]+)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*COOLERMASTER_VID\s*,\s*([^,\s)]+)/g;
for(const m of source.matchAll(re)){const pid=constants.get(m[3])??(/^0x/i.test(m[3])?m[3]:undefined);if(pid)detectors.push({name:m[1],handler:m[2],vid:"0x2516",pid});}
if(detectors.length!==38)throw new Error(`expected 38 detectors, found ${detectors.length}`);
const out=["// SPDX-License-Identifier: GPL-2.0-or-later",`// Generated from OpenRGB ${commit}.`,"use super::table::Detector;","pub static DETECTORS: &[Detector] = &[",...detectors.map(d=>`    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: ${d.vid}, pid: ${d.pid} },`),"];",""]; fs.mkdirSync("modules/coolermaster/src",{recursive:true}); fs.writeFileSync("modules/coolermaster/src/devices_table.rs",out.join("\n")); console.log(`generated ${detectors.length} Cooler Master HID detectors`);
