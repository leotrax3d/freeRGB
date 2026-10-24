//! Matches discovered HID interfaces against known protocol adapters.
//!
//! A match only says which adapter *would* drive the interface and how well
//! the protocol is established. Unknown devices are reported, never written to.

/// How well an adapter's protocol is established for a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SupportLevel {
    /// Known device, protocol not implemented. Read-only diagnostics only.
    Identified,
    /// Implemented and simulated, not yet validated on real hardware.
    Experimental,
    /// Validated on real hardware (see docs/DEVICES.md).
    Validated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterMatch {
    pub adapter: &'static str,
    pub device_name: String,
    pub level: SupportLevel,
}

/// Interface selector: VID, PID, and the vendor usage page/usage that the
/// adapter talks to.
struct Entry {
    vid: u16,
    pid: u16,
    usage_page: u16,
    usage: u16,
    adapter: &'static str,
    name: &'static str,
    level: SupportLevel,
}

const ENTRIES: &[Entry] = &[
    Entry {
        vid: 0x1B1C,
        pid: 0x0C3F,
        usage_page: 0xFF42,
        usage: 0x0001,
        adapter: "corsair-icue-link",
        name: "CORSAIR iCUE LINK System Hub",
        level: SupportLevel::Experimental,
    },
    Entry {
        vid: 0x1B1C,
        pid: 0x0C4E,
        usage_page: 0x000C,
        usage: 0x0001,
        adapter: "corsair-icue-link-lcd",
        name: "CORSAIR iCUE LINK AIO LCD",
        level: SupportLevel::Identified,
    },
    // ASUS Aura motherboard USB controllers (docs/MAINBOARD_RGB.md, OpenRGB
    // reference list). One protocol family; Gen1/Gen2 handled by the adapter.
    aura(0x18F3),
    aura(0x1939),
    aura(0x19AF),
    aura(0x1AA6),
    aura(0x1BED),
    // Gigabyte RGB Fusion 2 USB (ITE IT5702/IT8297), vendor page 0xFF89/0xCC.
    fusion2(0x048D, 0x5702, SupportLevel::Experimental),
    fusion2(0x048D, 0x8297, SupportLevel::Experimental),
    // Same family with more headers/other calibration: reported only.
    fusion2(0x048D, 0x5711, SupportLevel::Identified),
    fusion2(0x048D, 0x8950, SupportLevel::Identified),
    fusion2(0x0414, 0xA100, SupportLevel::Identified),
];

const fn aura(pid: u16) -> Entry {
    Entry {
        vid: 0x0B05,
        pid,
        usage_page: 0xFF72,
        usage: 0x00A1,
        adapter: "asus-aura-usb",
        name: "ASUS Aura motherboard RGB/ARGB",
        level: SupportLevel::Experimental,
    }
}

const fn fusion2(vid: u16, pid: u16, level: SupportLevel) -> Entry {
    Entry {
        vid,
        pid,
        usage_page: 0xFF89,
        usage: 0x00CC,
        adapter: "gigabyte-fusion2-usb",
        name: "Gigabyte RGB Fusion 2 motherboard ARGB",
        level,
    }
}

/// Motherboard RGB controllers that are recognised but not driven (any
/// interface): (vid, first pid, last pid, adapter, name). Never written to.
const IDENTIFIED_RANGES: &[(u16, u16, u16, &str, &str)] = &[
    (
        0x0B05,
        0x1867,
        0x1867,
        "asus-aura-addressable",
        "ASUS Aura addressable controller",
    ),
    (
        0x0B05,
        0x1872,
        0x1872,
        "asus-aura-addressable",
        "ASUS Aura addressable controller",
    ),
    (
        0x0B05,
        0x18A3,
        0x18A3,
        "asus-aura-addressable",
        "ASUS Aura addressable controller",
    ),
    (
        0x0B05,
        0x18A5,
        0x18A5,
        "asus-aura-addressable",
        "ASUS Aura addressable controller",
    ),
    (
        0x1462,
        0x7B00,
        0x7EFF,
        "msi-mystic-light",
        "MSI Mystic Light motherboard RGB",
    ),
    (
        0x1462,
        0x1720,
        0x1720,
        "msi-mystic-light",
        "MSI Mystic Light motherboard RGB",
    ),
    (
        0x26CE,
        0x01A2,
        0x01A2,
        "asrock-polychrome-usb",
        "ASRock Polychrome USB motherboard RGB",
    ),
];

/// Looks up the adapter for one HID interface using the built-in device
/// descriptions and the table of identified-only devices.
pub fn match_interface(vid: u16, pid: u16, usage_page: u16, usage: u16) -> Option<AdapterMatch> {
    match_interface_with(&[], vid, pid, usage_page, usage)
}

