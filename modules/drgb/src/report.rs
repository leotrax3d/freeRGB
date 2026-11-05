// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const REPORT_LEN: usize = 65;
pub const V4_REPORT_LEN: usize = 1025;
pub const V4_ONE_PACKAGE_SIZE: usize = 316;
pub const V4_PACKAGE_SIZE: usize = 340;
pub const V4_HEADER_SIZE: usize = 72;
pub fn frames(version: u8, counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    match version {
        2 => frames_v2(counts, colors),
        3 => frames_v3(counts, colors),
        4 => frames_v4(counts, colors),
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
    let mut header = [0u8; REPORT_LEN];
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
    out.push(header.to_vec());
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
        let endb = (start + 63).min(data.len());
        if endb > start {
            p[2..2 + (endb - start)].copy_from_slice(&data[start..endb]);
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
        let zone_colors = if consumed < colors.len() {
            let end = (consumed + count).min(colors.len());
            &colors[consumed..end]
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
    let mut header = [0u8; REPORT_LEN];
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
pub fn frames_v4(counts: &[usize], colors: &[Rgb]) -> Vec<Vec<u8>> {
    let zones_count = counts;
    let mut data = vec![0u8; colors.len() * 3 + V4_HEADER_SIZE];
    for (z, &count) in zones_count.iter().enumerate() {
        if z * 2 + 1 < data.len() {
            data[z * 2 + 1] = (count & 0xFF) as u8;
        }
    }
    for (i, c) in colors.iter().enumerate() {
        let off = V4_HEADER_SIZE + i * 3;
        if off + 2 < data.len() {
            data[off] = c.r;
            data[off + 1] = c.g;
            data[off + 2] = c.b;
        }
    }
    let total = colors.len();
    let n_packets = if total <= V4_ONE_PACKAGE_SIZE {
        1usize
    } else {
        1 + ((total - V4_ONE_PACKAGE_SIZE) / V4_PACKAGE_SIZE)
            + (((total - V4_ONE_PACKAGE_SIZE) % V4_PACKAGE_SIZE) > 0) as usize
    };
    let mut out = Vec::new();
    // header color-less report is raw 1025? SendPacketFS uses ArrayData 65; raw branch writes 64 bytes. In freeRGB send separately 65-byte header.
    // no separate header report for V4; it is embedded in the flat data buffer above

    for i in 0..n_packets {
        let count_for_packet = if i == 0 {
            total.min(V4_ONE_PACKAGE_SIZE)
        } else {
            let consumed = V4_ONE_PACKAGE_SIZE + (i - 1) * V4_PACKAGE_SIZE;
            (total - consumed).min(V4_PACKAGE_SIZE)
        };
        let marker = (100 + i) as u8;
        let mut buf = vec![0u8; V4_REPORT_LEN];
        buf[1] = marker;
        buf[2] = (n_packets + 99) as u8;
        buf[3] = (count_for_packet >> 8) as u8;
        buf[4] = (count_for_packet & 0xFF) as u8;
        let start = i * 1020;
        let endb = (start + 1020).min(data.len());
        if endb > start {
            let bytes = endb - start;
            buf[5..5 + bytes].copy_from_slice(&data[start..endb]);
        }
        out.push(buf);
    }
    out
}
