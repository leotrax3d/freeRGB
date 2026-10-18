//! Pure Bragi packet encoding and response decoding. No I/O.

/// Request target byte for the directly attached device. Wireless children
/// use `TARGET_BASE | index`.
pub const TARGET_BASE: u8 = 0x08;

/// Size of the header before payload in the first `WriteEndpoint` packet:
/// report id, target, command, handle, u32 length.
pub const WRITE_HEADER_LEN: usize = 8;
/// Header before payload in `StreamEndpoint` continuation packets:
/// report id, target, command, handle.
pub const STREAM_HEADER_LEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    SetProperty = 0x01,
    GetProperty = 0x02,
    CloseHandle = 0x05,
    WriteEndpoint = 0x06,
    StreamEndpoint = 0x07,
    ReadEndpoint = 0x08,
    CheckHandle = 0x09,
    OpenEndpoint = 0x0D,
    Ping = 0x12,
}

/// Properties this implementation reads. Values from the SignalRGB plugin's
/// property table; only `Mode` is ever written (see [`Property::is_writable`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Property {
    PollingRate = 0x01,
    /// Persisted to flash according to the plugin's documentation. Never written.
    HwBrightness = 0x02,
    Mode = 0x03,
    VendorId = 0x11,
    ProductId = 0x12,
    Firmware = 0x13,
    BootloaderFirmware = 0x14,
    SubdeviceBitmask = 0x36,
    Layout = 0x41,
    MaxPollingRate = 0x96,
}

