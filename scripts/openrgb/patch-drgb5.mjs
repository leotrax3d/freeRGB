import fs from 'node:fs';
const p='modules/drgb/src/report.rs';
let s=fs.readFileSync(p,'utf8');
s=s.replace(
`pub fn frames(version: u8, counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    match version {
        2 => frames_v2(counts, colors),
        3 => frames_v3(counts, colors),
        _ => frames_v1(counts, colors),
    }
}`,
`pub fn frames(version: u8, counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    match version {
        2 => frames_v2(counts, colors),
        3 => frames_v3(counts, colors),
        4 => frames_v4(counts, colors),
        _ => frames_v1(counts, colors),
    }
}`);
const marker='\npub fn frames_v3(counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {';
const idx=s.indexOf(marker);
let end=-1,depth=0,started=false;
for(let i=idx;i<s.length;i++){
 if(s[i]==='{'){depth++;started=true;}
 else if(s[i]==='}'){depth--; if(started&&depth===0){end=i+1;break;}}
}
const fn4=`\n\npub fn frames_v4(counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    let total = colors.len();
    let mut data = vec![0u8; total * 3 + 72];
    for (z, &count) in counts.iter().enumerate() {
        if z * 2 + 1 < data.len() {
            data[z * 2 + 1] = (count & 0xFF) as u8;
        }
    }
    for (i, c) in colors.iter().enumerate() {
        let off = 72 + i * 3;
        if off + 2 < data.len() {
            data[off] = c.r; data[off+1] = c.g; data[off+2] = c.b;
        }
    }
    let n_packets = if total <= 316 { 1usize } else { 1 + ((total - 316) / 340) + (((total - 316) % 340) > 0) as usize };
    let mut out = Vec::new();
    for i in 0..n_packets {
        let count_for_packet = if i == 0 {
            total.min(316)
        } else {
            let consumed = 316 + (i - 1) * 340;
            (total - consumed).min(340)
        };
        let marker = ((100 + i) as u16).min(255) as u8;
        let mut buf = vec![0u8; REPORT_LEN];
        buf[1] = marker;
        buf[2] = (n_packets + 99) as u8;
        buf[3] = (count_for_packet >> 8) as u8;
        buf[4] = (count_for_packet & 0xFF) as u8;
        let start = i * 1020;
        let endb = (start + 1020).min(data.len());
        if endb > start && buf.len() >= 6 {
            let dst_end = (5 + (endb - start)).min(buf.len());
            if dst_end > 5 {
                buf[5..dst_end].copy_from_slice(&data[start..start + (dst_end - 5).min(data.len()-start)]);
            }
        }
        out.push(buf);
    }
    if total == 0 {
        out.push(vec![0u8; REPORT_LEN]);
    }
    out
}`.replace('min(255) as u8', '');
// Insert before V3 function.
s=s.slice(0,end)+fn4+s.slice(end);
fs.writeFileSync(p,s);
console.log('done');
