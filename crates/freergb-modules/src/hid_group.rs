//! Host-owned HID interface groups. Construction never discovers or opens hardware.
//! Discovery must supply a connection-scoped physical parent ID (container ID on
//! Windows, USB parent on Linux), not a VID/PID or a product/serial string.
//! Clones share a worker and quarantine flag, serializing whole recipes across
//! receiver slots. A timed-out native call is never followed by another call.
use freergb_device_api::{HidTransport, TransportError};
use freergb_module_api::bus::Error;
use freergb_module_api::transport::hid::{Device, Interface, Operation};
use freergb_module_api::transport::{Operation as TransportOperation, Step};
use freergb_module_api::MAX_REPORT;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;

/// Trusted discovery metadata and the daemon-owned handle for one interface.
pub struct OpenedInterface {
    pub physical_parent: String,
    pub vid: u16,
    pub pid: u16,
    pub interface: Interface,
    pub transport: Box<dyn HidTransport>,
}

struct Job {
    device: Device,
    steps: Vec<Step>,
    deadline: Instant,
    reply: mpsc::SyncSender<Result<Vec<Vec<u8>>, Error>>,
}

struct Shared {
    device: Device,
    jobs: mpsc::SyncSender<Job>,
    faulted: Arc<AtomicBool>,
    leases: Mutex<BTreeMap<u8, u64>>,
}

#[derive(Clone)]
pub struct HidGroup(Arc<Shared>);

impl HidGroup {
    pub fn new(interfaces: Vec<OpenedInterface>) -> Result<Self, Error> {
        // Keep the reservation alive until the worker releases its handles,
        // even if a caller drops the last group while a native call is blocked.
        static PARENTS: Mutex<Vec<(String, std::sync::Weak<AtomicBool>)>> = Mutex::new(Vec::new());
        let first = interfaces.first().ok_or(Error::InvalidPolicy)?;
        let device = Device {
            vid: first.vid,
            pid: first.pid,
            interfaces: interfaces.iter().map(|i| i.interface.clone()).collect(),
            hidpp: None,
        };
        if first.physical_parent.is_empty()
            || !device.valid()
            || interfaces.iter().any(|i| {
                i.physical_parent != first.physical_parent
                    || i.vid != first.vid
                    || i.pid != first.pid
                    || i.transport.output_report_len() > MAX_REPORT
                    || i.transport.input_report_len() > MAX_REPORT
            })
        {
            return Err(Error::InvalidPolicy);
        }
        let mut parents = PARENTS.lock().map_err(|_| Error::Faulted)?;
        parents.retain(|(_, worker)| worker.strong_count() > 0);
        if parents
            .iter()
            .any(|(parent, _)| parent == &first.physical_parent)
        {
            return Err(Error::Busy);
        }
        let parent = first.physical_parent.clone();
        let faulted = Arc::new(AtomicBool::new(false));
        let worker_fault = faulted.clone();
        let (jobs, receiver) = mpsc::sync_channel::<Job>(32);
        std::thread::Builder::new()
            .name("module-hid-group".into())
            .spawn(move || {
                let mut handles: Vec<_> = interfaces.into_iter().map(|i| i.transport).collect();
                for job in receiver {
                    let result = execute(&mut handles, &worker_fault, &job);
                    if result.is_err() {
                        worker_fault.store(true, Ordering::Release);
                    }
                    let _ = job.reply.send(result);
                }
            })
            .map_err(|_| Error::Faulted)?;
        parents.push((parent, Arc::downgrade(&faulted)));
        Ok(Self(Arc::new(Shared {
            device,
            jobs,
            faulted,
            leases: Mutex::new(BTreeMap::new()),
        })))
    }

    pub fn is_faulted(&self) -> bool {
        self.0.faulted.load(Ordering::Acquire)
    }

    pub(crate) fn fault(&self) {
        self.0.faulted.store(true, Ordering::Release);
    }

    fn index(device: &Device) -> u8 {
        device.hidpp.as_ref().map_or(0, |h| h.device_index)
    }

