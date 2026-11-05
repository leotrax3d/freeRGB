//! Simulated hub: every write is checked against the signed policy first.
use super::*;

struct SimHub {
    policy: Policy,
    mode: u8,
    /// Mode observed while each endpoint was read.
    read_modes: Vec<u8>,
    open: Option<u8>,
    pending: Vec<Vec<u8>>,
    stuck_in_software: bool,
}

impl SimHub {
    fn new() -> Self {
        Self {
            policy: policy(),
            mode: Mode::Hardware as u8,
            read_modes: vec![],
            open: None,
            pending: vec![],
            stuck_in_software: false,
        }
    }

    fn respond(&mut self, cmd: u8, data: &[u8]) {
        let mut r = vec![0x00, TARGET_HUB, cmd, 0x00];
        r.extend_from_slice(data);
        self.pending.push(r);
    }
}

impl Hid for SimHub {
    fn write(&mut self, report: &[u8]) -> Result<usize, ModuleError> {
        self.policy
            .authorize(Op::Write, report, report.len())
            .map_err(|_| failed(format!("policy denies {report:02x?}")))?;
        let cmd = report[3];
        match cmd {
            0x12 => self.respond(cmd, &[]),
            0x02 => {
                let mode = self.mode;
                self.respond(cmd, &[mode, 0, 0]);
            }
            0x01 => {
                let wanted = report[6];
                if !(self.stuck_in_software && wanted == Mode::Hardware as u8) {
                    self.mode = wanted;
                }
                self.respond(cmd, &[]);
            }
            0x09 => {
                let mut r = vec![
                    0x00,
                    TARGET_HUB,
                    cmd,
                    if self.open.is_some() { 0 } else { 3 },
                ];
                r.push(0);
                self.pending.push(r);
            }
            0x05 => {
                self.open = None;
                self.respond(cmd, &[]);
            }
            0x0D => {
                self.open = Some(report[5]);
                self.respond(cmd, &[]);
            }
            0x08 => {
                self.read_modes.push(self.mode);
                let data: Vec<u8> = match self.open {
                    Some(EP_FAN_RPM) => vec![0x25, 0x00, 2, 0, 0xdc, 0x05, 1, 0, 0],
                    Some(EP_TEMPERATURES) => vec![0x10, 0x00, 1, 0, 0x2c, 0x01],
                    _ => vec![],
                };
                self.respond(cmd, &data);
            }
            _ => return Err(failed("unexpected command")),
        }
        Ok(report.len())
    }
    fn read(&mut self, len: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        self.policy
            .authorize(Op::Read, &[], len)
            .map_err(|_| failed("policy denies read"))?;
        Ok(if self.pending.is_empty() {
            vec![]
        } else {
            self.pending.remove(0)
        })
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        Err(failed("policy has no feature reports"))
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        Err(failed("policy has no feature reports"))
    }
}

#[test]
fn committed_policy_matches_generator_and_is_valid() {
    let committed: Policy = serde_json::from_str(POLICY).unwrap();
    assert_eq!(committed, policy());
    committed.validate().unwrap();
    for packet in packets() {
        committed
            .authorize(Op::Write, &packet, packet.len())
            .unwrap();
    }
    // Nothing that writes an endpoint or changes another property.
    let lighting = open_endpoint(TARGET_HUB, Handle::Lighting, 0x22);
    assert!(committed
        .authorize(Op::Write, &lighting, lighting.len())
        .is_err());
}

#[test]
fn sample_uses_a_software_window_and_restores_hardware_mode() {
    let mut hub = SimHub::new();
    let mut plugin = Link { policy: policy() };
    let sample = plugin.sample(&mut hub).unwrap();
    assert_eq!(hub.mode, Mode::Hardware as u8);
    assert_eq!(hub.read_modes, vec![Mode::Software as u8; 2]);
    let value = |id: &str| sample.sensors.iter().find(|s| s.id == id).unwrap().value;
    assert_eq!(value("slot0_rpm"), Some(1500.0));
    assert_eq!(value("slot1_rpm"), None, "status != 0 means absent");
    assert_eq!(value("slot0_temp"), Some(30.0));
    assert_eq!(value("slot5_temp"), None);
    assert_eq!(sample.sensors.len(), plugin.policy.sensors.len());
}

#[test]
fn hub_already_in_software_mode_is_not_switched() {
    let mut hub = SimHub::new();
    hub.mode = Mode::Software as u8;
    Link { policy: policy() }.sample(&mut hub).unwrap();
    assert_eq!(hub.mode, Mode::Software as u8);
}

#[test]
fn failed_restore_is_an_error_for_the_host_failsafe() {
    let mut hub = SimHub::new();
    hub.stuck_in_software = true;
    let error = Link { policy: policy() }.sample(&mut hub).unwrap_err();
    assert!(error.message.contains("hardware mode"));
}
