//! Drive temperatures without administrator rights or LHM.
//!
//! Windows: `\\.\PhysicalDriveN` is opened with *no* access rights (allowed
//! for every user), then `IOCTL_STORAGE_QUERY_PROPERTY` asks for
//! `StorageDeviceProperty` (model) and `StorageDeviceTemperatureProperty`
//! (current temperature, NVMe and most SATA drives). Nothing is written. The
//! call can block inside a storage driver; callers must run it off the fan
//! path with a timeout (`freergb-daemon` `storage_temps.rs`).
#![allow(unsafe_code)]

/// One drive temperature.
#[derive(Clone, Debug, PartialEq)]
pub struct DriveTemperature {
    /// `PhysicalDriveN` index.
    pub index: u32,
    pub model: String,
    pub celsius: f64,
}

/// Highest `PhysicalDriveN` index probed.
pub const MAX_DRIVES: u32 = 16;

/// Parses a `STORAGE_TEMPERATURE_DATA_DESCRIPTOR`: the first plausible
/// temperature entry (index 0 is the composite drive temperature).
pub fn parse_temperature(buffer: &[u8]) -> Option<f64> {
    const HEADER: usize = 24;
    const INFO: usize = 16;
    let count = u16::from_le_bytes(buffer.get(12..14)?.try_into().ok()?) as usize;
    (0..count.min(8)).find_map(|i| {
        let at = HEADER + i * INFO;
        let raw = i16::from_le_bytes(buffer.get(at + 2..at + 4)?.try_into().ok()?);
        (-40..=150).contains(&raw).then_some(f64::from(raw))
    })
}

/// Parses vendor + product strings of a `STORAGE_DEVICE_DESCRIPTOR`.
pub fn parse_model(buffer: &[u8]) -> String {
    let text = |offset_at: usize| -> Option<String> {
        let offset =
            u32::from_le_bytes(buffer.get(offset_at..offset_at + 4)?.try_into().ok()?) as usize;
        if offset == 0 || offset >= buffer.len() {
            return None;
        }
        let bytes: Vec<u8> = buffer[offset..]
            .iter()
            .take_while(|&&b| b != 0)
            .copied()
            .collect();
        let s = String::from_utf8_lossy(&bytes).trim().to_owned();
        (!s.is_empty()).then_some(s)
    };
    match (text(12), text(16)) {
        (Some(v), Some(p)) if !p.starts_with(&v) => format!("{v} {p}"),
        (_, Some(p)) => p,
        (Some(v), None) => v,
        _ => "Drive".into(),
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::c_void;
    type Handle = *mut c_void;
    const INVALID: Handle = -1isize as Handle;
    const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
    const SHARE_READ_WRITE: u32 = 0x1 | 0x2;
    const OPEN_EXISTING: u32 = 3;
    const DEVICE_PROPERTY: u32 = 0;
    const TEMPERATURE_PROPERTY: u32 = 52;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            security: *mut c_void,
            disposition: u32,
            flags: u32,
            template: Handle,
        ) -> Handle;
        fn DeviceIoControl(
            device: Handle,
            code: u32,
            input: *const c_void,
            input_size: u32,
            output: *mut c_void,
            output_size: u32,
            returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }

    fn query(device: Handle, property: u32) -> Option<Vec<u8>> {
        // STORAGE_PROPERTY_QUERY { PropertyId, QueryType = PropertyStandardQuery, AdditionalParameters[1] }
        let mut query = [0u8; 12];
        query[..4].copy_from_slice(&property.to_le_bytes());
        let mut out = vec![0u8; 1024];
        let mut returned = 0u32;
        // SAFETY: valid handle; buffers sized as passed.
        let ok = unsafe {
            DeviceIoControl(
                device,
                IOCTL_STORAGE_QUERY_PROPERTY,
                query.as_ptr().cast(),
                query.len() as u32,
                out.as_mut_ptr().cast(),
                out.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        (ok != 0).then(|| {
            out.truncate(returned as usize);
            out
        })
    }

    pub fn read_all() -> Vec<DriveTemperature> {
        let mut out = Vec::new();
        for index in 0..MAX_DRIVES {
            let name: Vec<u16> = format!(r"\\.\PhysicalDrive{index}")
                .encode_utf16()
                .chain([0])
                .collect();
            // SAFETY: NUL-terminated path; zero access rights (query only).
            let device = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    0,
                    SHARE_READ_WRITE,
                    std::ptr::null_mut(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if device == INVALID || device.is_null() {
                continue;
            }
            let celsius = query(device, TEMPERATURE_PROPERTY)
                .as_deref()
                .and_then(parse_temperature);
            let model = query(device, DEVICE_PROPERTY)
                .map(|b| parse_model(&b))
                .unwrap_or_else(|| "Drive".into());
            // SAFETY: handle from CreateFileW, closed once.
            unsafe { CloseHandle(device) };
            if let Some(celsius) = celsius {
                out.push(DriveTemperature {
                    index,
                    model,
                    celsius,
                });
            }
        }
        out
    }
}

/// Reads every drive temperature this user can query (Windows only; Linux
/// drive temperatures come from hwmon `nvme`/`drivetemp`).
pub fn read_all() -> Vec<DriveTemperature> {
    #[cfg(windows)]
    {
        imp::read_all()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(entries: &[i16]) -> Vec<u8> {
        let mut b = vec![0u8; 24 + 16 * entries.len()];
        b[12..14].copy_from_slice(&(entries.len() as u16).to_le_bytes());
        for (i, t) in entries.iter().enumerate() {
            b[24 + i * 16 + 2..24 + i * 16 + 4].copy_from_slice(&t.to_le_bytes());
        }
        b
    }

    #[test]
    fn temperature_descriptor_is_parsed_and_implausible_values_skipped() {
        assert_eq!(parse_temperature(&descriptor(&[41])), Some(41.0));
        assert_eq!(parse_temperature(&descriptor(&[-273, 38])), Some(38.0));
        assert_eq!(parse_temperature(&descriptor(&[])), None);
        assert_eq!(parse_temperature(&[0u8; 8]), None, "short buffer");
    }

    #[test]
    fn model_combines_vendor_and_product() {
        let mut b = vec![0u8; 64];
        b[12..16].copy_from_slice(&40u32.to_le_bytes());
        b[16..20].copy_from_slice(&48u32.to_le_bytes());
        b[40..44].copy_from_slice(b"WDC\0");
        b[48..56].copy_from_slice(b"SN850X\0\0");
        assert_eq!(parse_model(&b), "WDC SN850X");
        b[12..16].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(parse_model(&b), "SN850X");
        assert_eq!(parse_model(&[0u8; 4]), "Drive");
    }
}
