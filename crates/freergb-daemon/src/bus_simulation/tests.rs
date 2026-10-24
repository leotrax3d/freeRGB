use super::*;
use freergb_module_api::bus::{Operation, Rule};

fn pci() -> PciId {
    PciId {
        ven: 0x10de,
        dev: 1,
        subven: 0x1458,
        subdev: 2,
    }
}
fn hello() -> Hello {
    Hello {
        v: API,
        api: vec![API],
        transport: "simulation".into(),
    }
}
fn policy() -> Policy {
    Policy {
        pci: vec![pci()],
        backend: "simulation".into(),
        flat_registers: true,
        rules: [
            Operation::SmbusReadByteData,
            Operation::SmbusWriteByteData,
            Operation::SmbusReadBlockData,
            Operation::SmbusWriteBlockData,
            Operation::I2cReadRegister,
            Operation::I2cWriteRegister,
        ]
        .into_iter()
        .map(|op| Rule {
            address: 0x18,
            first: 0,
            last: 255,
            op,
            max_len: if op.byte() { 1 } else { 32 },
            write_mask: 255,
            write_value: 0,
        })
        .collect(),
    }
}
fn bus() -> Arc<SimulatedBus> {
    let mut registers = RegisterBus::default();
    let mut values = [0; 256];
    values[0x20..0x23].copy_from_slice(&[0xaa, 0xbb, 0xcc]);
    registers.attach(0x18, values);
    SimulatedBus::new(registers)
}
fn transfer(op: Operation, register: u8, bytes: &[u8]) -> Transfer {
    Transfer {
        op,
        address: 0x18,
        register,
        data: if op.writes() { bytes.to_vec() } else { vec![] },
        read_len: if op.writes() { 0 } else { bytes.len() as u8 },
    }
}
fn call(session: &Session, transfers: Vec<Transfer>) -> Call {
    Call {
        v: API,
        call: 7,
        capability: session.capability(),
        transfers,
    }
}
fn execute(session: &Session, transfers: Vec<Transfer>) -> Result<Vec<Vec<u8>>, Error> {
    session
        .call(&call(session, transfers), &AtomicBool::new(false))
        .result
}

#[test]
fn golden_wire_traces_for_all_six_operations() {
    let bus = bus();
    let session = bus.session(&hello(), policy(), pci(), true).unwrap();
    let operations = vec![
        transfer(Operation::SmbusReadByteData, 0x20, &[0]),
        transfer(Operation::SmbusReadBlockData, 0x20, &[0; 3]),
        transfer(Operation::I2cReadRegister, 0x20, &[0; 3]),
        transfer(Operation::SmbusWriteByteData, 0x20, &[1]),
        transfer(Operation::SmbusWriteBlockData, 0x21, &[2, 3]),
        transfer(Operation::I2cWriteRegister, 0x23, &[4, 5]),
        transfer(Operation::I2cReadRegister, 0x20, &[0; 5]),
    ];
    assert_eq!(
        execute(&session, operations).unwrap(),
        vec![
            vec![0xaa],
            vec![0xaa, 0xbb, 0xcc],
            vec![0xaa, 0xbb, 0xcc],
            vec![],
            vec![],
            vec![],
            vec![1, 2, 3, 4, 5]
        ]
    );
    assert_eq!(
        bus.trace(),
        vec![
            vec![0x30, 0x20, 0x31, 0xaa],
            vec![0x30, 0x20, 0x31, 3, 0xaa, 0xbb, 0xcc],
            vec![0x30, 0x20, 0x31, 0xaa, 0xbb, 0xcc],
            vec![0x30, 0x20, 1],
            vec![0x30, 0x21, 2, 2, 3],
            vec![0x30, 0x23, 4, 5],
            vec![0x30, 0x20, 0x31, 1, 2, 3, 4, 5]
        ]
    );
}

#[test]
fn bounded_json_dispatch_rejects_unknown_and_oversized_input() {
    let bus = bus();
    let session = bus.session(&hello(), policy(), pci(), true).unwrap();
    let request = call(
        &session,
        vec![transfer(Operation::SmbusReadByteData, 0x20, &[0])],
    );
    let bytes = serde_json::to_vec(&request).unwrap();
    let reply = session.call_json(&bytes, &AtomicBool::new(false)).unwrap();
    assert_eq!(reply.call, 7);
    assert_eq!(reply.result, Ok(vec![vec![0xaa]]));
    let mut value = serde_json::to_value(&request).unwrap();
    value["bus"] = serde_json::json!("other-device");
    assert_eq!(
        session.call_json(
            &serde_json::to_vec(&value).unwrap(),
            &AtomicBool::new(false)
        ),
        Err(Error::Denied)
    );
    assert_eq!(
        session.call_json(
            &vec![b' '; freergb_module_api::MAX_MESSAGE + 1],
            &AtomicBool::new(false)
        ),
        Err(Error::Denied)
    );
    assert_eq!(bus.trace().len(), 1);
}

