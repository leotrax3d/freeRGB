//! Raw report transport.

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("device not found or could not be opened: {0}")]
    Open(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("timed out waiting for a response")]
    Timeout,
    #[error("device disconnected")]
    Disconnected,
}

/// One opened HID interface.
///
/// Buffers passed to [`HidTransport::write`] start with the report ID byte
/// (`0x00` for devices without numbered reports), exactly as hidapi expects.
pub trait HidTransport: Send {
    /// Writes one output report. Implementations pad to
    /// [`HidTransport::output_report_len`] with zeros.
    fn write(&mut self, report: &[u8]) -> Result<usize, TransportError>;

    /// Reads one input report into `buf`. Returns `Ok(0)` on timeout.
    fn read_timeout(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, TransportError>;

    /// Output report length including the report ID byte.
    fn output_report_len(&self) -> usize;

    /// Input report length including the report ID byte (if any).
    fn input_report_len(&self) -> usize;

    /// Discards pending input reports.
    fn drain(&mut self) -> Result<(), TransportError> {
        let mut buf = vec![0u8; self.input_report_len().max(1)];
        while self.read_timeout(&mut buf, 0)? > 0 {}
        Ok(())
    }
}
