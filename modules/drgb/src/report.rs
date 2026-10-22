// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn v4(colors: &[Rgb], channels: usize, leds: usize) -> Vec<Vec<u8>> {
    let mut data = vec![0; 72 + colors.len() * 3];
    for n in 0..channels {
        data[n * 2] = (leds >> 8) as u8;
        data[n * 2 + 1] = leds as u8;
    }
    for (n, c) in colors.iter().enumerate() {
        let q = 72 + n * 3;
        data[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    let mut out = Vec::new();
    let mut off = 0;
    let packets = if colors.len() <= 340 {
        1
    } else {
        1 + (colors.len() - 340).div_ceil(340)
    };
    for i in 0..packets {
        let mut p = vec![0; 1025];
        p[1] = (100 + i) as u8;
        p[2] = (99 + packets) as u8;
        p[3] = if colors.len() > 255 { 1 } else { 0 };
        p[4] = (colors.len().min(340)) as u8;
        let n = (data.len() - off).min(1020);
        p[5..5 + n].copy_from_slice(&data[off..off + n]);
        off += n;
        out.push(p);
    }
    out
}
pub fn v3(colors: &[Rgb], channels: usize, leds: usize, marker: u8) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut z = vec![0; 64];
    z[0] = marker;
    z[1] = 0xbb;
    for n in 0..channels {
        z[n * 2 + 2] = (leds >> 8) as u8;
        z[n * 2 + 3] = leds as u8;
    }
    out.push(z);
    let bytes: Vec<u8> = colors.iter().flat_map(|c| [c.r, c.g, c.b]).collect();
    for (i, ch) in bytes.chunks(63).enumerate() {
        let mut p = vec![0; 65];
        p[1] = if i + 1 == bytes.len().div_ceil(63) {
            marker + if marker == 0x46 { 0x5c } else { 0x64 } + i as u8
        } else {
            marker + i as u8
        };
        p[2..2 + ch.len()].copy_from_slice(ch);
        out.push(p);
    }
    out
}
pub fn v2(colors: &[Rgb], channels: usize, leds: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (z, chunk) in colors.chunks(leds).take(channels).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|c| [c.r, c.g, c.b]).collect();
        let count = bytes.len().div_ceil(60);
        for (i, ch) in bytes.chunks(60).enumerate() {
            let mut p = vec![0; 65];
            p[1] = (i + 1) as u8;
            p[2] = count as u8;
            p[3] = z as u8;
            p[4] = 0xbb;
            p[4..4 + ch.len().min(60)].copy_from_slice(&ch[..ch.len().min(60)]);
            out.push(p);
        }
    }
    out
}
