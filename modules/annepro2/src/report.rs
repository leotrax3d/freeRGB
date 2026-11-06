// SPDX-License-Identifier: GPL-2.0-or-later
/// upstream: AnnePro2Controller.cpp SendDirect (annemone reverse engineering).
/// frame_data is the real 70-LED RGB stream (210 bytes); each packet carries up
/// to 51 payload bytes (max_command_length 55 - real_command_info_length 4).
pub fn direct(raw: &[u8]) -> Vec<Vec<u8>> {
    const MAX_MESSAGE: usize = 51;
    let frame_count = raw.len();
    let count = frame_count / MAX_MESSAGE + 1;
    let mut out = Vec::new();
    for pidx in 0..count {
        let start = pidx * MAX_MESSAGE;
        let end = (start + MAX_MESSAGE).min(frame_count);
        let part = &raw[start..end];
        let mut p = vec![0u8; 64];
        p[0] = 0;
        p[1] = 0x7b;
        p[2] = 0x10;
        p[3] = 0x41;
        p[4] = ((count << 4) + pidx) as u8;
        p[5] = (part.len() + 4) as u8;
        p[6] = 0;
        p[7] = 0;
        p[8] = 0x7d;
        p[9] = 32;
        p[10] = 3;
        p[11] = 0xff;
        p[12] = 2;
        p[13..13 + part.len()].copy_from_slice(part);
        out.push(p)
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        // 210 real bytes -> 5 packets, 51 payload bytes each, last 6 bytes.
        let raw = vec![1u8; 210];
        let p = direct(&raw);
        assert_eq!(p.len(), 5);
        assert_eq!(
            &p[0][0..14],
            &[0, 0x7b, 0x10, 0x41, 0x50, 55, 0, 0, 0x7d, 32, 3, 0xff, 2, 1]
        );
        assert_eq!(p[0].len(), 64);
        assert_eq!(p[4][4], 0x54);
        assert_eq!(p[4][5], 10);
        assert_eq!(&p[4][13..19], &[1; 6]);
        assert!(p[4][19..].iter().all(|b| *b == 0));
    }
}
