//! API v2 bus protocol. Separate from the unchanged v1 HID process protocol.
//! Only flat, 8-bit register maps are supported; no raw I2C or bank selectors.
use serde::{Deserialize, Serialize};

pub const API: u32 = 2;
pub const MAX_BLOCK: usize = 32;
pub const MAX_OPERATIONS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PciId {
    pub ven: u16,
    pub dev: u16,
    pub subven: u16,
    pub subdev: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    SmbusReadByteData,
    SmbusWriteByteData,
    SmbusReadBlockData,
    SmbusWriteBlockData,
    I2cReadRegister,
    I2cWriteRegister,
}

impl Operation {
    pub fn writes(self) -> bool {
        matches!(self, Self::SmbusWriteByteData | Self::SmbusWriteBlockData | Self::I2cWriteRegister)
    }

    pub fn byte(self) -> bool {
        matches!(self, Self::SmbusReadByteData | Self::SmbusWriteByteData)
    }
}

/// One signed rule. All bytes must match this single rule, including a write mask.
/// `write_mask` authorizes variable bits; other bits must equal `write_value`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub address: u8,
    pub first: u8,
    pub last: u8,
    pub op: Operation,
    pub max_len: u8,
    pub write_mask: u8,
    pub write_value: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub pci: Vec<PciId>,
    /// The sole backend currently supported. Native names fail validation.
    pub backend: String,
    /// Must be true: banked controllers require a later audited recipe protocol.
    pub flat_registers: bool,
    pub rules: Vec<Rule>,
}

impl Policy {
    pub fn validate(&self) -> Result<(), Error> {
        if self.backend != "simulation" || !self.flat_registers
            || self.pci.is_empty() || self.pci.len() > 256
            || self.rules.is_empty() || self.rules.len() > 256
        {
            return Err(Error::InvalidPolicy);
        }
        for rule in &self.rules {
            if !(0x08..=0x77).contains(&rule.address) || rule.first > rule.last
                || rule.max_len == 0 || usize::from(rule.max_len) > MAX_BLOCK
                || (rule.op.byte() && rule.max_len != 1)
                || rule.write_value & rule.write_mask != 0
                || (rule.op.writes() && forbidden(rule.address))
            {
                return Err(Error::InvalidPolicy);
            }
        }
        Ok(())
    }

    pub fn authorize(&self, request: &Transfer) -> Result<(), Error> {
        let len = request.len();
        if !(0x08..=0x77).contains(&request.address)
            || len == 0 || len > MAX_BLOCK || (request.op.byte() && len != 1)
            || (request.op.writes() && (request.read_len != 0 || forbidden(request.address)))
            || (!request.op.writes() && !request.data.is_empty())
        {
            return Err(Error::Denied);
        }
        let last = usize::from(request.register).checked_add(len - 1).ok_or(Error::Denied)?;
        if self.rules.iter().any(|rule| {
            rule.address == request.address && rule.op == request.op
                && request.register >= rule.first && last <= usize::from(rule.last)
                && len <= usize::from(rule.max_len)
                && request.data.iter().all(|b| b & !rule.write_mask == rule.write_value)
        }) {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
}

pub fn forbidden(address: u8) -> bool {
    matches!(address, 0x36 | 0x37 | 0x50..=0x57)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transfer {
    pub op: Operation,
    pub address: u8,
    pub register: u8,
    pub data: Vec<u8>,
    pub read_len: u8,
}

impl Transfer {
    pub fn len(&self) -> usize {
        if self.op.writes() { self.data.len() } else { usize::from(self.read_len) }
    }

    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub v: u32,
    pub api: Vec<u32>,
    pub transport: String,
}

impl Hello {
    pub fn negotiate(&self) -> Result<(), Error> {
        if self.v == API && self.api.contains(&API) && self.transport == "simulation" {
            Ok(())
        } else { Err(Error::Unsupported) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub v: u32,
    pub call: u64,
    pub capability: u64,
    pub transfers: Vec<Transfer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub v: u32,
    pub call: u64,
    /// Read bytes, or an empty vector for each completed write.
    pub result: Result<Vec<Vec<u8>>, Error>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    InvalidPolicy,
    Unsupported,
    Denied,
    Busy,
    Nack,
    Timeout,
    Disconnected,
    Cancelled,
    Faulted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_and_raw_transfers_are_rejected() {
        assert_eq!(Hello { v: 1, api: vec![1], transport: "simulation".into() }.negotiate(), Err(Error::Unsupported));
        assert!(serde_json::from_str::<Transfer>(r#"{"op":"i2c_raw","address":80,"register":0,"data":[1],"read_len":0}"#).is_err());
        assert!(serde_json::from_str::<Transfer>(r#"{"op":"i2c_read_register","address":256,"register":0,"data":[],"read_len":1}"#).is_err());
    }

    #[test]
    fn v2_wire_round_trip() {
        let call = Call { v: API, call: 7, capability: 3, transfers: vec![Transfer {
            op: Operation::SmbusReadByteData, address: 0x18, register: 2, data: vec![], read_len: 1,
        }] };
        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("smbus_read_byte_data"));
        assert_eq!(serde_json::from_str::<Call>(&json).unwrap(), call);
    }
}