impl Property {
    /// Allowlist of properties this crate will ever write. `Mode` toggles
    /// between hardware and software lighting control and is set by SignalRGB on
    /// every start/stop, so its effect is well established and reversible.
    pub fn is_writable(self) -> bool {
        matches!(self, Property::Mode)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    Hardware = 0x01,
    Software = 0x02,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Handle {
    Lighting = 0x00,
    Background = 0x01,
    Auxiliary = 0x02,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Endpoint {
    /// Planar RGB: all red bytes, then green, then blue.
    Lighting = 0x01,
    /// Interleaved RGB with a 2-byte `[0x12, 0x00]` prefix.
    LightingController = 0x22,
}

/// Data-type prefix the plugin prepends to `LightingController` payloads.
pub const LIGHTING_CONTROLLER_PREFIX: [u8; 2] = [0x12, 0x00];

fn base(target: u8, cmd: Command) -> Vec<u8> {
    vec![0x00, target, cmd as u8]
}

pub fn ping(target: u8) -> Vec<u8> {
    base(target, Command::Ping)
}

pub fn get_property(target: u8, prop: Property) -> Vec<u8> {
    let mut p = base(target, Command::GetProperty);
    p.extend_from_slice(&(prop as u16).to_le_bytes());
    p
}

/// Encodes a property write. Returns `None` for properties outside the
/// allowlist so callers cannot accidentally build e.g. a flash write.
pub fn set_property(target: u8, prop: Property, value: u32) -> Option<Vec<u8>> {
    if !prop.is_writable() {
        return None;
    }
    let mut p = base(target, Command::SetProperty);
    p.extend_from_slice(&(prop as u16).to_le_bytes());
    p.extend_from_slice(&value.to_le_bytes()[..3]);
    Some(p)
}

pub fn open_endpoint(target: u8, handle: Handle, endpoint: Endpoint) -> Vec<u8> {
    let mut p = base(target, Command::OpenEndpoint);
    p.extend_from_slice(&[handle as u8, endpoint as u8]);
    p
}

pub fn close_handle(target: u8, handle: Handle) -> Vec<u8> {
    let mut p = base(target, Command::CloseHandle);
    p.extend_from_slice(&[0x01, handle as u8]);
    p
}

pub fn check_handle(target: u8, handle: Handle) -> Vec<u8> {
    let mut p = base(target, Command::CheckHandle);
    p.extend_from_slice(&[handle as u8, 0x00]);
    p
}

/// Splits `payload` into one `WriteEndpoint` packet followed by as many
/// `StreamEndpoint` packets as needed for `out_len`-byte reports.
///
/// `stream_target` is the target byte of continuation packets. The plugin
/// uses the plain child index (without `0x08`) unless the device needs an
/// acknowledgement per continuation packet, in which case it sets `0x08`.
pub fn write_endpoint_packets(
    target: u8,
    stream_target: u8,
    handle: Handle,
    payload: &[u8],
    out_len: usize,
) -> Vec<Vec<u8>> {
    assert!(out_len > WRITE_HEADER_LEN, "output report too small");
    let first_cap = out_len - WRITE_HEADER_LEN;
    let (first, mut rest) = payload.split_at(payload.len().min(first_cap));

    let mut p = base(target, Command::WriteEndpoint);
    p.push(handle as u8);
    p.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    p.extend_from_slice(first);
    let mut out = vec![p];

    let cap = out_len - STREAM_HEADER_LEN;
    while !rest.is_empty() {
        let (chunk, r) = rest.split_at(rest.len().min(cap));
        let mut s = base(stream_target, Command::StreamEndpoint);
        s.push(handle as u8);
        s.extend_from_slice(chunk);
        out.push(s);
        rest = r;
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResponseError {
    #[error("empty or short response ({0} bytes)")]
    Short(usize),
    #[error("response does not echo command {expected:#04x}")]
    WrongCommand { expected: u8 },
    #[error("device error {code} ({})", describe_error(*.code))]
    Device { code: u8 },
}

/// Meaning of error codes as observed/annotated in the plugin. "Probable".
pub fn describe_error(code: u8) -> &'static str {
    match code {
        1 => "invalid value",
        2 => "packet exceeds length",
        3 => "operation failed / unsupported / handle closed",
        5 => "property not supported",
        6 => "handle not open",
        9 => "property is read-only",
        _ => "unknown",
    }
}

/// A decoded response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response<'a> {
    pub command: u8,
    pub error: u8,
    pub data: &'a [u8],
}

impl<'a> Response<'a> {
    /// Decodes a raw input report as returned by the transport.
    ///
    /// Whether the transport delivers the report-ID byte is platform
    /// dependent, so both `[id, target, cmd, err, ..]` and
    /// `[target, cmd, err, ..]` are accepted by locating the command echo.
    pub fn parse(raw: &'a [u8], expected: Command) -> Result<Self, ResponseError> {
        let cmd = expected as u8;
        let off = if raw.len() > 3 && raw[0] == 0x00 && raw[2] == cmd {
            1
        } else if raw.len() > 2 && raw[1] == cmd {
            0
        } else if raw.len() < 3 {
            return Err(ResponseError::Short(raw.len()));
        } else {
            return Err(ResponseError::WrongCommand { expected: cmd });
        };
        Ok(Response {
            command: cmd,
            error: raw[off + 2],
            data: &raw[off + 3..],
        })
    }

    /// Decodes a response whose command echo sits at `cmd_index`
    /// (`[.., cmd, err, data..]`). Used for framings with extra header bytes,
    /// such as the iCUE LINK hub.
    pub fn parse_at(
        raw: &'a [u8],
        expected: Command,
        cmd_index: usize,
    ) -> Result<Self, ResponseError> {
        let cmd = expected as u8;
        if raw.len() < cmd_index + 2 {
            return Err(ResponseError::Short(raw.len()));
        }
        if raw[cmd_index] != cmd {
            return Err(ResponseError::WrongCommand { expected: cmd });
        }
        Ok(Response {
            command: cmd,
            error: raw[cmd_index + 1],
            data: &raw[cmd_index + 2..],
        })
    }

    pub fn ok(self) -> Result<Self, ResponseError> {
        match self.error {
            0 => Ok(self),
            code => Err(ResponseError::Device { code }),
        }
    }

    /// Property value: 24-bit little-endian in the first three data bytes.
    pub fn property_value(&self) -> u32 {
        let b = |i: usize| u32::from(*self.data.get(i).unwrap_or(&0));
        b(0) | b(1) << 8 | b(2) << 16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_simple_commands() {
        assert_eq!(ping(0x08), vec![0x00, 0x08, 0x12]);
        assert_eq!(
            get_property(0x08, Property::ProductId),
            vec![0x00, 0x08, 0x02, 0x12, 0x00]
        );
        assert_eq!(
            open_endpoint(0x08, Handle::Lighting, Endpoint::LightingController),
            vec![0x00, 0x08, 0x0D, 0x00, 0x22]
        );
        assert_eq!(
            close_handle(0x08, Handle::Background),
            vec![0x00, 0x08, 0x05, 0x01, 0x01]
        );
        assert_eq!(
            check_handle(0x08, Handle::Auxiliary),
            vec![0x00, 0x08, 0x09, 0x02, 0x00]
        );
    }

    #[test]
    fn set_property_allowlist() {
        assert_eq!(
            set_property(0x08, Property::Mode, Mode::Software as u32),
            Some(vec![0x00, 0x08, 0x01, 0x03, 0x00, 0x02, 0x00, 0x00])
        );
        assert_eq!(set_property(0x08, Property::HwBrightness, 1000), None);
        assert_eq!(set_property(0x08, Property::PollingRate, 4), None);
    }

    #[test]
    fn splits_payload_into_write_and_stream() {
        let payload: Vec<u8> = (0..150u8).collect();
        let pk = write_endpoint_packets(0x08, 0x08, Handle::Lighting, &payload, 64);
        assert_eq!(pk.len(), 3);
        assert_eq!(&pk[0][..8], &[0x00, 0x08, 0x06, 0x00, 150, 0, 0, 0]);
        assert_eq!(pk[0].len(), 64);
        assert_eq!(&pk[1][..4], &[0x00, 0x08, 0x07, 0x00]);
        assert_eq!(pk[1][4], 56);
        assert_eq!(pk[1].len(), 64);
        let total: usize = pk[0].len() - 8 + pk[1].len() - 4 + pk[2].len() - 4;
        assert_eq!(total, 150);
        assert_eq!(*pk[2].last().unwrap(), 149);
    }

    #[test]
    fn small_payload_single_packet() {
        let pk = write_endpoint_packets(0x08, 0x00, Handle::Lighting, &[1, 2, 3], 64);
        assert_eq!(pk, vec![vec![0x00, 0x08, 0x06, 0x00, 3, 0, 0, 0, 1, 2, 3]]);
    }

    #[test]
    fn parses_responses_with_and_without_report_id() {
        let with_id = [0x00, 0x00, 0x02, 0x00, 0xFD, 0x1B, 0x00, 0x00];
        let r = Response::parse(&with_id, Command::GetProperty).unwrap();
        assert_eq!(r.error, 0);
        assert_eq!(r.property_value(), 0x1BFD);

        let without = [0x00, 0x02, 0x00, 0xFD, 0x1B, 0x00];
        let r = Response::parse(&without, Command::GetProperty).unwrap();
        assert_eq!(r.property_value(), 0x1BFD);

        let err = [0x00, 0x08, 0x0D, 0x03];
        let e = Response::parse(&err, Command::OpenEndpoint)
            .unwrap()
            .ok()
            .unwrap_err();
        assert_eq!(e, ResponseError::Device { code: 3 });

        assert!(matches!(
            Response::parse(&[0x00, 0x08, 0x05, 0x00], Command::Ping),
            Err(ResponseError::WrongCommand { .. })
        ));
        assert!(matches!(
            Response::parse(&[0x12], Command::Ping),
            Err(ResponseError::Short(1))
        ));
    }
}
