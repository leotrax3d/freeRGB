// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const REPORT_LEN: usize = 65;
pub fn frames(zone_counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut data = Vec::with_capacity(colors.len() * 3);
    for c in colors {
        data.push(c.r);
        data.push(c.g);
        data.push(c.b);
    }
    let total = data.len() / 3;
    let mut out = Vec::new();
    // header report
    let mut header = vec![0u8; REPORT_LEN];
    header[1] = 0x46;
    header[2] = 0xBB;
    let mut offset = 3usize;
    for &count in zone_counts.iter() {
        if offset + 1 < REPORT_LEN {
            header[offset] = ((count >> 8) & 0xFF) as u8;
            header[offset + 1] = (count & 0xFF) as u8;
            offset += 2;
        }
        if total > 1800 {
            break;
        }
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
