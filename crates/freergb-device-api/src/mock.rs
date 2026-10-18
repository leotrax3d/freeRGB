//! Scriptable in-memory [`HidTransport`] for protocol simulation tests.
//!
//! Every written report is recorded. Responses come from a user-supplied
//! responder closure, so a test can emulate a device state machine without any
//! hardware.

use crate::transport::{HidTransport, TransportError};
use std::collections::VecDeque;

type Responder = Box<dyn FnMut(&[u8]) -> Vec<Vec<u8>> + Send>;

pub struct MockTransport {
    out_len: usize,
    in_len: usize,
    written: Vec<Vec<u8>>,
    pending: VecDeque<Vec<u8>>,
    responder: Responder,
}

impl MockTransport {
    /// `responder` receives each padded written report and returns zero or more
    /// input reports to queue.
    pub fn new(
        out_len: usize,
        in_len: usize,
        responder: impl FnMut(&[u8]) -> Vec<Vec<u8>> + Send + 'static,
    ) -> Self {
        Self {
            out_len,
            in_len,
            written: Vec::new(),
            pending: VecDeque::new(),
            responder: Box::new(responder),
        }
    }

    pub fn written(&self) -> &[Vec<u8>] {
        &self.written
    }

    pub fn take_written(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.written)
    }
}

impl HidTransport for MockTransport {
    fn write(&mut self, report: &[u8]) -> Result<usize, TransportError> {
        if report.len() > self.out_len {
            return Err(TransportError::Io(format!(
                "report of {} bytes exceeds output length {}",
                report.len(),
                self.out_len
            )));
        }
        let mut padded = report.to_vec();
        padded.resize(self.out_len, 0);
        let responses = (self.responder)(&padded);
        self.pending.extend(responses);
        self.written.push(padded);
        Ok(self.out_len)
    }

    fn read_timeout(&mut self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize, TransportError> {
        match self.pending.pop_front() {
            Some(r) => {
                let n = r.len().min(buf.len());
                buf[..n].copy_from_slice(&r[..n]);
                Ok(n)
            }
            None => Ok(0),
        }
    }

    fn output_report_len(&self) -> usize {
        self.out_len
    }

    fn input_report_len(&self) -> usize {
        self.in_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_responds() {
        let mut t = MockTransport::new(8, 8, |r| vec![vec![r[1], 0xAA]]);
        t.write(&[0x00, 0x42]).unwrap();
        assert_eq!(t.written()[0], vec![0x00, 0x42, 0, 0, 0, 0, 0, 0]);
        let mut buf = [0u8; 8];
        assert_eq!(t.read_timeout(&mut buf, 10).unwrap(), 2);
        assert_eq!(&buf[..2], &[0x42, 0xAA]);
        assert_eq!(t.read_timeout(&mut buf, 10).unwrap(), 0);
        assert!(t.write(&[0u8; 9]).is_err());
    }
}
