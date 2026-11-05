import fs from 'node:fs';
import path from 'node:path';
const p='modules/drgb/src/report.rs';
let s=fs.readFileSync(p,'utf8');
s=s.replace(
`pub fn frames(version: u8, counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    match version {
        2 => frames_v2(counts, colors),
        _ => frames_v1(counts, colors),
    }
}`,
`pub fn frames(version: u8, counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    match version {
        2 => frames_v2(counts, colors),
        3 => frames_v3(counts, colors),
        _ => frames_v1(counts, colors),
    }
}`);
// append frames_v3 after frames_v2 (find end of frames_v2 by marker)
const marker='\npub fn frames_v2(counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {';
const idx=s.lastIndexOf(marker);
// find last closing brace after marker
let end=-1;
let depth=0;
let started=false;
for(let i=idx;i<s.length;i++){
 if(s[i]==='{'){depth++;started=true;}
 else if(s[i]==='}'){depth--; if(started && depth===0){end=i+1;break;}}
}
const fn3=`\n\npub fn frames_v3(counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut data = Vec::with_capacity(colors.len() * 3);
    for c in colors {
        data.push(c.r);
        data.push(c.g);
        data.push(c.b);
    }
    let total = data.len() / 3;
    let mut out = Vec::new();
    let mut header = vec![0u8; REPORT_LEN];
    header[1] = 0x60;
    header[2] = 0xBB;
    let mut offset = 3usize;
    for &count in counts {
        if offset + 1 >= REPORT_LEN { break; }
        header[offset] = ((count >> 8) & 0xFF) as u8;
        header[offset + 1] = (count & 0xFF) as u8;
        offset += 2;
    }
    out.push(header);
    if total == 0 { return out; }
    let packets = total.div_ceil(21);
    for i in 0..packets {
        let is_last = i + 1 == packets;
        let marker = if is_last { 0xC8_u16 + i as u16 } else { 0x64_u16 + i as u16 } as u8;
        let mut p = vec![0u8; REPORT_LEN];
        p[1] = marker;
        let start = i * 63;
        let endb = (start + 63).min(data.len());
        if endb > start {
            p[2..2 + (endb - start)].copy_from_slice(&data[start..endb]);
        }
        out.push(p);
    }
    out
}
`;
s=s.slice(0,end)+fn3+s.slice(end);
fs.writeFileSync(p,s);
console.log('patched');
