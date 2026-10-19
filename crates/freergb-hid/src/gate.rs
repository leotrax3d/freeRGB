//! FIFO admission for HID operations. A fast worker must not jump ahead of
//! workers already waiting for the process-wide transport gate.
use std::sync::{Condvar, Mutex};

pub(crate) struct FairGate {
    tickets: Mutex<Tickets>,
    changed: Condvar,
}

struct Tickets {
    next: u64,
    serving: u64,
}

impl FairGate {
    pub(crate) const fn new() -> Self {
        Self {
            tickets: Mutex::new(Tickets {
                next: 0,
                serving: 0,
            }),
            changed: Condvar::new(),
        }
    }

    pub(crate) fn lock(&self) -> GateGuard<'_> {
        let mut state = self.tickets.lock().unwrap_or_else(|e| e.into_inner());
        let ticket = state.next;
        state.next = state.next.wrapping_add(1);
        while state.serving != ticket {
            state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        GateGuard { gate: self }
    }
}

/// Drop advances the queue even when the HID operation unwinds.
pub(crate) struct GateGuard<'a> {
    gate: &'a FairGate,
}

impl Drop for GateGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.gate.tickets.lock().unwrap_or_else(|e| e.into_inner());
        state.serving = state.serving.wrapping_add(1);
        // notify_one could wake a later ticket, leaving the next owner asleep.
        self.gate.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc};
    use std::time::{Duration, Instant};

    #[test]
    fn queued_workers_enter_in_order_without_overlap() {
        let gate = Arc::new(FairGate::new());
        let held = gate.lock();
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        let mut workers = Vec::new();
        for id in 0..8 {
            let worker_gate = gate.clone();
            let active = active.clone();
            let tx = tx.clone();
            workers.push(std::thread::spawn(move || {
                let _guard = worker_gate.lock();
                use std::sync::atomic::Ordering::SeqCst;
                assert_eq!(active.fetch_add(1, SeqCst), 0);
                tx.send(id).unwrap();
                std::thread::yield_now();
                assert_eq!(active.fetch_sub(1, SeqCst), 1);
            }));
            // Observe admission, rather than assuming a sleep queues a thread.
            let deadline = Instant::now() + Duration::from_secs(5);
            while gate.tickets.lock().unwrap().next != id + 2 {
                assert!(Instant::now() < deadline, "worker failed to queue");
                std::thread::yield_now();
            }
        }
        drop(held);
        // Re-entry cannot overtake any of the already queued workers.
        let _again = gate.lock();
        for id in 0..8 {
            assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), id);
        }
        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn panic_releases_gate() {
        let gate = FairGate::new();
        assert!(std::panic::catch_unwind(|| {
            let _guard = gate.lock();
            panic!("simulated transport panic");
        })
        .is_err());
        let _guard = gate.lock();
    }

    #[test]
    fn ticket_counter_wraps() {
        let gate = FairGate::new();
        *gate.tickets.lock().unwrap() = Tickets {
            next: u64::MAX,
            serving: u64::MAX,
        };
        drop(gate.lock());
        let _guard = gate.lock();
        assert_eq!(gate.tickets.lock().unwrap().serving, 0);
    }
}
