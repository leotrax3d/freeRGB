//! Minimal HID report descriptor parser: computes the largest input, output
//! and feature report sizes. Only short items are interpreted; long items are
//! skipped. Reference: USB HID 1.11, section 6.2.2.

/// Largest report payload in bytes, excluding the report-ID byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReportSizes {
    pub input: usize,
    pub output: usize,
    pub feature: usize,
    pub uses_report_ids: bool,
}

impl ReportSizes {
    /// Buffer length for hidapi writes: hidapi always expects a leading
    /// report-ID byte, `0x00` when the device uses no report IDs.
    pub fn output_with_id(&self) -> usize {
        self.output + 1
    }

    /// Read buffer length; one spare byte covers platforms that prepend the ID.
    pub fn input_with_id(&self) -> usize {
        self.input + 1
    }
}

#[derive(Clone, Copy, Default)]
struct Globals {
    report_size: u32,
    report_count: u32,
    report_id: u8,
}

pub fn parse_report_sizes(desc: &[u8]) -> Result<ReportSizes, String> {
    use std::collections::HashMap;

    let mut g = Globals::default();
    let mut stack: Vec<Globals> = Vec::new();
    // (kind, report id) -> bits
    let mut bits: HashMap<(u8, u8), u32> = HashMap::new();
    let mut uses_ids = false;
    let mut i = 0;

    while i < desc.len() {
        let prefix = desc[i];
        if prefix == 0xFE {
            // Long item: [0xFE, size, tag, data...]
            let size = *desc.get(i + 1).ok_or("truncated long item")? as usize;
            i += 3 + size;
            continue;
        }
        let size = match prefix & 0x03 {
            3 => 4,
            n => n as usize,
        };
        let data = desc
            .get(i + 1..i + 1 + size)
            .ok_or_else(|| format!("truncated item at offset {i}"))?;
        let value = data
            .iter()
            .rev()
            .fold(0u32, |acc, &b| (acc << 8) | u32::from(b));
        let tag_type = prefix & 0xFC;

        match tag_type {
            // Main items: Input / Output / Feature
            0x80 | 0x90 | 0xB0 => {
                *bits.entry((tag_type, g.report_id)).or_default() += g.report_size * g.report_count;
            }
            0x74 => g.report_size = value,
            0x94 => g.report_count = value,
            0x84 => {
                g.report_id = value as u8;
                uses_ids = true;
            }
            0xA4 => stack.push(g),
            0xB4 => g = stack.pop().ok_or("pop without push")?,
            _ => {}
        }
        i += 1 + size;
    }

    let max = |kind: u8| {
        bits.iter()
            .filter(|((k, _), _)| *k == kind)
            .map(|(_, &b)| (b as usize).div_ceil(8))
            .max()
            .unwrap_or(0)
    };
    let sizes = ReportSizes {
        input: max(0x80),
        output: max(0x90),
        feature: max(0xB0),
        uses_report_ids: uses_ids,
    };
    if sizes.input == 0 && sizes.output == 0 && sizes.feature == 0 {
        return Err("descriptor declares no reports".into());
    }
    Ok(sizes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Typical vendor-defined 64-byte in/out collection without report IDs.
    const VENDOR_64: &[u8] = &[
        0x06, 0x42, 0xFF, // Usage Page (0xFF42)
        0x09, 0x01, // Usage (1)
        0xA1, 0x01, // Collection (Application)
        0x15, 0x00, // Logical Min 0
        0x26, 0xFF, 0x00, // Logical Max 255
        0x75, 0x08, // Report Size 8
        0x95, 0x40, // Report Count 64
        0x09, 0x01, 0x81, 0x02, // Usage, Input
        0x09, 0x01, 0x91, 0x02, // Usage, Output
        0xC0, // End Collection
    ];

    #[test]
    fn vendor_64_byte() {
        let s = parse_report_sizes(VENDOR_64).unwrap();
        assert_eq!(
            s,
            ReportSizes {
                input: 64,
                output: 64,
                feature: 0,
                uses_report_ids: false
            }
        );
        assert_eq!(s.output_with_id(), 65);
    }

    #[test]
    fn report_ids_take_max_per_id() {
        let d = [
            0x85, 0x01, 0x75, 0x08, 0x95, 0x10, 0x91, 0x02, // id1: 16 bytes out
            0x85, 0x02, 0x95, 0x20, 0x91, 0x02, // id2: 32 bytes out
            0x95, 0x04, 0x81, 0x02, // id2: 4 bytes in
        ];
        let s = parse_report_sizes(&d).unwrap();
        assert_eq!(s.output, 32);
        assert_eq!(s.input, 4);
        assert!(s.uses_report_ids);
    }

    #[test]
    fn push_pop_and_errors() {
        let d = [0x75, 0x08, 0x95, 0x02, 0xA4, 0x95, 0x08, 0xB4, 0x81, 0x02];
        assert_eq!(parse_report_sizes(&d).unwrap().input, 2);
        assert!(parse_report_sizes(&[0xB4]).is_err());
        assert!(parse_report_sizes(&[0x95]).is_err());
        assert!(parse_report_sizes(&[0xC0]).is_err());
    }
}
