// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

pub fn offset(pid: u16) -> u8 {
    if pid == 0x1919 {
        0
    } else {
        16
    }
}

pub fn needs_begin(pid: u16) -> bool {
    pid == 0x19B9 || pid == 0x1968
}

pub fn begin(pid: u16) -> Vec<Vec<u8>> {
    if !needs_begin(pid) {
        return Vec::new();
    }
    let mut p1 = vec![0u8; 8];
    p1[0] = 0x03;
    p1[1] = 0x02;
    p1[2] = 0xa1;
    p1[3] = 0x80;
    p1[4] = 0x20;
    let mut p2 = p1.clone();
    p2[4] = 0x30;
    vec![p1, p2]
}

pub fn update_led(pid: u16, led: u8, c: Rgb) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    let base = offset(pid) + led * 3;
    let mut r = vec![0u8; 8];
    r[0] = 0x03;
    r[1] = 0x02;
    r[2] = 0xa1;
    r[3] = 0x80;
    r[4] = base;
    r[5] = c.r;
    v.push(r);
    let mut r = vec![0u8; 8];
    r[0] = 0x03;
    r[1] = 0x02;
    r[2] = 0xa1;
    r[3] = 0x80;
    r[4] = base + 1;
    r[5] = c.b;
    v.push(r);
    let mut r = vec![0u8; 8];
    r[0] = 0x03;
    r[1] = 0x02;
    r[2] = 0xa1;
    r[3] = 0x80;
    r[4] = base + 2;
    r[5] = c.g;
    v.push(r);
    v
}

pub fn apply() -> Vec<u8> {
    let mut p = vec![0u8; 8];
    p[0] = 0x03;
    p[1] = 0x02;
    p[2] = 0xa1;
    p[3] = 0x80;
    p[4] = 0xa0;
    p[5] = 0x01;
    p
}

pub fn frame(colors: &[Rgb], pid: u16) -> Vec<Vec<u8>> {
    let mut v = begin(pid);
    for (i, c) in colors.iter().take(3).enumerate() {
        v.extend(update_led(pid, i as u8, *c));
    }
    v.push(apply());
    v
}
