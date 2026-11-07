//! API 4: signed, atomic transport recipes. Non-HID simulation and host-owned HID groups.
use crate::bus::{forbidden, Error, PciId};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

pub mod hid;
pub mod sdk;
pub const API: u32 = 4;
pub const MAX_STEPS: usize = 128;
pub const MAX_BYTES: usize = 4096;
pub const MAX_TRANSACTION_BYTES: usize = 16384;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Identity {
    /// One logical device; interface slots are indices into this signed list.
    HidGroup {
        device: hid::Device,
    },
    Bus {
        pci: PciId,
        backend: BusBackend,
        port: u8,
    },
    Usb {
        vid: u16,
        pid: u16,
        interface: u8,
    },
    Serial {
        vid: u16,
        pid: u16,
        com: u16,
        /// Fixed 8 data bits, no parity, one stop bit, no flow control.
        baud: u32,
    },
    Network {
        host: Ipv4Addr,
        port: u16,
        protocol: NetworkProtocol,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusBackend {
    Nvidia,
    AmdGpu,
    Motherboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkProtocol {
    Udp,
    Tcp,
}

/// Canonical IPv4 subnet, including /32 for an exact host. No DNS or redirects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subnet {
    pub network: Ipv4Addr,
    pub prefix: u8,
}
impl Subnet {
    fn mask(&self) -> Option<u32> {
        (self.prefix <= 32).then(|| {
            u32::MAX
                .checked_shl(32 - u32::from(self.prefix))
                .unwrap_or(0)
        })
    }
    pub fn contains(&self, host: Ipv4Addr) -> bool {
        self.mask().is_some_and(|mask| {
            u32::from(self.network) & mask == u32::from(self.network)
                && u32::from(host) & mask == u32::from(self.network)
        })
    }
}

/// Each operand is exact. Word data is two bytes in SMBus little-endian order;
/// raw I2C has no count/register byte. Consecutive stop=false messages form a
/// repeated-start sequence, ending with stop=true. SMBus operations always STOP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Hid {
        interface: u8,
        operation: hid::Operation,
    },
    SmbusWriteByte {
        address: u8,
    },
    SmbusReadByteData {
        address: u8,
        register: u8,
    },
    SmbusWriteByteData {
        address: u8,
        register: u8,
    },
    SmbusReadWordData {
        address: u8,
        register: u8,
    },
    SmbusWriteWordData {
        address: u8,
        register: u8,
    },
    SmbusReadBlockData {
        address: u8,
        register: u8,
    },
    SmbusWriteBlockData {
        address: u8,
        register: u8,
    },
    I2cReadBlockData {
        address: u8,
        register: u8,
    },
    I2cWriteBlockData {
        address: u8,
        register: u8,
    },
    I2cRead {
        address: u8,
        stop: bool,
    },
    I2cWrite {
        address: u8,
        stop: bool,
    },
    UsbControl {
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
    },
    UsbBulk {
        endpoint: u8,
    },
    SerialRead,
    SerialWrite,
    NetworkRead,
    NetworkWrite,
}

