//! Grouped HID descriptors and HID++ addressing for API 4 signed recipes.
//! No device paths, serials or native handles cross the process boundary.
//!
//! A schema-4 package selects backend `hid` and signs the complete ordered
//! interface list. The host binds that list to handles from one physical USB
//! connection. Interface slots in operations refer to this list, not arbitrary
//! OS handles. Legacy HID API 1 and simulation packages keep their wire formats.
//!
//! The child may run an audited `probe` recipe before claiming control. It must
//! complete `claim` during initialize, use its ordinary recipes while claimed,
//! and complete `release` during shutdown. Claim/release packet semantics are
//! family-specific and belong in the signed recipes. Failures revoke the lease;
//! abandoning a claimed device quarantines the shared group until it is reopened.
//!
//! HID++ descriptors bind the receiver slot and software ID. A signed Logitech
//! PID of 0xffff permits wildcard detection, while interface selectors remain
//! exact. [`Device::exchange`] builds an addressed request and a correlated read;
//! a signed mask may allow runtime-discovered feature indices. The host matches
//! the complete response header and withholds other slots' reports.
use serde::{Deserialize, Serialize};

pub const MAX_INTERFACES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    pub number: i32,
    pub usage_page: u16,
    pub usage: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hidpp {
    /// Paired receiver slot 1..6, or 0xff for a directly attached device.
    pub device_index: u8,
    /// Nonzero HID++ 2.0 software ID, fixed for this logical device.
    pub software_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub vid: u16,
    pub pid: u16,
    pub interfaces: Vec<Interface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidpp: Option<Hidpp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Write,
    SendFeature,
    /// For HID++, the complete response header is checked before disclosure.
    Read {
        report_id: u8,
        feature: u8,
        function: u8,
    },
    GetFeature {
        report_id: u8,
    },
    /// Correlate to the preceding HID++ write on this interface in this recipe.
    /// Supports feature indices discovered at runtime without exposing other slots.
    ReadReply {
        report_id: u8,
    },
}

impl Operation {
    pub fn reads(&self) -> bool {
        matches!(
            self,
            Self::Read { .. } | Self::GetFeature { .. } | Self::ReadReply { .. }
        )
    }
}

pub fn report_len(report_id: u8) -> Option<usize> {
    match report_id {
        0x10 => Some(7),
        0x11 => Some(20),
        0x12 => Some(64),
        _ => None,
    }
}

impl Device {
    /// A signed Logitech HID++ PID 0xffff is a wildcard. All interface selectors,
    /// the vendor, receiver index and software ID still have to match exactly.
    pub fn matches(&self, actual: &Self) -> bool {
        self.vid == actual.vid
            && (self.pid == actual.pid
                || (self.pid == 0xffff && self.vid == 0x046d && self.hidpp.is_some()))
            && self.interfaces == actual.interfaces
            && self.hidpp == actual.hidpp
    }

    /// Build an addressed HID++ 2.0 request and its correlated response read.
    /// `function` is the 4-bit function number, before adding the software ID.
    pub fn exchange(
        &self,
        interface: u8,
        report_id: u8,
        feature: u8,
        function: u8,
        parameters: &[u8],
    ) -> Result<Vec<super::Step>, crate::bus::Error> {
        let h = self.hidpp.as_ref().ok_or(crate::bus::Error::Denied)?;
        let len = report_len(report_id).ok_or(crate::bus::Error::Denied)?;
        if !self.valid()
            || function > 15
            || parameters.len() > len - 4
            || usize::from(interface) >= self.interfaces.len()
        {
            return Err(crate::bus::Error::Denied);
        }
        let mut data = vec![0; len];
        data[..4].copy_from_slice(&[
            report_id,
            h.device_index,
            feature,
            (function << 4) | h.software_id,
        ]);
        data[4..4 + parameters.len()].copy_from_slice(parameters);
        Ok(vec![
            super::Step {
                operation: super::Operation::Hid {
                    interface,
                    operation: Operation::Write,
                },
                data,
                read_len: 0,
            },
            super::Step {
                operation: super::Operation::Hid {
                    interface,
                    operation: Operation::ReadReply { report_id },
                },
                data: vec![],
                read_len: len as u16,
            },
        ])
    }
    pub fn valid(&self) -> bool {
        !self.interfaces.is_empty()
            && self.interfaces.len() <= MAX_INTERFACES
            && self
                .interfaces
                .iter()
                .enumerate()
                .all(|(n, i)| i.number >= 0 && !self.interfaces[..n].contains(i))
            && self.hidpp.as_ref().is_none_or(|h| {
                self.vid == 0x046d
                    && (matches!(h.device_index, 1..=6 | 0xff))
                    && (1..=15).contains(&h.software_id)
            })
    }