#[test]
fn forbidden_writes_never_dispatch_even_with_malicious_rules() {
    for address in [0x36, 0x37, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57] {
        for op in [
            Operation::SmbusWriteByteData,
            Operation::SmbusWriteBlockData,
            Operation::I2cWriteRegister,
        ] {
            let mut policy = policy();
            for rule in &mut policy.rules {
                rule.address = address;
            }
            assert_eq!(policy.validate(), Err(Error::InvalidPolicy));
            let mut request = transfer(op, 0, &[1]);
            request.address = address;
            assert_eq!(policy.authorize(&request), Err(Error::Denied));
        }
    }
}

#[test]
fn bad_batch_is_rejected_before_first_write() {
    let bus = bus();
    let session = bus.session(&hello(), policy(), pci(), true).unwrap();
    let write = transfer(Operation::I2cWriteRegister, 0x20, &[5]);
    for bad in [
        Transfer {
            address: 0x50,
            ..write.clone()
        },
        transfer(Operation::I2cReadRegister, 255, &[0; 2]),
        transfer(Operation::I2cWriteRegister, 0, &[0; 33]),
        Transfer {
            read_len: 1,
            ..write.clone()
        },
        Transfer {
            address: 0xa0,
            ..write.clone()
        },
    ] {
        assert_eq!(
            execute(&session, vec![write.clone(), bad]),
            Err(Error::Denied)
        );
        assert!(bus.trace().is_empty());
    }
}

#[test]
fn exact_pci_consent_version_capability_and_revocation() {
    let bus = bus();
    assert!(matches!(
        bus.session(&hello(), policy(), pci(), false),
        Err(Error::Denied)
    ));
    assert!(matches!(
        bus.session(&hello(), policy(), PciId { subdev: 3, ..pci() }, true),
        Err(Error::Denied)
    ));
    assert!(matches!(
        bus.session(&Hello { v: 1, ..hello() }, policy(), pci(), true),
        Err(Error::Unsupported)
    ));
    let session = bus.session(&hello(), policy(), pci(), true).unwrap();
    let mut request = call(
        &session,
        vec![transfer(Operation::SmbusReadByteData, 0x20, &[0])],
    );
    request.capability += 1;
    assert_eq!(
        session.call(&request, &AtomicBool::new(false)).result,
        Err(Error::Denied)
    );
    session.revoke();
    assert_eq!(execute(&session, request.transfers), Err(Error::Denied));
    assert!(bus.trace().is_empty());
}

#[test]
fn register_ranges_and_value_masks_cannot_be_combined_to_widen_access() {
    let mut policy = policy();
    policy.rules.retain(|r| r.op == Operation::I2cWriteRegister);
    policy.rules[0].first = 0x20;
    policy.rules[0].last = 0x21;
    policy.rules[0].write_mask = 0x0f;
    policy.rules[0].write_value = 0xa0;
    policy.validate().unwrap();
    assert_eq!(
        policy.authorize(&transfer(Operation::I2cWriteRegister, 0x20, &[0xa5, 0xaf])),
        Ok(())
    );
    for request in [
        transfer(Operation::I2cWriteRegister, 0x20, &[0xb5]),
        transfer(Operation::I2cWriteRegister, 0x21, &[0xa5, 0xaf]),
        transfer(Operation::I2cWriteRegister, 0x1f, &[0xa5]),
    ] {
        assert_eq!(policy.authorize(&request), Err(Error::Denied));
    }
}

#[test]
fn transport_failure_quarantines_bus_and_never_retries() {
    for error in [Error::Nack, Error::Timeout, Error::Disconnected] {
        let bus = bus();
        let session = bus.session(&hello(), policy(), pci(), true).unwrap();
        bus.registers.lock().unwrap().fail_next(error);
        let write = transfer(Operation::I2cWriteRegister, 0, &[5]);
        assert_eq!(execute(&session, vec![write.clone()]), Err(error));
        assert_eq!(execute(&session, vec![write.clone()]), Err(Error::Denied));
        let other = bus.session(&hello(), policy(), pci(), true).unwrap();
        assert_eq!(execute(&other, vec![write]), Err(Error::Faulted));
        assert!(bus.trace().is_empty());
    }
}