impl Operation {
    pub fn reads(&self) -> bool {
        match self {
            Self::Hid { operation, .. } => operation.reads(),
            Self::SmbusReadByteData { .. }
            | Self::SmbusReadWordData { .. }
            | Self::SmbusReadBlockData { .. }
            | Self::I2cReadBlockData { .. }
            | Self::I2cRead { .. }
            | Self::SerialRead
            | Self::NetworkRead => true,
            Self::UsbControl { request_type, .. } => request_type & 0x80 != 0,
            Self::UsbBulk { endpoint } => endpoint & 0x80 != 0,
            _ => false,
        }
    }
    fn address(&self) -> Option<u8> {
        match self {
            Self::SmbusWriteByte { address }
            | Self::SmbusReadByteData { address, .. }
            | Self::SmbusWriteByteData { address, .. }
            | Self::SmbusReadWordData { address, .. }
            | Self::SmbusWriteWordData { address, .. }
            | Self::SmbusReadBlockData { address, .. }
            | Self::SmbusWriteBlockData { address, .. }
            | Self::I2cReadBlockData { address, .. }
            | Self::I2cWriteBlockData { address, .. }
            | Self::I2cRead { address, .. }
            | Self::I2cWrite { address, .. } => Some(*address),
            _ => None,
        }
    }
    fn stop(&self) -> bool {
        match self {
            Self::I2cRead { stop, .. } | Self::I2cWrite { stop, .. } => *stop,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub operation: Operation,
    pub data: Vec<u8>,
    pub read_len: u16,
}
impl Step {
    pub fn len(&self) -> usize {
        if self.operation.reads() {
            usize::from(self.read_len)
        } else {
            self.data.len()
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn valid(&self, identity: &Identity) -> bool {
        use Operation::*;
        let n = self.len();
        if n == 0
            || n > MAX_BYTES
            || (self.operation.reads() && !self.data.is_empty())
            || (!self.operation.reads() && self.read_len != 0)
        {
            return false;
        }
        if let Some(address) = self.operation.address() {
            // Read-only SPD data commands allowed; raw reads are excluded too.
            if !(0x08..=0x77).contains(&address)
                || (forbidden(address)
                    && (!self.operation.reads() || matches!(self.operation, I2cRead { .. })))
            {
                return false;
            }
        }
        match (&self.operation, identity) {
            (
                Hid {
                    interface,
                    operation,
                },
                Identity::HidGroup { device },
            ) => device.valid_step(*interface, operation, &self.data, self.read_len),
            (
                SmbusWriteByte { .. } | SmbusReadByteData { .. } | SmbusWriteByteData { .. },
                Identity::Bus { .. },
            ) => n == 1,
            (SmbusReadWordData { .. } | SmbusWriteWordData { .. }, Identity::Bus { .. }) => n == 2,
            (SmbusReadBlockData { .. } | SmbusWriteBlockData { .. }, Identity::Bus { .. }) => {
                n <= 32
            }
            (I2cReadBlockData { .. } | I2cWriteBlockData { .. }, Identity::Bus { .. }) => n <= 255,
            (I2cRead { .. } | I2cWrite { .. }, Identity::Bus { .. }) => true,
            (
                UsbControl {
                    request_type,
                    index,
                    ..
                },
                Identity::Usb { interface, .. },
            ) => {
                // Vendor device/interface/endpoint requests only. All setup
                // operands remain exact signed fields; no standard USB writes.
                matches!(request_type & 0x7f, 0x40..=0x42)
                    && (request_type & 0x1f != 1 || *index == u16::from(*interface))
            }
            (UsbBulk { endpoint }, Identity::Usb { .. }) => {
                endpoint & 0x70 == 0 && endpoint & 0x0f != 0
            }
            (SerialRead | SerialWrite, Identity::Serial { .. }) => true,
            (NetworkRead | NetworkWrite, Identity::Network { .. }) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub id: String,
    pub steps: Vec<Step>,
    /// One mask per write byte; fixed bits are taken from steps.data. Read masks
    /// must be empty. Selectors/commands should have zero masks after audit.
    pub masks: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Non-HID transports use simulation; grouped HID uses host-supplied handles.
    pub backend: String,
    pub identities: Vec<Identity>,
    pub subnets: Vec<Subnet>,
    pub recipes: Vec<Recipe>,
}
impl Policy {
    pub fn validate(&self) -> Result<(), Error> {
        if !matches!(self.backend.as_str(), "simulation" | "hid")
            || (self.backend == "simulation"
                && self
                    .identities
                    .iter()
                    .any(|i| matches!(i, Identity::HidGroup { .. })))
            || (self.backend == "hid"
                && self
                    .identities
                    .iter()
                    .any(|i| !matches!(i, Identity::HidGroup { .. })))
            || self.identities.is_empty()
            || self.identities.len() > 1024
            || self.recipes.is_empty()
            || self.recipes.len() > 256
            || self.subnets.len() > 64
        {
            return Err(Error::InvalidPolicy);
        }
        if self.subnets.iter().any(|s| !s.contains(s.network)) {
            return Err(Error::InvalidPolicy);
        }
        for identity in &self.identities {
            match identity {
                Identity::HidGroup { device } if !device.valid() => {
                    return Err(Error::InvalidPolicy)
                }
                Identity::Serial { com, baud, .. }
                    if *com == 0 || *baud == 0 || *baud > 4_000_000 =>
                {
                    return Err(Error::InvalidPolicy)
                }
                Identity::Network { host, port, .. }
                    if *port == 0
                        || host.is_unspecified()
                        || host.is_multicast()
                        || host.is_broadcast()
                        || !self.subnets.iter().any(|s| s.contains(*host)) =>
                {
                    return Err(Error::InvalidPolicy)
                }
                _ => {}
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        for r in &self.recipes {
            if r.id.is_empty()
                || r.id.len() > 64
                || !ids.insert(&r.id)
                || r.steps.is_empty()
                || r.steps.len() > MAX_STEPS
                || r.steps.len() != r.masks.len()
                || r.steps.iter().map(Step::len).sum::<usize>() > MAX_TRANSACTION_BYTES
                || !r.steps.last().is_some_and(|s| s.operation.stop())
                || !self
                    .identities
                    .iter()
                    .any(|i| r.steps.iter().all(|s| s.valid(i)))
                || r.steps
                    .iter()
                    .zip(&r.masks)
                    .any(|(s, m)| m.len() != s.data.len())
            {
                return Err(Error::InvalidPolicy);
            }
            if self.identities.iter().any(|identity| match identity {
                Identity::HidGroup { device } if device.hidpp.is_some() => {
                    r.steps.iter().zip(&r.masks).any(|(step, masks)| {
                        matches!(step.operation, Operation::Hid { .. })
                            && !step.operation.reads()
                            // The index is checked against the bound identity;
                            // feature indices may be discovered at runtime.
                            && (masks.first() != Some(&0) || masks.get(3) != Some(&0))
                    })
                }
                _ => false,
            }) {
                return Err(Error::InvalidPolicy);
            }
            // Repeated starts cannot bridge to SMBus or another address.
            if r.steps.windows(2).any(|w| {
                !w[0].operation.stop()
                    && (!matches!(
                        w[1].operation,
                        Operation::I2cRead { .. } | Operation::I2cWrite { .. }
                    ) || w[0].operation.address() != w[1].operation.address())
            }) {
                return Err(Error::InvalidPolicy);
            }
        }
        if self
            .identities
            .iter()
            .any(|i| matches!(i, Identity::HidGroup { .. }))
            && !["claim", "release"].iter().all(|id| {
                self.recipes
                    .iter()
                    .any(|r| r.id == *id && r.steps.iter().any(|s| !s.operation.reads()))
            })
        {
            return Err(Error::InvalidPolicy);
        }
        Ok(())
    }
    pub fn authorize(
        &self,
        identity: &Identity,
        recipe: &str,
        steps: &[Step],
    ) -> Result<(), Error> {
        if !self.allows_identity(identity) {
            return Err(Error::Denied);
        }
        let r = self
            .recipes
            .iter()
            .find(|r| r.id == recipe)
            .ok_or(Error::Denied)?;
        if steps.len() != r.steps.len()
            || steps.iter().zip(&r.steps).zip(&r.masks).any(|((s, t), m)| {
                !s.valid(identity)
                    || s.operation != t.operation
                    || s.read_len != t.read_len
                    || s.data.len() != t.data.len()
                    || s.data
                        .iter()
                        .zip(&t.data)
                        .zip(m)
                        .any(|((a, b), mask)| (a ^ b) & !mask != 0)
            })
        {
            return Err(Error::Denied);
        }
        Ok(())
    }

    pub fn allows_identity(&self, actual: &Identity) -> bool {
        self.identities.iter().any(|signed| match (signed, actual) {
            (Identity::HidGroup { device }, Identity::HidGroup { device: actual }) => {
                device.matches(actual)
            }
            _ => signed == actual,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub v: u32,
    pub call: u64,
    pub capability: u64,
    pub recipe: String,
    pub steps: Vec<Step>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessCall {
    pub request: u64,
    pub transaction: Call,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub v: u32,
    pub call: u64,
    pub result: Result<Vec<Vec<u8>>, Error>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub v: u32,
    pub id: u64,
    pub method: Method,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Method {
    Hello,
    Identify { identity: Identity },
    Initialize { identity: Identity, capability: u64 },
    Frame { rgb: String },
    Shutdown,
}
pub type Response = crate::bus::Response;

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> Policy {
        Policy {
            backend: "simulation".into(),
            identities: vec![Identity::Usb {
                vid: 0x1234,
                pid: 0x5678,
                interface: 2,
            }],
            subnets: vec![],
            recipes: vec![Recipe {
                id: "rgb".into(),
                steps: vec![Step {
                    operation: Operation::UsbBulk { endpoint: 2 },
                    data: vec![0x40, 0, 0, 0],
                    read_len: 0,
                }],
                masks: vec![vec![0, 0xff, 0x0f, 0xff]],
            }],
        }
    }
    #[test]
    fn masks_allow_only_reviewed_payload_bits_and_exact_usb_operands() {
        let p = policy();
        p.validate().unwrap();
        let identity = &p.identities[0];
        let mut steps = p.recipes[0].steps.clone();
        steps[0].data = vec![0x40, 255, 15, 8];
        p.authorize(identity, "rgb", &steps).unwrap();
        steps[0].data[2] = 16;
        assert_eq!(p.authorize(identity, "rgb", &steps), Err(Error::Denied));
        steps[0].data[2] = 0;
        steps[0].operation = Operation::UsbBulk { endpoint: 3 };
        assert_eq!(p.authorize(identity, "rgb", &steps), Err(Error::Denied));
        let wrong = Identity::Usb {
            vid: 0x1234,
            pid: 0x5679,
            interface: 2,
        };
        assert_eq!(
            p.authorize(&wrong, "rgb", &p.recipes[0].steps),
            Err(Error::Denied)
        );
        let mut invalid = p.clone();
        invalid.recipes[0].steps[0].operation = Operation::UsbBulk { endpoint: 0x72 };
        assert_eq!(invalid.validate(), Err(Error::InvalidPolicy));
    }
    #[test]
    fn subnet_is_canonical_and_never_grants_arbitrary_hosts_or_protocols() {
        let subnet = Subnet {
            network: "192.0.2.0".parse().unwrap(),
            prefix: 24,
        };
        assert!(subnet.contains("192.0.2.7".parse().unwrap()));
        assert!(!subnet.contains("192.0.3.7".parse().unwrap()));
        assert!(!Subnet {
            network: "192.0.2.1".parse().unwrap(),
            prefix: 24
        }
        .contains("192.0.2.1".parse().unwrap()));
        assert!(!Subnet {
            network: "0.0.0.0".parse().unwrap(),
            prefix: 33
        }
        .contains("192.0.2.1".parse().unwrap()));
        let identity = Identity::Network {
            host: "192.0.2.7".parse().unwrap(),
            port: 6742,
            protocol: NetworkProtocol::Tcp,
        };
        let mut p = policy();
        p.identities = vec![identity.clone()];
        p.subnets = vec![subnet];
        p.recipes[0].steps[0].operation = Operation::NetworkWrite;
        p.validate().unwrap();
        for wrong in [
            Identity::Network {
                host: "192.0.2.8".parse().unwrap(),
                port: 6742,
                protocol: NetworkProtocol::Tcp,
            },
            Identity::Network {
                host: "192.0.2.7".parse().unwrap(),
                port: 6743,
                protocol: NetworkProtocol::Tcp,
            },
            Identity::Network {
                host: "192.0.2.7".parse().unwrap(),
                port: 6742,
                protocol: NetworkProtocol::Udp,
            },
        ] {
            assert_eq!(
                p.authorize(&wrong, "rgb", &p.recipes[0].steps),
                Err(Error::Denied)
            );
        }
        p.subnets.clear();
        assert_eq!(p.validate(), Err(Error::InvalidPolicy));
    }
    #[test]
    fn v4_is_strict_and_does_not_widen_api2() {
        let step = Step {
            operation: Operation::I2cWrite {
                address: 0x60,
                stop: true,
            },
            data: vec![0; 64],
            read_len: 0,
        };
        let json = serde_json::to_string(&step).unwrap();
        assert_eq!(serde_json::from_str::<Step>(&json).unwrap(), step);
        assert!(serde_json::from_str::<crate::bus::Transfer>(&json).is_err());
        assert!(serde_json::from_str::<Identity>(
            r#"{"kind":"serial","vid":1,"pid":2,"com":7,"baud":115200,"path":"COM8"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Operation>(
            r#"{"op":"i2c_write","address":256,"stop":true}"#
        )
        .is_err());
    }
}
