//! Optional read-only identity extension, version 1, to HID module API 1.
use crate::{ErrorCode, ModuleError, MAX_REPORT};
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const MAX_STRING: usize = 1024;

/// Signed package permission; absent policy means no probe and no string reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub version: u32,
    #[serde(default)]
    pub serial: bool,
    #[serde(default)]
    pub product: bool,
    #[serde(default)]
    pub features: Vec<Feature>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub report_id: u8,
    /// Includes the report ID byte.
    pub max_len: usize,
}

impl Policy {
    pub fn validate(&self) -> Result<(), ModuleError> {
        let mut ids = std::collections::BTreeSet::new();
        if self.version != VERSION
            || self.features.len() > 256
            || self
                .features
                .iter()
                .any(|f| f.max_len == 0 || f.max_len > MAX_REPORT || !ids.insert(f.report_id))
        {
            return Err(ModuleError::new(
                ErrorCode::Protocol,
                "invalid probe policy",
            ));
        }
        Ok(())
    }
}

/// A probe cannot issue output reports, feature writes, or input reads.
pub trait Hid {
    fn serial_string(&mut self) -> Result<Option<String>, ModuleError>;
    fn product_string(&mut self) -> Result<Option<String>, ModuleError>;
    fn feature(&mut self, report_id: u8, len: usize) -> Result<Vec<u8>, ModuleError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_rejects_unknown_versions_duplicate_ids_and_invalid_lengths() {
        let mut policy = Policy {
            version: VERSION,
            serial: true,
            product: false,
            features: vec![Feature {
                report_id: 0x52,
                max_len: 200,
            }],
        };
        assert!(policy.validate().is_ok());
        policy.version += 1;
        assert!(policy.validate().is_err());
        policy.version = VERSION;
        policy.features.push(policy.features[0].clone());
        assert!(policy.validate().is_err());
        policy.features.pop();
        for len in [0, MAX_REPORT + 1, usize::MAX] {
            policy.features[0].max_len = len;
            assert!(policy.validate().is_err());
        }
    }
    #[test]
    fn sdk_strings_have_stable_wire_names_and_preserve_absent_vs_empty() {
        for (data, expected) in [
            (None, None),
            (Some(crate::encode(b"")), Some(String::new())),
            (
                Some(crate::encode("Controller \u{00e4}".as_bytes())),
                Some("Controller \u{00e4}".into()),
            ),
        ] {
            let reply = crate::CallReply {
                v: 1,
                call: 1,
                data,
                len: None,
                error: None,
            };
            let input = format!("{}\n", serde_json::to_string(&reply).unwrap());
            let mut output = Vec::new();
            let mut host = crate::sdk::Host::new(std::io::Cursor::new(input), &mut output);
            assert_eq!(host.serial_string().unwrap(), expected);
            let call: crate::Call = serde_json::from_slice(&output).unwrap();
            assert_eq!(call.op, crate::Op::SerialString);
            assert_eq!(call.v, 1);
            assert_eq!((call.data.as_str(), call.len, call.timeout_ms), ("", 0, 0));
        }
        for data in [vec![0xff], vec![b'x'; MAX_STRING + 1]] {
            let input = format!(
                "{}\n",
                serde_json::to_string(&crate::CallReply {
                    v: 1,
                    call: 1,
                    data: Some(crate::encode(&data)),
                    len: Some(data.len()),
                    error: None,
                })
                .unwrap()
            );
            let mut host = crate::sdk::Host::new(std::io::Cursor::new(input), Vec::new());
            assert!(host.product_string().is_err());
        }
        let request = crate::Request {
            v: 1,
            id: 5,
            method: crate::Method::Probe {
                version: VERSION,
                interface: crate::InterfaceInfo {
                    vid: 1,
                    pid: 2,
                    interface: 0,
                    usage_page: 1,
                    usage: 2,
                    product: None,
                },
            },
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["method"], "probe");
        assert_eq!(value["params"]["version"], VERSION);
    }
}