    pub(super) fn valid_step(&self, slot: u8, op: &Operation, data: &[u8], read_len: u16) -> bool {
        if usize::from(slot) >= self.interfaces.len() {
            return false;
        }
        let Some(h) = &self.hidpp else {
            return !matches!(op, Operation::ReadReply { .. });
        };
        match op {
            Operation::Write | Operation::SendFeature => {
                data.len() >= 4
                    && report_len(data[0]) == Some(data.len())
                    && data[1] == h.device_index
                    && data[3] & 0x0f == h.software_id
            }
            Operation::Read {
                report_id,
                function,
                ..
            } => {
                report_len(*report_id) == Some(usize::from(read_len))
                    && function & 0x0f == h.software_id
            }
            Operation::ReadReply { report_id } => {
                report_len(*report_id) == Some(usize::from(read_len))
            }
            // Feature reads cannot correlate a HID++ response to its requester.
            Operation::GetFeature { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Error;
    use crate::transport::{Identity, Operation as TransportOperation, Policy, Recipe, Step};

    fn device() -> Device {
        Device {
            vid: 0x046d,
            pid: 0xc547,
            interfaces: vec![
                Interface {
                    number: 1,
                    usage_page: 0xff00,
                    usage: 1,
                },
                Interface {
                    number: 2,
                    usage_page: 0xff00,
                    usage: 2,
                },
            ],
            hidpp: Some(Hidpp {
                device_index: 2,
                software_id: 7,
            }),
        }
    }
    fn policy() -> Policy {
        let mut data = vec![0; 20];
        data[..4].copy_from_slice(&[0x11, 2, 0x0a, 0x17]);
        Policy {
            backend: "hid".into(),
            identities: vec![Identity::HidGroup { device: device() }],
            subnets: vec![],
            recipes: ["claim", "frame", "release"]
                .into_iter()
                .map(|id| Recipe {
                    id: id.into(),
                    steps: vec![Step {
                        operation: TransportOperation::Hid {
                            interface: 1,
                            operation: Operation::Write,
                        },
                        data: data.clone(),
                        read_len: 0,
                    }],
                    masks: vec![vec![0; 20]],
                })
                .collect(),
        }
    }
    #[test]
    fn signed_group_checks_receiver_address_software_id_and_interface_slot() {
        let p = policy();
        p.validate().unwrap();
        let identity = &p.identities[0];
        let original = p.recipes[1].steps.clone();
        p.authorize(identity, "frame", &original).unwrap();
        for byte in 0..4 {
            let mut steps = original.clone();
            steps[0].data[byte] ^= 1;
            assert_eq!(p.authorize(identity, "frame", &steps), Err(Error::Denied));
        }
        let mut steps = original;
        steps[0].operation = TransportOperation::Hid {
            interface: 0,
            operation: Operation::Write,
        };
        assert_eq!(p.authorize(identity, "frame", &steps), Err(Error::Denied));
        for change in 0..7 {
            let mut invalid = p.clone();
            match change {
                0 => invalid.backend = "simulation".into(),
                1 => invalid.recipes[1].steps[0].data[1] = 3,
                2 => invalid.recipes[1].masks[0][3] = 0xff,
                3 => invalid.recipes[1].steps[0].data.pop().map(|_| ()).unwrap(),
                4 => {
                    invalid.recipes[1].steps[0].operation = TransportOperation::Hid {
                        interface: 2,
                        operation: Operation::Write,
                    }
                }
                5 => invalid.recipes.retain(|r| r.id != "claim"),
                _ => invalid.recipes.retain(|r| r.id != "release"),
            }
            assert_eq!(invalid.validate(), Err(Error::InvalidPolicy));
        }
    }
    #[test]
    fn group_descriptors_and_hidpp_headers_are_bounded_and_versioned() {
        let d = device();
        assert!(d.valid());
        let json = serde_json::to_string(&Identity::HidGroup { device: d.clone() }).unwrap();
        assert_eq!(
            serde_json::from_str::<Identity>(&json).unwrap(),
            Identity::HidGroup { device: d.clone() }
        );
        assert!(serde_json::from_str::<Device>(&json).is_err());
        for index in [0, 7, 0xfe] {
            let mut invalid = d.clone();
            invalid.hidpp.as_mut().unwrap().device_index = index;
            assert!(!invalid.valid());
        }
        let mut invalid = d.clone();
        invalid.interfaces.push(invalid.interfaces[0].clone());
        assert!(!invalid.valid());
        for (report, len) in [(0x10, 7), (0x11, 20), (0x12, 64)] {
            let mut data = vec![0; len];
            data[..4].copy_from_slice(&[report, 2, 0x0a, 0x17]);
            assert!(d.valid_step(1, &Operation::Write, &data, 0));
            assert!(d.valid_step(
                1,
                &Operation::Read {
                    report_id: report,
                    feature: 0x0a,
                    function: 0x17
                },
                &[],
                len as u16
            ));
            assert!(!d.valid_step(
                1,
                &Operation::Read {
                    report_id: report,
                    feature: 0x0a,
                    function: 0x18
                },
                &[],
                len as u16
            ));
        }
        assert!(!d.valid_step(1, &Operation::GetFeature { report_id: 0x11 }, &[], 20));
    }
    #[test]
    fn wildcard_pid_and_runtime_feature_indices_preserve_receiver_binding() {
        let mut p = policy();
        let mut signed = device();
        signed.pid = 0xffff;
        p.identities = vec![Identity::HidGroup { device: signed }];
        let frame = &mut p.recipes[1];
        frame.steps = device().exchange(1, 0x11, 0, 1, &[1, 2, 3]).unwrap();
        frame.masks = vec![vec![0; 20], vec![]];
        frame.masks[0][1] = 0xff; // Bound index is still enforced by valid_step.
        frame.masks[0][2] = 0xff; // Feature index returned by root discovery.
        p.validate().unwrap();
        let actual = Identity::HidGroup { device: device() };
        let discovered = device().exchange(1, 0x11, 0x3b, 1, &[1, 2, 3]).unwrap();
        p.authorize(&actual, "frame", &discovered).unwrap();
        let mut wrong = discovered.clone();
        wrong[0].data[1] = 3;
        assert_eq!(p.authorize(&actual, "frame", &wrong), Err(Error::Denied));
        let mut other_vendor = device();
        other_vendor.vid = 0x1234;
        assert!(!p.allows_identity(&Identity::HidGroup {
            device: other_vendor
        }));
        assert_eq!(&discovered[0].data[..7], &[0x11, 2, 0x3b, 0x17, 1, 2, 3]);
        assert!(device().exchange(2, 0x11, 0, 1, &[]).is_err());
        assert!(device().exchange(1, 0x11, 0, 16, &[]).is_err());
        assert!(device().exchange(1, 0x11, 0, 1, &[0; 17]).is_err());
    }
}
