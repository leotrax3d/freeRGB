// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

pub fn high_speed(channel: usize, colors: &[Rgb], flag: u8) -> Vec<u8> {
    let mut p = vec![0; 1025];
    p[1] = channel as u8;
    p[2] = flag;
    p[3] = (colors.len() / 256) as u8;
    p[4] = (colors.len() % 256) as u8;
    for (i, c) in colors.iter().enumerate() {
        let q = 5 + i * 3;
        if q + 2 >= p.len() {
            break;
        }
        p[q..q + 3].copy_from_slice(&[c.g, c.r, c.b]);
    }
    p
}

pub fn fixed(pid: u16, channel: usize, packet: usize, colors: &[Rgb]) -> Vec<u8> {
    let interval = match pid {
        0x1616 => 2,
        0x1f01 | 0x2a08 | 0x2c08 => 6,
        0x1f11 | 0x2a01 => 30,
        _ => 25,
    };
    let swap = matches!(pid, 0x1f01 | 0x1f11 | 0x2a08 | 0x2c08);
    let mut p = vec![0; 65];
    p[1] = (packet + channel * interval) as u8;
    for (i, c) in colors.iter().take(21).enumerate() {
        let q = 2 + i * 3;
        if swap {
            p[q..q + 3].copy_from_slice(&[c.g, c.r, c.b]);
        } else {
            p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
        }
    }
    p
}

pub fn update() -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 0xff;
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn high_speed_is_byte_exact() {
        let p = high_speed(2, &[Rgb { r: 1, g: 2, b: 3 }], 0);
        assert_eq!(&p[..8], &[0, 2, 0, 0, 1, 2, 1, 3]);
    }
    #[test]
    fn fixed_packet_is_byte_exact() {
        let p = fixed(0x1f01, 3, 1, &[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(&p[..6], &[0, 19, 2, 1, 3, 0]);
    }
    #[test]
    fn update_is_byte_exact() {
        assert_eq!(&update()[..3], &[0, 0xff, 0]);
    }
}
