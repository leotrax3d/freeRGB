use super::*;
use crate::bus_simulation::{Bus, Session};
use freergb_module_api::bus::{Call, Hello, Operation, PciId, Policy, Rule, API};
use freergb_modules::bus_host::Transport;
use std::sync::Mutex;
use std::time::Duration;

fn pci() -> PciId {
    PciId {
        ven: 1,
        dev: 2,
        subven: 3,
        subdev: 4,
    }
}
fn policy(first: u8, last: u8) -> Policy {
    Policy {
        pci: vec![pci()],
        backend: "windows-driver".into(),
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
            first,
            last,
            op,
            max_len: if op.byte() { 1 } else { 32 },
            write_mask: 0x0f,
            write_value: 0xa0,
        })
        .collect(),
    }
}
fn transfer(op: Operation, register: u8) -> Transfer {
    Transfer {
        op,
        address: 0x18,
        register,
        data: if op.writes() { vec![0xa1] } else { vec![] },
        read_len: if op.writes() { 0 } else { 1 },
    }
}
fn session(bus: &Arc<Bus<NativeBackend>>, first: u8, last: u8) -> Session<NativeBackend> {
    bus.session(
        &Hello {
            v: API,
            api: vec![API],
            transport: "native".into(),
        },
        policy(first, last),
        pci(),
        true,
    )
    .unwrap()
}
fn call(session: &Session<NativeBackend>, transfers: Vec<Transfer>) -> Call {
    Call {
        v: API,
        call: 1,
        capability: session.capability(),
        transfers,
    }
}

type Trace = Arc<Mutex<Vec<(Operation, u8, u8, Vec<u8>)>>>;
struct Driver {
    trace: Trace,
}
impl Driver {
    fn record(&self, op: Operation, a: u8, r: u8, data: &[u8]) {
        self.trace.lock().unwrap().push((op, a, r, data.to_vec()));
    }
}
impl windows::DriverInterface for Driver {
    fn read_byte(&mut self, a: u8, r: u8) -> Result<u8, Error> {
        self.record(Operation::SmbusReadByteData, a, r, &[]);
        Ok(7)
    }
    fn write_byte(&mut self, a: u8, r: u8, v: u8) -> Result<(), Error> {
        self.record(Operation::SmbusWriteByteData, a, r, &[v]);
        Ok(())
    }
    fn read_block(&mut self, a: u8, r: u8, n: u8) -> Result<Vec<u8>, Error> {
        self.record(Operation::SmbusReadBlockData, a, r, &[]);
        Ok(vec![7; n.into()])
    }
    fn write_block(&mut self, a: u8, r: u8, d: &[u8]) -> Result<(), Error> {
        self.record(Operation::SmbusWriteBlockData, a, r, d);
        Ok(())
    }
    fn i2c_read(&mut self, a: u8, r: u8, n: u8) -> Result<Vec<u8>, Error> {
        self.record(Operation::I2cReadRegister, a, r, &[]);
        Ok(vec![7; n.into()])
    }
    fn i2c_write(&mut self, a: u8, r: u8, d: &[u8]) -> Result<(), Error> {
        self.record(Operation::I2cWriteRegister, a, r, d);
        Ok(())
    }
}
fn fixture() -> (Arc<Bus<NativeBackend>>, Trace) {
    let trace = Arc::new(Mutex::new(vec![]));
    let backend = NativeBackend::windows(Driver {
        trace: trace.clone(),
    })
    .unwrap();
    (Bus::new(backend), trace)
}

