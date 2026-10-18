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
    pub device_name: &'static str,
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
        pid: 0x1BFD,
        usage_page: 0xFF42,
        usage: 0x0001,
        adapter: "corsair-bragi",
        name: "CORSAIR K70 CORE RGB",
        level: SupportLevel::Experimental,
    },
    Entry {
        vid: 0x1B1C,
        pid: 0x0C3F,
        usage_page: 0xFF42,
        usage: 0x0001,
        adapter: "corsair-icue-link",
        name: "CORSAIR iCUE LINK System Hub",
        level: SupportLevel::Identified,
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
    Entry {
        vid: 0x0B05,
        pid: 0x19AF,
        usage_page: 0xFF72,
        usage: 0x00A1,
        adapter: "asus-aura-usb",
        name: "ASUS AURA LED Controller",
        level: SupportLevel::Identified,
    },
];

/// Looks up the adapter for one HID interface.
pub fn match_interface(vid: u16, pid: u16, usage_page: u16, usage: u16) -> Option<AdapterMatch> {
    ENTRIES
        .iter()
        .find(|e| e.vid == vid && e.pid == pid && e.usage_page == usage_page && e.usage == usage)
        .map(|e| AdapterMatch {
            adapter: e.adapter,
            device_name: e.name,
            level: e.level,
        })
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
        assert_eq!(m.level, SupportLevel::Experimental);
        // The keyboard's boot interface must not be matched.
        assert!(match_interface(0x1B1C, 0x1BFD, 0x0001, 0x0006).is_none());
        assert!(match_interface(0x1B1C, 0x1BFD, 0xFF42, 0x0002).is_none());
    }

    #[test]
    fn unknown_is_none() {
        assert!(match_interface(0x1234, 0x5678, 0xFF00, 1).is_none());
        assert_eq!(vendor_name(0x1B1C), Some("Corsair"));
    }
}
