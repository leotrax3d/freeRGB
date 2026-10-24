// SPDX-License-Identifier: GPL-2.0-or-later
pub fn checksum(mut p: [u8; 64]) -> [u8; 64] {
    let sum: u16 = p[3..].iter().map(|x| *x as u16).sum();
    p[1] = sum as u8;
    p[2] = (sum >> 8) as u8;
    p
}
pub fn v1_begin() -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 4;
    p[1] = 1;
    p[3] = 1;
    p
}
pub fn v1_end() -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 4;
    p[1] = 2;
    p[3] = 2;
    p
}
pub fn v1_colors(colors: &[u8], offset: usize) -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 4;
    p[3] = 0x11;
    p[4] = colors.len() as u8;
    p[5] = offset as u8;
    p[6] = (offset >> 8) as u8;
    p[8..8 + colors.len()].copy_from_slice(colors);
    checksum(p)
}
pub fn v2(cmd: u8, offset: u16, data: &[u8]) -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 4;
    p[3] = cmd;
    p[4] = data.len() as u8;
    p[5] = offset as u8;
    p[6] = (offset >> 8) as u8;
    p[8..8 + data.len()].copy_from_slice(data);
    checksum(p)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v1_begin_exact() {
        assert_eq!(&v1_begin()[..4], &[4, 1, 0, 1]);
    }
    #[test]
    fn color_checksum_exact() {
        let p = v1_colors(&[1, 2, 3], 0);
        assert_eq!(&p[..12], &[4, 0x1a, 0, 0x11, 3, 0, 0, 0, 1, 2, 3, 0]);
    }
    #[test]
    fn v2_dynamic_exact() {
        let p = v2(0x12, 0, &[1, 2, 3]);
        assert_eq!(&p[..12], &[4, 0x1b, 0, 0x12, 3, 0, 0, 0, 1, 2, 3, 0]);
    }
}