#[test]
fn windows_driver_dispatches_all_six_operations_through_api2() {
    let (bus, trace) = fixture();
    let s = session(&bus, 0x20, 0x30);
    let operations = [
        Operation::SmbusReadByteData,
        Operation::SmbusWriteByteData,
        Operation::SmbusReadBlockData,
        Operation::SmbusWriteBlockData,
        Operation::I2cReadRegister,
        Operation::I2cWriteRegister,
    ];
    let request = call(
        &s,
        operations.iter().map(|op| transfer(*op, 0x20)).collect(),
    );
    let response = s.call(&request, &AtomicBool::new(false));
    assert_eq!(
        response.result.unwrap(),
        vec![vec![7], vec![], vec![7], vec![], vec![7], vec![]]
    );
    assert_eq!(
        *trace.lock().unwrap(),
        operations
            .iter()
            .map(|op| (
                *op,
                0x18,
                0x20,
                if op.writes() { vec![0xa1] } else { vec![] }
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn per_module_rules_preflight_entire_batch_and_capabilities() {
    let (bus, trace) = fixture();
    let a = session(&bus, 0x20, 0x20);
    let b = session(&bus, 0x30, 0x30);
    let read = transfer(Operation::I2cReadRegister, 0x20);
    let denied = transfer(Operation::I2cReadRegister, 0x30);
    assert_eq!(
        a.call(
            &call(&a, vec![read.clone(), denied]),
            &AtomicBool::new(false)
        )
        .result,
        Err(Error::Denied)
    );
    assert_eq!(
        b.call(&call(&a, vec![read.clone()]), &AtomicBool::new(false))
            .result,
        Err(Error::Denied)
    );
    let mut write = transfer(Operation::I2cWriteRegister, 0x20);
    write.data = vec![0xb1];
    assert_eq!(
        a.call(&call(&a, vec![write]), &AtomicBool::new(false))
            .result,
        Err(Error::Denied)
    );
    let mut address = read.clone();
    address.address = 0x19;
    assert_eq!(
        a.call(&call(&a, vec![address]), &AtomicBool::new(false))
            .result,
        Err(Error::Denied)
    );
    assert!(trace.lock().unwrap().is_empty());
    assert!(a
        .call(&call(&a, vec![read]), &AtomicBool::new(false))
        .result
        .is_ok());
}

#[test]
fn native_requires_consent_and_matching_backend() {
    let (bus, _) = fixture();
    let hello = Hello {
        v: API,
        api: vec![API],
        transport: "native".into(),
    };
    assert!(bus.session(&hello, policy(0, 255), pci(), false).is_err());
    let mut wrong = policy(0, 255);
    wrong.backend = "linux-i2c-dev".into();
    assert!(bus.session(&hello, wrong, pci(), true).is_err());
}

struct Blocking {
    entered: mpsc::SyncSender<()>,
    release: mpsc::Receiver<()>,
}
impl Device for Blocking {
    fn transfer(&mut self, _: &Transfer) -> Result<Vec<u8>, Error> {
        self.entered.send(()).unwrap();
        self.release.recv().unwrap();
        Ok(vec![7])
    }
}
#[test]
fn kernel_timeout_returns_and_quarantines_all_sessions_without_retry() {
    let (entered, seen) = mpsc::sync_channel(1);
    let (release, wait) = mpsc::sync_channel(1);
    let backend = NativeBackend::spawn(
        "windows-driver",
        Blocking {
            entered,
            release: wait,
        },
    )
    .unwrap();
    let bus = Bus::new(backend);
    let mut a = session(&bus, 0, 255);
    let b = session(&bus, 0, 255);
    let request = call(&a, vec![transfer(Operation::I2cReadRegister, 0x20)]);
    let start = Instant::now();
    assert_eq!(
        a.transact(&request, start + Duration::from_millis(40))
            .result,
        Err(Error::Timeout)
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    seen.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        b.call(&call(&b, request.transfers), &AtomicBool::new(false))
            .result,
        Err(Error::Faulted)
    );
    release.send(()).unwrap();
}

struct Broken;
impl Device for Broken {
    fn transfer(&mut self, _: &Transfer) -> Result<Vec<u8>, Error> {
        Ok(vec![0; 33])
    }
}
#[test]
fn malformed_native_response_faults_bus() {
    let bus = Bus::new(NativeBackend::spawn("windows-driver", Broken).unwrap());
    let s = session(&bus, 0, 255);
    let other = session(&bus, 0, 255);
    let request = call(&s, vec![transfer(Operation::I2cReadRegister, 0)]);
    assert_eq!(
        s.call(&request, &AtomicBool::new(false)).result,
        Err(Error::Faulted)
    );
    assert_eq!(
        s.call(&request, &AtomicBool::new(false)).result,
        Err(Error::Denied)
    );
    assert_eq!(
        other
            .call(&call(&other, request.transfers), &AtomicBool::new(false))
            .result,
        Err(Error::Faulted)
    );
}