    pub(crate) fn reserve(&self, device: &Device, capability: u64) -> Result<(), Error> {
        if !device.valid()
            || device.vid != self.0.device.vid
            || device.pid != self.0.device.pid
            || device.interfaces != self.0.device.interfaces
            || self.is_faulted()
        {
            return Err(Error::Denied);
        }
        let mut leases = self.0.leases.lock().map_err(|_| Error::Faulted)?;
        let index = Self::index(device);
        // A generic logical device owns the complete group; receiver slots may
        // coexist only with other explicitly addressed receiver slots.
        if leases.contains_key(&index)
            || leases.contains_key(&0)
            || leases.contains_key(&0xff)
            || (matches!(index, 0 | 0xff) && !leases.is_empty())
        {
            return Err(Error::Busy);
        }
        leases.insert(index, capability);
        Ok(())
    }

    pub(crate) fn release(&self, device: &Device, capability: u64) {
        if let Ok(mut leases) = self.0.leases.lock() {
            let index = Self::index(device);
            if leases.get(&index) == Some(&capability) {
                leases.remove(&index);
            }
        }
    }

    pub(crate) fn transact(
        &self,
        device: &Device,
        steps: &[Step],
        deadline: Instant,
    ) -> Result<Vec<Vec<u8>>, Error> {
        if self.is_faulted() {
            return Err(Error::Faulted);
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        let (reply, receive) = mpsc::sync_channel(1);
        self.0
            .jobs
            .try_send(Job {
                device: device.clone(),
                steps: steps.to_vec(),
                deadline,
                reply,
            })
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => Error::Busy,
                _ => Error::Disconnected,
            })?;
        match receive.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(result) => result,
            Err(_) => {
                self.fault();
                Err(Error::Timeout)
            }
        }
    }
}

fn transport_error(error: TransportError) -> Error {
    match error {
        TransportError::Timeout => Error::Timeout,
        TransportError::Disconnected => Error::Disconnected,
        _ => Error::Faulted,
    }
}

