//! Signed packages and real fixture processes; all transport I/O is simulated.
use ed25519_dalek::SigningKey;
use freergb_core::Rgb;
use freergb_module_api::bus::{Error, PciId};
use freergb_module_api::transport::*;
use freergb_modules::catalog::*;
use freergb_modules::package::{self, PackageManifest};
use freergb_modules::store::{Store, Verified, VerifiedTransport};
use freergb_modules::transport_host::{Broker, SimulatedBus, TransportProcess};
use freergb_modules::trust::{sign, TrustedKeys};
use std::time::{Duration, Instant};

fn bus_identity() -> Identity {
    Identity::Bus {
        pci: PciId {
            ven: 0x10de,
            dev: 0x1234,
            subven: 0x1458,
            subdev: 0x5678,
        },
        backend: BusBackend::Nvidia,
        port: 1,
    }
}
fn write(operation: Operation, data: &[u8]) -> Step {
    Step {
        operation,
        data: data.into(),
        read_len: 0,
    }
}
fn read(operation: Operation, len: u16) -> Step {
    Step {
        operation,
        data: vec![],
        read_len: len,
    }
}
fn policy(identity: Identity, steps: Vec<Step>) -> Policy {
    Policy {
        backend: "simulation".into(),
        identities: vec![identity],
        subnets: vec![Subnet {
            network: "192.0.2.0".parse().unwrap(),
            prefix: 24,
        }],
        recipes: ["initialize", "frame", "shutdown"]
            .into_iter()
            .map(|id| Recipe {
                id: id.into(),
                masks: steps.iter().map(|s| vec![0; s.data.len()]).collect(),
                steps: steps.clone(),
            })
            .collect(),
    }
}
fn call(b: &Broker, steps: Vec<Step>) -> Call {
    Call {
        v: API,
        call: 1,
        capability: b.capability(),
        recipe: "frame".into(),
        steps,
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(2)
}

#[test]
fn exact_gpu_subsystem_backend_and_port_are_required() {
    let identity = bus_identity();
    let p = policy(
        identity.clone(),
        vec![write(
            Operation::SmbusWriteByteData {
                address: 0x60,
                register: 0x36,
            },
            &[1],
        )],
    );
    p.validate().unwrap(); // Register 0x36 is legal; bus address 0x36 is forbidden.
    if let Identity::Bus { pci, backend, port } = identity {
        assert!(Broker::new(
            p.clone(),
            Identity::Bus {
                pci,
                backend,
                port: 2
            },
            SimulatedBus::default()
        )
        .is_err());
        let mut other = pci;
        other.subdev ^= 1;
        assert!(Broker::new(
            p.clone(),
            Identity::Bus {
                pci: other,
                backend,
                port
            },
            SimulatedBus::default()
        )
        .is_err());
        assert!(Broker::new(
            p,
            Identity::Bus {
                pci,
                backend: BusBackend::AmdGpu,
                port
            },
            SimulatedBus::default()
        )
        .is_err());
    }
}

#[test]
fn deny_entire_recipe_before_dispatch_and_revoke_on_failure() {
    let steps = vec![
        write(Operation::SmbusWriteByte { address: 0x62 }, &[0x10]),
        write(Operation::SmbusWriteByte { address: 0x62 }, &[3]),
    ];
    let p = policy(bus_identity(), steps.clone());
    for mutate in 0..6 {
        let bus = SimulatedBus::default();
        let mut b = Broker::new(p.clone(), bus_identity(), bus.clone()).unwrap();
        let mut c = call(&b, steps.clone());
        match mutate {
            0 => c.steps[1].data[0] ^= 1,
            1 => {
                c.steps.pop();
            }
            2 => c.steps[1].operation = Operation::SmbusWriteByte { address: 0x50 },
            3 => c.capability += 1,
            4 => c.v = 2,
            _ => c.recipe = "unknown".into(),
        }
        assert_eq!(b.transact(&c, deadline()).result, Err(Error::Denied));
        assert!(bus.trace().is_empty());
        assert_eq!(
            b.transact(&call(&b, steps.clone()), deadline()).result,
            Err(Error::Denied)
        );
    }
}

#[test]
fn spd_aliases_limits_and_repeated_start_shapes_fail_closed() {
    for address in [0, 0x36, 0x37, 0x50, 0x57, 0x78, 0xa0] {
        for op in [
            Operation::SmbusWriteByte { address },
            Operation::SmbusWriteWordData {
                address,
                register: 0,
            },
            Operation::I2cWrite {
                address,
                stop: true,
            },
        ] {
            let n = if matches!(op, Operation::SmbusWriteWordData { .. }) {
                2
            } else {
                1
            };
            assert!(policy(bus_identity(), vec![write(op, &vec![0; n])])
                .validate()
                .is_err());
        }
    }
    assert!(policy(
        bus_identity(),
        vec![write(
            Operation::SmbusWriteBlockData {
                address: 0x60,
                register: 0
            },
            &[0; 33]
        )]
    )
    .validate()
    .is_err());
    assert!(policy(
        bus_identity(),
        vec![write(
            Operation::I2cWrite {
                address: 0x60,
                stop: false
            },
            &[0; 64]
        )]
    )
    .validate()
    .is_err());
    assert!(policy(
        bus_identity(),
        vec![
            write(
                Operation::I2cWrite {
                    address: 0x60,
                    stop: false
                },
                &[1]
            ),
            read(
                Operation::I2cRead {
                    address: 0x61,
                    stop: true
                },
                4
            )
        ]
    )
    .validate()
    .is_err());
    let mut p = policy(
        bus_identity(),
        vec![write(Operation::SmbusWriteByte { address: 0x60 }, &[1])],
    );
    p.recipes[0].masks.clear();
    assert!(p.validate().is_err());
}

#[test]
fn partial_failure_short_reply_timeout_replay_and_alias_quarantine() {
    let steps = vec![
        write(
            Operation::SmbusWriteWordData {
                address: 0x60,
                register: 0,
            },
            &[0x80, 0x10],
        ),
        read(
            Operation::SmbusReadByteData {
                address: 0x60,
                register: 0x81,
            },
            1,
        ),
    ];
    let p = policy(bus_identity(), steps.clone());
    for reply in [Err(Error::Nack), Ok(vec![])] {
        let bus = SimulatedBus::default();
        bus.script([Ok(vec![]), reply]);
        let mut b = Broker::new(p.clone(), bus_identity(), bus.clone()).unwrap();
        assert!(b
            .transact(&call(&b, steps.clone()), deadline())
            .result
            .is_err());
        assert_eq!(bus.trace(), steps);
        assert!(bus.is_faulted());
        let mut alias = Broker::new(p.clone(), bus_identity(), bus.clone()).unwrap();
        assert_eq!(
            alias
                .transact(&call(&alias, steps.clone()), deadline())
                .result,
            Err(Error::Faulted)
        );
        assert_eq!(bus.trace(), steps);
    }
    let bus = SimulatedBus::default();
    let mut b = Broker::new(p.clone(), bus_identity(), bus.clone()).unwrap();
    assert_eq!(
        b.transact(&call(&b, steps.clone()), Instant::now()).result,
        Err(Error::Timeout)
    );
    assert!(bus.trace().is_empty());
    let mut b = Broker::new(p, bus_identity(), bus.clone()).unwrap();
    let c = call(&b, steps);
    assert!(b.transact(&c, deadline()).result.is_ok());
    assert_eq!(b.transact(&c, deadline()).result, Err(Error::Denied));
}

fn temp(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "freergb-a7-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
fn manifest(p: Policy) -> PackageManifest {
    PackageManifest {
        schema: 4,
        id: "transport-fixture".into(),
        version: "0.0.1".into(),
        family: "fixture".into(),
        api: vec![API],
        target: host_target().into(),
        executable: format!("freergb-transport-fixture{}", std::env::consts::EXE_SUFFIX),
        license: "MIT".into(),
        devices: vec![],
        bus: None,
        cooling: None,
        transport: Some(p),
        probe: None,
        files: Default::default(),
    }
}
fn cases() -> Vec<Policy> {
    // Wire facts independently transcribed from the pinned OpenRGB sources:
    // ENESMBusInterface_i2c_smbus::ENERegisterRead selects 0x8010 with a
    // swapped SMBus word (wire bytes 80 10), then reads command 81.
    let ene = vec![
        write(
            Operation::SmbusWriteWordData {
                address: 0x60,
                register: 0,
            },
            &[0x80, 0x10],
        ),
        read(
            Operation::SmbusReadByteData {
                address: 0x60,
                register: 0x81,
            },
            1,
        ),
    ];
    // Blackwell packets are 64 raw bytes, never split/count-prefixed.
    let mut packet = vec![0; 64];
    packet[..2].copy_from_slice(&[0x10, 0x01]);
    let raw = vec![
        write(
            Operation::I2cWrite {
                address: 0x60,
                stop: true,
            },
            &packet,
        ),
        read(
            Operation::I2cRead {
                address: 0x60,
                stop: true,
            },
            4,
        ),
    ];
    // RGBFusionGPU at address 62: command 40, RGB, four zero padding bytes.
    let stream = [0x40, 1, 2, 3, 0, 0, 0, 0]
        .map(|b| write(Operation::SmbusWriteByte { address: 0x62 }, &[b]))
        .to_vec();
    let usb = Identity::Usb {
        vid: 0x1234,
        pid: 0x5678,
        interface: 2,
    };
    let mut result = vec![
        policy(bus_identity(), ene),
        policy(bus_identity(), raw),
        policy(bus_identity(), stream),
        policy(
            usb.clone(),
            vec![
                write(
                    Operation::UsbControl {
                        request_type: 0x41,
                        request: 9,
                        value: 1,
                        index: 2,
                    },
                    &[1, 2, 3],
                ),
                read(
                    Operation::UsbControl {
                        request_type: 0xc1,
                        request: 9,
                        value: 1,
                        index: 2,
                    },
                    3,
                ),
            ],
        ),
        policy(
            usb,
            vec![
                write(Operation::UsbBulk { endpoint: 2 }, &[1, 2, 3]),
                read(Operation::UsbBulk { endpoint: 0x82 }, 3),
            ],
        ),
        policy(
            Identity::Serial {
                vid: 0x1234,
                pid: 0x5678,
                com: 7,
                baud: 115200,
            },
            vec![
                write(Operation::SerialWrite, &[1, 2, 3]),
                read(Operation::SerialRead, 3),
            ],
        ),
    ];
    for protocol in [NetworkProtocol::Udp, NetworkProtocol::Tcp] {
        result.push(policy(
            Identity::Network {
                host: "192.0.2.7".parse().unwrap(),
                port: 1234,
                protocol,
            },
            vec![
                write(Operation::NetworkWrite, &[1, 2, 3]),
                read(Operation::NetworkRead, 3),
            ],
        ));
    }
    result
}

#[test]
fn signed_catalog_install_real_child_lifecycle_for_every_transport() {
    for p in cases() {
        p.validate().unwrap();
        let identity = p.identities[0].clone();
        let steps = p.recipes[0].steps.clone();
        let root = temp("signed");
        let package = root.join("fixture.zip");
        let m = manifest(p.clone());
        package::build(m.clone(),&[(m.executable.clone(),std::fs::read(env!("CARGO_BIN_EXE_freergb-transport-fixture")).unwrap()),
            ("LICENSE".into(),b"MIT synthetic fixture".to_vec()),("PROVENANCE.md".into(),b"Independent synthetic fixture; no family implementation. Owner acceptance: no hardware tested.".to_vec()),
            ("fixture-policy.json".into(),serde_json::to_vec(&p).unwrap())],&package).unwrap();
        let entry = ModuleEntry {
            id: m.id.clone(),
            version: m.version.clone(),
            family: m.family.clone(),
            name: "Fixture".into(),
            api: vec![API],
            license: m.license.clone(),
            devices: vec![],
            origin: Origin {
                project: "freeRGB".into(),
                url: "https://github.com/leotrax3d/freeRGB".into(),
                commit: "0000000".into(),
            },
            requires: vec![],
            artifacts: vec![Artifact {
                target: host_target().into(),
                url: "fixture.zip".into(),
                size: std::fs::metadata(&package).unwrap().len(),
                sha256: package::sha256_file(&package).unwrap(),
            }],
            source: Artifact {
                target: "source".into(),
                url: "source.zip".into(),
                size: 1,
                sha256: "0".repeat(64),
            },
        };
        let catalog = Catalog {
            schema: TRANSPORT_CATALOG_SCHEMA,
            sequence: 1,
            generated: String::new(),
            modules: vec![entry],
            relevant_vendors: vec![],
        };
        let key = SigningKey::from_bytes(&[17; 32]);
        let keys = TrustedKeys::default().with_development_key(key.verifying_key());
        let bytes = serde_json::to_vec(&catalog).unwrap();
        let signed = Catalog::verify(&bytes, sign(&key, &bytes).as_bytes(), &keys).unwrap();
        let store = Store::open(root.join("store")).unwrap();
        store
            .install_transport(&signed.modules[0], &package)
            .unwrap();
        assert!(store.verified(&m.id).is_err());
        assert!(store.verified_bus(&m.id).is_err());
        let bus = SimulatedBus::default();
        assert!(TransportProcess::start(
            store.verified_transport(&m.id).unwrap(),
            identity.clone(),
            bus.clone(),
            false
        )
        .is_err());
        assert!(bus.trace().is_empty());
        let (mut child, caps) = TransportProcess::start(
            store.verified_transport(&m.id).unwrap(),
            identity,
            bus.clone(),
            true,
        )
        .unwrap();
        assert_eq!(caps.layout.leds.len(), 1);
        child.frame(&[Rgb { r: 9, g: 8, b: 7 }]).unwrap();
        child.shutdown().unwrap();
        assert_eq!(
            bus.trace(),
            (0..3).flat_map(|_| steps.clone()).collect::<Vec<_>>()
        );
        assert!(!bus.is_faulted());
        // Installed signed policy cannot be widened without invalidating the manifest hash.
        let installed = store.verified_transport(&m.id).unwrap();
        drop(installed);
        let file = root
            .join("store")
            .join(&m.id)
            .join(&m.version)
            .join("module.json");
        let mut modified: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        modified["transport"]["backend"] = "native".into();
        std::fs::write(file, serde_json::to_vec(&modified).unwrap()).unwrap();
        assert!(store.verified_transport(&m.id).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn child_crash_hang_and_policy_violation_fault_the_bus() {
    for mode in ["crash", "hang", "deny"] {
        let root = temp(mode);
        let p = cases().remove(2);
        let identity = p.identities[0].clone();
        std::fs::write(
            root.join("fixture-policy.json"),
            serde_json::to_vec(&p).unwrap(),
        )
        .unwrap();
        std::fs::write(root.join("fixture-mode"), mode).unwrap();
        let mut m = manifest(p);
        m.files = [
            (m.executable.clone(), "0".repeat(64)),
            ("LICENSE".into(), "0".repeat(64)),
            ("PROVENANCE.md".into(), "0".repeat(64)),
        ]
        .into();
        let verified = VerifiedTransport::unpackaged(Verified {
            manifest: m,
            executable: env!("CARGO_BIN_EXE_freergb-transport-fixture").into(),
            directory: root.clone(),
        })
        .unwrap();
        let bus = SimulatedBus::default();
        let (mut child, _) =
            TransportProcess::start(verified, identity, bus.clone(), true).unwrap();
        let before = bus.trace();
        assert!(child.frame(&[Rgb { r: 1, g: 2, b: 3 }]).is_err());
        assert!(bus.is_faulted());
        assert_eq!(bus.trace(), before);
        assert!(child.frame(&[Rgb { r: 1, g: 2, b: 3 }]).is_err());
        drop(child);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn grouped_hid_real_child_claims_controls_and_releases_two_interfaces() {
    use freergb_device_api::{HidTransport, TransportError};
    use freergb_module_api::transport::hid::{Device, Hidpp, Interface, Operation as HidOperation};
    use freergb_modules::hid_group::{HidGroup, OpenedInterface};
    use std::sync::{Arc, Mutex};
    type Trace = Arc<Mutex<Vec<(u8, Vec<u8>)>>>;
    struct Mock(u8, Trace);
    impl HidTransport for Mock {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, TransportError> {
            self.1.lock().unwrap().push((self.0, bytes.into()));
            Ok(64) // HidTransport pads output to the handle's report length.
        }
        fn read_timeout(&mut self, _: &mut [u8], _: i32) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn output_report_len(&self) -> usize {
            64
        }
        fn input_report_len(&self) -> usize {
            64
        }
    }
    for mode in ["healthy", "no-claim", "no-release", "crash", "hang"] {
        let root = temp(mode);
        let interfaces = vec![
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
        ];
        let device = Device {
            vid: 0x046d,
            pid: 0xc547,
            interfaces: interfaces.clone(),
            hidpp: Some(Hidpp {
                device_index: 2,
                software_id: 7,
            }),
        };
        let p = Policy {
            backend: "hid".into(),
            identities: vec![Identity::HidGroup {
                device: device.clone(),
            }],
            subnets: vec![],
            recipes: ["claim", "initialize", "frame", "shutdown", "release"]
                .into_iter()
                .enumerate()
                .map(|(n, id)| {
                    let steps: Vec<_> = (0..2)
                        .map(|interface| {
                            let mut data = vec![0; 20];
                            data[..5].copy_from_slice(&[0x11, 2, 0x0a, 0x17, n as u8]);
                            write(
                                Operation::Hid {
                                    interface,
                                    operation: HidOperation::Write,
                                },
                                &data,
                            )
                        })
                        .collect();
                    Recipe {
                        id: id.into(),
                        masks: steps.iter().map(|s| vec![0; s.data.len()]).collect(),
                        steps,
                    }
                })
                .collect(),
        };
        p.validate().unwrap();
        std::fs::write(
            root.join("fixture-policy.json"),
            serde_json::to_vec(&p).unwrap(),
        )
        .unwrap();
        std::fs::write(root.join("fixture-mode"), mode).unwrap();
        let trace = Arc::new(Mutex::new(Vec::new()));
        let group = HidGroup::new(
            interfaces
                .into_iter()
                .enumerate()
                .map(|(slot, interface)| OpenedInterface {
                    physical_parent: root.to_string_lossy().into_owned(),
                    vid: device.vid,
                    pid: device.pid,
                    interface,
                    transport: Box::new(Mock(slot as u8, trace.clone())),
                })
                .collect(),
        )
        .unwrap();
        let mut m = manifest(p);
        m.files = [
            (m.executable.clone(), "0".repeat(64)),
            ("LICENSE".into(), "0".repeat(64)),
            ("PROVENANCE.md".into(), "0".repeat(64)),
        ]
        .into();
        let verified = VerifiedTransport::unpackaged(Verified {
            manifest: m,
            executable: env!("CARGO_BIN_EXE_freergb-transport-fixture").into(),
            directory: root.clone(),
        })
        .unwrap();
        let started = TransportProcess::start_hid(verified, device, group.clone(), true);
        if mode == "no-claim" {
            assert!(started.is_err());
            assert!(trace.lock().unwrap().is_empty());
            assert!(group.is_faulted());
        } else {
            let (mut child, _) = started.unwrap();
            let frame = child.frame(&[Rgb { r: 1, g: 2, b: 3 }]);
            if matches!(mode, "crash" | "hang") {
                assert!(frame.is_err());
                assert!(group.is_faulted());
                assert_eq!(trace.lock().unwrap().len(), 4); // Claim + initialization only.
            } else {
                frame.unwrap();
                let shutdown = child.shutdown();
                if mode == "no-release" {
                    assert!(shutdown.is_err());
                    assert!(group.is_faulted());
                } else {
                    shutdown.unwrap();
                    assert!(!group.is_faulted());
                    let calls = trace.lock().unwrap();
                    assert_eq!(calls.len(), 10);
                    for (n, pair) in calls.as_chunks::<2>().0.iter().enumerate() {
                        assert_eq!((pair[0].0, pair[1].0), (0, 1));
                        assert_eq!(&pair[0].1[..5], &[0x11, 2, 0x0a, 0x17, n as u8]);
                    }
                }
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
