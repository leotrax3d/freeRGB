//! Driver-neutral Windows implementation. No DLL loading, device handles,
//! IOCTL numbers or kernel port access exist here. A future protected-helper
//! adapter must implement this interface against the owner's selected driver.
use super::Device;
use freergb_module_api::bus::{Error, Operation, Transfer};

/// Each method is one transaction on a host-bound physical adapter. Implementors
/// must not retry, scan, change banks, or accept paths/ports from a module.
/// SMBus block reads must clock at most `length` bytes, or return Unsupported
/// before I/O (standard variable-length SMBus needs a full 32-byte permission).
pub trait DriverInterface: Send + 'static {
    fn read_byte(&mut self, address: u8, register: u8) -> Result<u8, Error>;
    fn write_byte(&mut self, address: u8, register: u8, value: u8) -> Result<(), Error>;
    fn read_block(&mut self, address: u8, register: u8, length: u8) -> Result<Vec<u8>, Error>;
    fn write_block(&mut self, address: u8, register: u8, data: &[u8]) -> Result<(), Error>;
    /// Register-select followed by repeated START, read, STOP.
    fn i2c_read(&mut self, address: u8, register: u8, length: u8) -> Result<Vec<u8>, Error>;
    /// Register and data in a single message, STOP.
    fn i2c_write(&mut self, address: u8, register: u8, data: &[u8]) -> Result<(), Error>;
}

pub struct WindowsDevice<D: DriverInterface>(D);
impl<D: DriverInterface> WindowsDevice<D> {
    pub fn new(driver: D) -> Self {
        Self(driver)
    }
}
impl<D: DriverInterface> Device for WindowsDevice<D> {
    fn transfer(&mut self, t: &Transfer) -> Result<Vec<u8>, Error> {
        match t.op {
            Operation::SmbusReadByteData => {
                self.0.read_byte(t.address, t.register).map(|b| vec![b])
            }
            Operation::SmbusWriteByteData => self
                .0
                .write_byte(t.address, t.register, *t.data.first().ok_or(Error::Denied)?)
                .map(|_| vec![]),
            Operation::SmbusReadBlockData => self.0.read_block(t.address, t.register, t.read_len),
            Operation::SmbusWriteBlockData => self
                .0
                .write_block(t.address, t.register, &t.data)
                .map(|_| vec![]),
            Operation::I2cReadRegister => self.0.i2c_read(t.address, t.register, t.read_len),
            Operation::I2cWriteRegister => self
                .0
                .i2c_write(t.address, t.register, &t.data)
                .map(|_| vec![]),
        }
    }
}