#[test]
fn partial_batch_is_neither_replayed_nor_blindly_restored() {
    let bus = bus();
    let session = bus.session(&hello(), policy(), pci(), true).unwrap();
    bus.registers.lock().unwrap().fail_after(1, Error::Timeout);
    assert_eq!(
        execute(
            &session,
            vec![
                transfer(Operation::I2cWriteRegister, 0x20, &[1]),
                transfer(Operation::I2cWriteRegister, 0x21, &[2])
            ]
        ),
        Err(Error::Timeout)
    );
    assert_eq!(bus.trace(), vec![vec![0x30, 0x20, 1]]);
    let registers = bus.registers.lock().unwrap();
    assert_eq!(registers.devices[&0x18][0x20], 1);
    assert_eq!(registers.devices[&0x18][0x21], 0xbb);
}

#[test]
fn unwind_quarantines_and_releases_admission() {
    let bus = bus();
    assert!(std::panic::catch_unwind(|| {
        let _held = bus
            .admit(
                Instant::now() + Duration::from_secs(1),
                &AtomicBool::new(false),
                &AtomicBool::new(false),
            )
            .unwrap();
        panic!("simulated mutation panic");
    })
    .is_err());
    assert!(matches!(
        bus.admit(
            Instant::now() + Duration::from_secs(1),
            &AtomicBool::new(false),
            &AtomicBool::new(false)
        ),
        Err(Error::Faulted)
    ));
    assert!(!bus.admission.lock().unwrap().active);
}

#[test]
fn queued_cancellation_and_timeout_do_not_stall_fifo() {
    let bus = bus();
    let held = bus
        .admit(
            Instant::now() + Duration::from_secs(2),
            &AtomicBool::new(false),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(matches!(
        bus.admit(
            Instant::now(),
            &AtomicBool::new(false),
            &AtomicBool::new(false)
        ),
        Err(Error::Timeout)
    ));
    assert!(matches!(
        bus.admit(
            Instant::now() + Duration::from_secs(1),
            &AtomicBool::new(true),
            &AtomicBool::new(false)
        ),
        Err(Error::Cancelled)
    ));
    assert!(bus.admission.lock().unwrap().queue.is_empty());
    drop(held);
    let session = bus.session(&hello(), policy(), pci(), true).unwrap();
    assert!(execute(
        &session,
        vec![transfer(Operation::SmbusReadByteData, 0, &[0])]
    )
    .is_ok());
}

#[test]
fn revocation_wakes_a_queued_session_without_faulting_other_sessions() {
    let bus = bus();
    let session = Arc::new(bus.session(&hello(), policy(), pci(), true).unwrap());
    let held = bus
        .admit(
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
            &AtomicBool::new(false),
        )
        .unwrap();
    let worker_session = session.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        tx.send(execute(
            &worker_session,
            vec![transfer(Operation::I2cWriteRegister, 0, &[5])],
        ))
        .unwrap()
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while bus.admission.lock().unwrap().queue.is_empty() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    session.revoke();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        Err(Error::Cancelled)
    );
    worker.join().unwrap();
    drop(held);
    assert!(bus.trace().is_empty());
    let other = bus.session(&hello(), policy(), pci(), true).unwrap();
    assert!(execute(
        &other,
        vec![transfer(Operation::SmbusReadByteData, 0, &[0])]
    )
    .is_ok());
}

#[test]
fn sessions_share_fifo_and_batches_do_not_interleave() {
    let bus = bus();
    let held = bus
        .admit(
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
            &AtomicBool::new(false),
        )
        .unwrap();
    let mut workers = vec![];
    for id in 0..8 {
        let session = bus.session(&hello(), policy(), pci(), true).unwrap();
        workers.push(std::thread::spawn(move || {
            execute(
                &session,
                vec![
                    transfer(Operation::I2cWriteRegister, id, &[id]),
                    transfer(Operation::I2cWriteRegister, id, &[id]),
                ],
            )
            .unwrap()
        }));
        let deadline = Instant::now() + Duration::from_secs(5);
        while bus.admission.lock().unwrap().queue.len() != usize::from(id) + 1 {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    drop(held);
    for worker in workers {
        worker.join().unwrap();
    }
    let expected: Vec<_> = (0..8)
        .flat_map(|id| [vec![0x30, id, id], vec![0x30, id, id]])
        .collect();
    assert_eq!(bus.trace(), expected);
}
