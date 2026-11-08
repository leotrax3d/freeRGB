//! Safe i2c-dev binding; no scan, force-slave, kernel module load or retries.
use super::Device;
use freergb_module_api::bus::{Error, Operation, Transfer, MAX_BLOCK};
use i2cdev::core::{I2CDevice, I2CMessage, I2CTransfer};
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError, LinuxI2CMessage};
use std::path::PathBuf;

/// The protected owner binds the adapter number; no I/O until an authorized
/// transaction reaches the worker. Never construct from module-provided input.
pub struct LinuxDevice {
    path: PathBuf,
}
impl LinuxDevice {
    pub fn adapter(number: u32) -> Self {
        Self {
            path: PathBuf::from(format!("/dev/i2c-{number}")),
        }
    }
}

fn map_error(error: LinuxI2CError) -> Error {
    let error: std::io::Error = error.into();
    match error.raw_os_error() {
        Some(1 | 13) => Error::Denied,
        Some(16) => Error::Busy,
        Some(6 | 121) => Error::Nack,
        Some(110) => Error::Timeout,
        Some(2 | 19) => Error::Disconnected,
        Some(25 | 95) => Error::Unsupported,
        _ => Error::Faulted,
    }
}

impl Device for LinuxDevice {
    fn transfer(&mut self, t: &Transfer) -> Result<Vec<u8>, Error> {
        // SMBus block count is supplied by the device, not by the caller. Never
        // clock a possible 32-byte response under a shorter register permission.
        if t.op == Operation::SmbusReadBlockData && t.len() != MAX_BLOCK {
            return Err(Error::Unsupported);
        }
        let mut device =
            LinuxI2CDevice::new(&self.path, u16::from(t.address)).map_err(map_error)?;
        match t.op {
            Operation::SmbusReadByteData => device
                .smbus_read_byte_data(t.register)
                .map(|v| vec![v])
                .map_err(map_error),
            Operation::SmbusWriteByteData => device
                .smbus_write_byte_data(t.register, *t.data.first().ok_or(Error::Denied)?)
                .map(|_| vec![])
                .map_err(map_error),
            Operation::SmbusReadBlockData => {
                device.smbus_read_block_data(t.register).map_err(map_error)
            }
            Operation::SmbusWriteBlockData => device
                .smbus_write_block_data(t.register, &t.data)
                .map(|_| vec![])
                .map_err(map_error),
            Operation::I2cReadRegister => {
                let command = [t.register];
                let mut result = vec![0; t.len()];
                let mut messages = [
                    LinuxI2CMessage::write(&command),
                    LinuxI2CMessage::read(&mut result),
                ];
                let count = device.transfer(&mut messages).map_err(map_error)?;
                if count != 2 {
                    return Err(Error::Faulted);
                }
                Ok(result)
            }
            Operation::I2cWriteRegister => {
                let mut data = vec![t.register];
                data.extend_from_slice(&t.data);
                let mut messages = [LinuxI2CMessage::write(&data)];
                let count = device.transfer(&mut messages).map_err(map_error)?;
                if count != 1 {
                    return Err(Error::Faulted);
                }
                Ok(vec![])
            }
        }
    }
}
