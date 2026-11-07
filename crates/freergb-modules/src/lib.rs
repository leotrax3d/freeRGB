//! Hardware modules: signed catalog, local device matching, verified
//! downloads, atomic installation and the process host that runs modules.
//! Design and formats: `docs/HARDWARE_MODULES.md`.

pub mod bus_host;
pub mod catalog;
pub mod cooling_host;
pub mod host;
pub mod package;
pub mod platform;
pub mod setup;
pub mod source;
pub mod store;
pub mod transport_host;
pub mod trust;

pub type Result<T> = std::io::Result<T>;

pub(crate) fn invalid(message: impl ToString) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.to_string())
}

/// Network/transport failures are retried; they are not "invalid data".
pub(crate) fn net(error: reqwest::Error) -> std::io::Error {
    if error.is_timeout() {
        std::io::Error::new(std::io::ErrorKind::TimedOut, error.to_string())
    } else {
        std::io::Error::other(error.to_string())
    }
}

#[cfg(test)]
mod tests;
