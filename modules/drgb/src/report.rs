// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const REPORT_LEN: usize = 65;
pub fn frames(version: u8, counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    match version {
        2 => frames_v2(counts, colors),
        3 => frames_v3(counts, colors),
        _ => frames_v1(counts, colors),
    }
}
pub fn frames_v1(counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut data = Vec::with_capacity(colors.len() * 3);
    for c in colors {
        data.push(c.r);
        data.push(c.g);
        data.push(c.b);
    }
    let total = data.len() / 3;
    let mut out = Vec::new();
    let mut header = vec![0u8; REPORT_LEN];
    header[1] = 0x46;
    header[2] = 0xBB;
    let mut offset = 3usize;
    for &count in counts {
        if offset + 1 >= REPORT_LEN {
            break;
        }
        header[offset] = ((count >> 8) & 0xFF) as u8;
        header[offset + 1] = (count & 0xFF) as u8;
        offset += 2;
    }
    out.push(header);
    if total == 0 {
        return out;
    }
    let packets = total.div_ceil(21);
    for i in 0..packets {
        let is_last = i + 1 == packets;
        let marker = if is_last {
            0xA3_u16 + i as u16
        } else {
            0x47_u16 + i as u16
        } as u8;
        let mut p = vec![0u8; REPORT_LEN];
        p[1] = marker;
        let start = i * 63;
        let end = (start + 63).min(data.len());
        if end > start {
            p[2..2 + (end - start)].copy_from_slice(&data[start..end]);
        }
        out.push(p);
    }
    out
}
pub fn frames_v2(counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut consumed = 0usize;
    for (z, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        // Slice this zone's colors from the flattened input.
        let zone_start = consumed;
        let zone_end = consumed + count.min(colors.len().saturating_sub(consumed));
        let zone_colors = if zone_start < colors.len() {
            &colors[zone_start..zone_end]
        } else {
            &[] as &[Rgb]
        };
        let n_packets = count.div_ceil(20);
        for p in 0..n_packets {
            let mut buf = vec![0u8; REPORT_LEN];
            buf[1] = (p + 1) as u8;
            buf[2] = n_packets as u8;
            buf[3] = z as u8;
            buf[4] = 0xBB;
            for i in 0..20usize {
                let led = p * 20 + i;
                if led >= count {
                    break;
                }
                let col = if led < zone_colors.len() {
                    zone_colors[led]
                } else {
                    Rgb { r: 0, g: 0, b: 0 }
                };
                let start = 5 + i * 3;
                if start + 2 < buf.len() {
                    buf[start] = col.r;
                    buf[start + 1] = col.g;
                    buf[start + 2] = col.b;
                }
            }
            out.push(buf);
        }
        consumed += count;
    }
    out
}

pub fn frames_v3(counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
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
        if offset + 1 >= REPORT_LEN {
            break;
        }
        header[offset] = ((count >> 8) & 0xFF) as u8;
        header[offset + 1] = (count & 0xFF) as u8;
        offset += 2;
    }
    out.push(header);
    if total == 0 {
        return out;
    }
    let packets = total.div_ceil(21);
    for i in 0..packets {
        let is_last = i + 1 == packets;
        let marker = if is_last {
            0xC8_u16 + i as u16
        } else {
            0x64_u16 + i as u16
        } as u8;
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