/// Like [`match_interface`], additionally consulting `extra` descriptions
/// (user files, always experimental). Built-in descriptions win.
pub fn match_interface_with(
    extra: &[crate::descriptions::DeviceDescription],
    vid: u16,
    pid: u16,
    usage_page: u16,
    usage: u16,
) -> Option<AdapterMatch> {
    let from_desc = crate::descriptions::builtin()
        .iter()
        .chain(extra)
        .find(|d| d.matches(vid, pid, usage_page, usage))
        .map(|d| AdapterMatch {
            adapter: d.adapter,
            device_name: d.name.clone(),
            level: d.support,
        });
    from_desc
        .or_else(|| {
            ENTRIES
                .iter()
                .find(|e| {
                    e.vid == vid && e.pid == pid && e.usage_page == usage_page && e.usage == usage
                })
                .map(|e| AdapterMatch {
                    adapter: e.adapter,
                    device_name: e.name.to_owned(),
                    level: e.level,
                })
        })
        .or_else(|| {
            IDENTIFIED_RANGES
                .iter()
                .find(|(v, first, last, _, _)| *v == vid && (*first..=*last).contains(&pid))
                .map(|(_, _, _, adapter, name)| AdapterMatch {
                    adapter,
                    device_name: (*name).to_owned(),
                    level: SupportLevel::Identified,
                })
        })
}

/// Whether `adapter` drives motherboard RGB headers (the global "manage
/// motherboard RGB" switch and vendor-software warnings apply to it).
pub fn is_mainboard_rgb(adapter: &str) -> bool {
    adapter.starts_with("asus-aura")
        || adapter.starts_with("gigabyte-fusion2")
        || adapter == "msi-mystic-light"
        || adapter == "asrock-polychrome-usb"
}

/// Coarse vendor name for diagnostics.
pub fn vendor_name(vid: u16) -> Option<&'static str> {
    Some(match vid {
        0x1B1C => "Corsair",
        0x0B05 => "ASUS",
        0x1532 => "Razer",
        0x046D => "Logitech",
        0x1038 => "SteelSeries",
        0x0951 | 0x03F0 => "HyperX",
        0x1E71 => "NZXT",
        0x0CF2 => "Lian Li",
        0x1462 => "MSI",
        0x048D | 0x0414 => "Gigabyte",
        0x26CE => "ASRock",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_k70_vendor_interface_only() {
        let m = match_interface(0x1B1C, 0x1BFD, 0xFF42, 0x0001).unwrap();
        assert_eq!(m.adapter, "corsair-bragi");
        assert_eq!(m.level, SupportLevel::Validated);
        assert_eq!(m.device_name, "CORSAIR K70 CORE RGB");
        // The keyboard's boot interface must not be matched.
        assert!(match_interface(0x1B1C, 0x1BFD, 0x0001, 0x0006).is_none());
        assert!(match_interface(0x1B1C, 0x1BFD, 0xFF42, 0x0002).is_none());
    }

    #[test]
    fn unknown_is_none() {
        assert!(match_interface(0x1234, 0x5678, 0xFF00, 1).is_none());
        assert_eq!(vendor_name(0x1B1C), Some("Corsair"));
    }

    #[test]
    fn mainboard_controllers_are_matched_by_family() {
        for pid in [0x18F3, 0x1939, 0x19AF, 0x1AA6, 0x1BED] {
            let m = match_interface(0x0B05, pid, 0xFF72, 0x00A1).unwrap();
            assert_eq!(
                (m.adapter, m.level),
                ("asus-aura-usb", SupportLevel::Experimental)
            );
            assert!(match_interface(0x0B05, pid, 0x0001, 0x0006).is_none());
        }
        for pid in [0x5702, 0x8297] {
            let m = match_interface(0x048D, pid, 0xFF89, 0x00CC).unwrap();
            assert_eq!(m.adapter, "gigabyte-fusion2-usb");
            assert_eq!(m.level, SupportLevel::Experimental);
        }
        let it5711 = match_interface(0x048D, 0x5711, 0xFF89, 0x00CC).unwrap();
        assert_eq!(it5711.level, SupportLevel::Identified);
        for (vid, pid, adapter) in [
            (0x1462, 0x7C56, "msi-mystic-light"),
            (0x26CE, 0x01A2, "asrock-polychrome-usb"),
            (0x0B05, 0x18A5, "asus-aura-addressable"),
        ] {
            let m = match_interface(vid, pid, 0x0001, 0x0000).unwrap();
            assert_eq!((m.adapter, m.level), (adapter, SupportLevel::Identified));
            assert!(is_mainboard_rgb(m.adapter));
        }
        assert!(!is_mainboard_rgb("corsair-bragi"));
    }
}