fn execute(
    handles: &mut [Box<dyn HidTransport>],
    faulted: &AtomicBool,
    job: &Job,
) -> Result<Vec<Vec<u8>>, Error> {
    // Preflight every operation, including handle report lengths, before I/O.
    let mut last_headers = vec![None; handles.len()];
    let mut expected_headers = Vec::with_capacity(job.steps.len());
    for step in &job.steps {
        let TransportOperation::Hid {
            interface,
            operation,
        } = &step.operation
        else {
            return Err(Error::Denied);
        };
        let handle = handles.get(usize::from(*interface)).ok_or(Error::Denied)?;
        let limit = match operation {
            Operation::Write => handle.output_report_len(),
            Operation::Read { .. } | Operation::ReadReply { .. } => handle.input_report_len(),
            _ => MAX_REPORT,
        };
        if step.len() > limit {
            return Err(Error::Denied);
        }
        let slot = usize::from(*interface);
        let header = match operation {
            Operation::Write | Operation::SendFeature if job.device.hidpp.is_some() => {
                last_headers[slot] = Some(
                    <[u8; 4]>::try_from(step.data.get(..4).ok_or(Error::Denied)?)
                        .map_err(|_| Error::Denied)?,
                );
                None
            }
            Operation::Read {
                report_id,
                feature,
                function,
            } => job
                .device
                .hidpp
                .as_ref()
                .map(|h| [*report_id, h.device_index, *feature, *function]),
            Operation::ReadReply { report_id } => {
                let header = last_headers[slot].take().ok_or(Error::Denied)?;
                if header[0] != *report_id {
                    return Err(Error::Denied);
                }
                Some(header)
            }
            _ => None,
        };
        expected_headers.push(header);
    }
    let mut replies = Vec::with_capacity(job.steps.len());
    for (step, expected_header) in job.steps.iter().zip(expected_headers) {
        if faulted.load(Ordering::Acquire) {
            return Err(Error::Faulted);
        }
        if Instant::now() >= job.deadline {
            return Err(Error::Timeout);
        }
        let TransportOperation::Hid {
            interface,
            operation,
        } = &step.operation
        else {
            unreachable!()
        };
        let handle = &mut handles[usize::from(*interface)];
        let reply = match operation {
            Operation::Write | Operation::SendFeature => {
                let n = if matches!(operation, Operation::Write) {
                    handle.write(&step.data)
                } else {
                    handle.send_feature(&step.data)
                }
                .map_err(transport_error)?;
                let expected = if matches!(operation, Operation::Write) {
                    handle.output_report_len()
                } else {
                    step.data.len()
                };
                if n != expected {
                    return Err(Error::Faulted);
                }
                Vec::new()
            }
            Operation::GetFeature { report_id } => {
                let mut bytes = vec![0; step.len()];
                bytes[0] = *report_id;
                let n = handle.get_feature(&mut bytes).map_err(transport_error)?;
                if n != bytes.len() || bytes[0] != *report_id {
                    return Err(Error::Faulted);
                }
                bytes
            }
            Operation::Read { report_id, .. } | Operation::ReadReply { report_id } => {
                let mut bytes = vec![0; handle.input_report_len().max(1)];
                let mut matched = false;
                // Ignore bounded unrelated receiver notifications, never expose
                // another slot's data, and never treat a zero read as success.
                for _ in 0..32 {
                    if faulted.load(Ordering::Acquire) {
                        return Err(Error::Faulted);
                    }
                    let remaining = job.deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Err(Error::Timeout);
                    }
                    let ms = remaining.as_millis().clamp(1, 100) as i32;
                    let n = handle
                        .read_timeout(&mut bytes, ms)
                        .map_err(transport_error)?;
                    if n == 0 {
                        continue;
                    }
                    if n > bytes.len() {
                        return Err(Error::Faulted);
                    }
                    if let Some(header) = expected_header {
                        if n < 4 || bytes[..4] != header {
                            continue;
                        }
                    } else if bytes[0] != *report_id {
                        continue;
                    }
                    if n != step.len() {
                        return Err(Error::Faulted);
                    }
                    matched = true;
                    break;
                }
                if !matched {
                    return Err(Error::Timeout);
                }
                bytes.truncate(step.len());
                bytes
            }
        };
        if faulted.load(Ordering::Acquire) {
            return Err(Error::Faulted);
        }
        if Instant::now() >= job.deadline {
            return Err(Error::Timeout);
        }
        replies.push(reply);
    }
    Ok(replies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport_host::Broker;
    use freergb_module_api::transport::hid::Hidpp;
    use freergb_module_api::transport::{Call, Identity, Policy, Recipe, API};
    use std::collections::VecDeque;
    use std::time::Duration;

    #[derive(Default)]
    struct Wire {
        trace: Vec<(u8, Vec<u8>)>,
        reads: VecDeque<Vec<u8>>,
        short: bool,
        pause: Option<(mpsc::SyncSender<()>, mpsc::Receiver<()>)>,
        stopped: Option<mpsc::SyncSender<()>>,
    }
    struct Mock(u8, Arc<Mutex<Wire>>);
    impl HidTransport for Mock {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, TransportError> {
            let mut wire = self.1.lock().unwrap();
            wire.trace.push((self.0, bytes.into()));
            let short = wire.short;
            let pause = wire.pause.take();
            drop(wire);
            if let Some((entered, resume)) = pause {
                entered.send(()).unwrap();
                resume.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            Ok(if short {
                bytes.len() - 1
            } else {
                self.output_report_len()
            })
        }
        fn read_timeout(&mut self, buffer: &mut [u8], _: i32) -> Result<usize, TransportError> {
            let bytes = self.1.lock().unwrap().reads.pop_front().unwrap_or_default();
            let n = bytes.len().min(buffer.len());
            buffer[..n].copy_from_slice(&bytes[..n]);
            Ok(n)
        }
        fn output_report_len(&self) -> usize {
            64
        }
        fn input_report_len(&self) -> usize {
            64
        }
    }
    impl Drop for Mock {
        fn drop(&mut self) {
            if self.0 == 2 {
                if let Some(stopped) = self.1.lock().unwrap().stopped.take() {
                    let _ = stopped.send(());
                }
            }
        }
    }
    fn descriptor(number: i32) -> Interface {
        Interface {
            number,
            usage_page: 0xff00,
            usage: 1,
        }
    }
    fn opened(parent: &str, number: u8, wire: &Arc<Mutex<Wire>>) -> OpenedInterface {
        OpenedInterface {
            physical_parent: parent.into(),
            vid: 0x046d,
            pid: 0xc547,
            interface: descriptor(i32::from(number)),
            transport: Box::new(Mock(number, wire.clone())),
        }
    }
    fn setup() -> (HidGroup, Arc<Mutex<Wire>>) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let parent = format!("mock-usb-{}", NEXT.fetch_add(1, Ordering::Relaxed));
        let wire = Arc::new(Mutex::new(Wire::default()));
        (
            HidGroup::new(vec![opened(&parent, 1, &wire), opened(&parent, 2, &wire)]).unwrap(),
            wire,
        )
    }
    fn device(index: u8) -> Device {
        Device {
            vid: 0x046d,
            pid: 0xc547,
            interfaces: vec![descriptor(1), descriptor(2)],
            hidpp: Some(Hidpp {
                device_index: index,
                software_id: 7,
            }),
        }
    }
    fn packet(index: u8, feature: u8, function: u8) -> Vec<u8> {
        let mut data = vec![0; 20];
        data[..4].copy_from_slice(&[0x11, index, feature, function]);
        data
    }
    fn write(slot: u8, data: Vec<u8>) -> Step {
        Step {
            operation: TransportOperation::Hid {
                interface: slot,
                operation: Operation::Write,
            },
            data,
            read_len: 0,
        }
    }
    fn policy(device: &Device) -> Policy {
        Policy {
            backend: "hid".into(),
            identities: vec![Identity::HidGroup {
                device: device.clone(),
            }],
            subnets: vec![],
            recipes: ["probe", "claim", "frame", "release"]
                .into_iter()
                .map(|id| {
                    let steps = vec![
                        write(
                            0,
                            packet(device.hidpp.as_ref().unwrap().device_index, 0x0a, 0x17),
                        ),
                        write(
                            1,
                            packet(device.hidpp.as_ref().unwrap().device_index, 0x0b, 0x17),
                        ),
                    ];
                    Recipe {
                        id: id.into(),
                        masks: steps.iter().map(|s| vec![0; s.data.len()]).collect(),
                        steps,
                    }
                })
                .collect(),
        }
    }
    fn transact(
        broker: &mut Broker,
        policy: &Policy,
        id: &str,
        sequence: u64,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let steps = policy
            .recipes
            .iter()
            .find(|r| r.id == id)
            .unwrap()
            .steps
            .clone();
        let call = Call {
            v: API,
            call: sequence,
            capability: broker.capability(),
            recipe: id.into(),
            steps,
        };
        broker
            .transact(&call, Instant::now() + Duration::from_secs(2))
            .result
    }

    #[test]
    fn grouping_requires_same_physical_parent_and_exact_signed_interfaces() {
        let wire = Arc::new(Mutex::new(Wire::default()));
        assert!(HidGroup::new(vec![opened("one", 1, &wire), opened("two", 2, &wire)]).is_err());
        assert!(HidGroup::new(vec![opened("one", 1, &wire), opened("one", 1, &wire)]).is_err());
        let (group, _) = setup();
        let mut wrong = device(1);
        wrong.interfaces[1].usage += 1;
        assert!(Broker::new_hid(policy(&wrong), wrong, group.clone()).is_err());
        let d = device(1);
        let p = policy(&d);
        let first = Broker::new_hid(p.clone(), d.clone(), group.clone()).unwrap();
        assert!(Broker::new_hid(p.clone(), d.clone(), group.clone()).is_err());
        drop(first); // No software claim: safely reusable without quarantine.
        assert!(Broker::new_hid(p, d, group).is_ok());
        assert!(wire.lock().unwrap().trace.is_empty());
    }

    #[test]
    fn claim_release_and_receiver_slot_ownership_cover_the_complete_group() {
        let (group, wire) = setup();
        let d = device(1);
        let p = policy(&d);
        let mut first = Broker::new_hid(p.clone(), d.clone(), group.clone()).unwrap();
        let second_device = device(2);
        let second_policy = policy(&second_device);
        let mut second =
            Broker::new_hid(second_policy.clone(), second_device, group.clone()).unwrap();
        assert!(transact(&mut first, &p, "claim", 1).is_ok());
        assert!(transact(&mut second, &second_policy, "claim", 1).is_ok());
        assert!(transact(&mut first, &p, "frame", 2).is_ok());
        assert!(transact(&mut first, &p, "release", 3).is_ok());
        drop(first);
        assert!(!group.is_faulted());
        assert_eq!(
            wire.lock()
                .unwrap()
                .trace
                .iter()
                .map(|(slot, _)| *slot)
                .collect::<Vec<_>>(),
            [1, 2, 1, 2, 1, 2, 1, 2]
        );
        assert!(transact(&mut second, &second_policy, "release", 2).is_ok());
        drop(second);
        assert!(!group.is_faulted());
    }

    #[test]
    fn no_frame_before_claim_and_unreleased_drop_quarantines_aliases() {
        let (group, wire) = setup();
        let d = device(1);
        let p = policy(&d);
        let mut denied = Broker::new_hid(p.clone(), d.clone(), group.clone()).unwrap();
        assert_eq!(transact(&mut denied, &p, "frame", 1), Err(Error::Denied));
        assert!(wire.lock().unwrap().trace.is_empty());
        let mut active = Broker::new_hid(p.clone(), d, group.clone()).unwrap();
        transact(&mut active, &p, "claim", 1).unwrap();
        drop(active);
        assert!(group.is_faulted());
        assert!(Broker::new_hid(p, device(1), group).is_err());
    }

    #[test]
    fn hidpp_reads_filter_other_slots_and_correlate_full_response_header() {
        let (group, wire) = setup();
        let d = device(1);
        let mut p = policy(&d);
        p.recipes
            .iter_mut()
            .find(|r| r.id == "frame")
            .unwrap()
            .steps
            .push(Step {
                operation: TransportOperation::Hid {
                    interface: 1,
                    operation: Operation::Read {
                        report_id: 0x11,
                        feature: 0x0b,
                        function: 0x17,
                    },
                },
                data: vec![],
                read_len: 20,
            });
        p.recipes
            .iter_mut()
            .find(|r| r.id == "frame")
            .unwrap()
            .masks
            .push(vec![]);
        let mut broker = Broker::new_hid(p.clone(), d, group.clone()).unwrap();
        transact(&mut broker, &p, "claim", 1).unwrap();
        let expected = packet(1, 0x0b, 0x17);
        wire.lock().unwrap().reads.extend([
            packet(2, 0x0b, 0x17),
            packet(1, 0x0b, 0x18),
            packet(1, 0x0c, 0x17),
            expected.clone(),
        ]);
        let replies = transact(&mut broker, &p, "frame", 2).unwrap();
        assert_eq!(replies[2], expected);
        transact(&mut broker, &p, "release", 3).unwrap();
        assert!(!group.is_faulted());
    }

    #[test]
    fn all_steps_are_preflighted_and_short_writes_fail_closed() {
        for short in [false, true] {
            let (group, wire) = setup();
            let d = device(1);
            let mut p = policy(&d);
            if short {
                wire.lock().unwrap().short = true;
            } else {
                let r = p.recipes.iter_mut().find(|r| r.id == "claim").unwrap();
                // Signed policy allows a long feature report, but the bound
                // output handle is only 64 bytes. No earlier step may execute.
                r.steps[1].data.resize(65, 0);
                r.masks[1].resize(65, 0);
                // Generic multi-interface policy, rather than HID++ packet sizes.
                for i in &mut p.identities {
                    let Identity::HidGroup { device } = i else {
                        unreachable!()
                    };
                    device.hidpp = None;
                }
            }
            let d = if short {
                d
            } else {
                let Identity::HidGroup { device } = &p.identities[0] else {
                    unreachable!()
                };
                device.clone()
            };
            let mut broker = Broker::new_hid(p.clone(), d, group.clone()).unwrap();
            assert!(transact(&mut broker, &p, "claim", 1).is_err());
            assert!(group.is_faulted());
            assert_eq!(wire.lock().unwrap().trace.len(), usize::from(short));
        }
    }

    #[test]
    fn blocked_native_call_times_out_and_never_dispatches_the_next_step() {
        let (group, wire) = setup();
        let d = device(1);
        let p = policy(&d);
        let mut broker = Broker::new_hid(p.clone(), d, group.clone()).unwrap();
        let (entered, started) = mpsc::sync_channel(1);
        let (resume, paused) = mpsc::sync_channel(1);
        let (stopped, finished) = mpsc::sync_channel(1);
        wire.lock().unwrap().pause = Some((entered, paused));
        wire.lock().unwrap().stopped = Some(stopped);
        let call = Call {
            v: API,
            call: 1,
            capability: broker.capability(),
            recipe: "claim".into(),
            steps: p.recipes[1].steps.clone(),
        };
        let transaction = std::thread::spawn(move || {
            broker
                .transact(&call, Instant::now() + Duration::from_secs(1))
                .result
        });
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(transaction.join().unwrap(), Err(Error::Timeout));
        assert!(group.is_faulted());
        drop(group);
        resume.send(()).unwrap();
        finished.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(wire.lock().unwrap().trace.len(), 1);
    }
    #[test]
    fn runtime_feature_exchange_reads_only_the_request_correlated_response() {
        let (group, wire) = setup();
        let d = device(1);
        let mut p = policy(&d);
        let frame = p.recipes.iter_mut().find(|r| r.id == "frame").unwrap();
        frame.steps = d.exchange(1, 0x11, 0, 1, &[]).unwrap();
        frame.masks = vec![vec![0; 20], vec![]];
        frame.masks[0][2] = 0xff;
        let mut broker = Broker::new_hid(p.clone(), d.clone(), group.clone()).unwrap();
        transact(&mut broker, &p, "claim", 1).unwrap();
        let expected = packet(1, 0x3b, 0x17);
        wire.lock().unwrap().reads.extend([
            packet(2, 0x3b, 0x17),
            packet(1, 0, 0x17),
            expected.clone(),
        ]);
        let call = Call {
            v: API,
            call: 2,
            capability: broker.capability(),
            recipe: "frame".into(),
            steps: d.exchange(1, 0x11, 0x3b, 1, &[]).unwrap(),
        };
        assert_eq!(
            broker
                .transact(&call, Instant::now() + Duration::from_secs(2))
                .result
                .unwrap()[1],
            expected
        );
        transact(&mut broker, &p, "release", 3).unwrap();
        assert!(!group.is_faulted());
    }
    #[test]
    fn concurrent_receiver_recipes_never_interleave_interface_writes() {
        let (group, wire) = setup();
        let barrier = Arc::new(std::sync::Barrier::new(6));
        let threads: Vec<_> = (1..=6)
            .map(|index| {
                let d = device(index);
                let p = policy(&d);
                let mut broker = Broker::new_hid(p.clone(), d, group.clone()).unwrap();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    for (sequence, recipe) in ["claim", "frame", "release"].into_iter().enumerate()
                    {
                        transact(&mut broker, &p, recipe, sequence as u64 + 1).unwrap();
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(!group.is_faulted());
        let wire = wire.lock().unwrap();
        assert_eq!(wire.trace.len(), 36);
        for pair in wire.trace.as_chunks::<2>().0 {
            assert_eq!((pair[0].0, pair[1].0), (1, 2));
            assert_eq!(pair[0].1[1], pair[1].1[1]);
        }
    }
}
