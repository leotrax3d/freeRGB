//! Release updates for the modern GUI, using the same verified updater as
//! freeRGB Classic (`crates/freergb-update`): stable releases only, exact
//! installer asset, SHA-256 checked before staging and again before starting
//! Setup. Installation is always an explicit user action (Setup asks for UAC).
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Default)]
pub struct Updates {
    found: Mutex<Option<freergb_update::Update>>,
    downloaded: Mutex<Option<PathBuf>>,
}

#[derive(serde::Serialize)]
pub struct UpdateInfo {
    pub current: &'static str,
    pub available: Option<String>,
    pub size: Option<u64>,
    pub downloaded: bool,
}

fn directory() -> Result<PathBuf, String> {
    #[cfg(windows)]
    let root = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    Ok(root
        .ok_or("user configuration directory unavailable")?
        .join(if cfg!(windows) { "freeRGB" } else { "freergb" })
        .join("updates"))
}

impl Updates {
    fn info(&self) -> UpdateInfo {
        let found = self.found.lock().unwrap_or_else(|e| e.into_inner()).clone();
        UpdateInfo {
            current: env!("CARGO_PKG_VERSION"),
            available: found.as_ref().map(|u| u.version.clone()),
            size: found.as_ref().map(|u| u.size),
            downloaded: self.downloaded.lock().unwrap_or_else(|e| e.into_inner()).is_some(),
        }
    }
    pub fn check(&self) -> Result<UpdateInfo, String> {
        let update = freergb_update::check()?;
        *self.found.lock().unwrap_or_else(|e| e.into_inner()) = update;
        *self.downloaded.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(self.info())
    }
    pub fn download(&self) -> Result<UpdateInfo, String> {
        let update = self
            .found
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or("Check for updates first")?;
        let path = freergb_update::download(&update, &directory()?)?;
        *self.downloaded.lock().unwrap_or_else(|e| e.into_inner()) = Some(path);
        Ok(self.info())
    }
    /// Re-verifies the staged installer, then starts it (it asks for UAC and
    /// stops the service and this window itself).
    pub fn install(&self) -> Result<(), String> {
        let update = self
            .found
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or("Check for updates first")?;
        let path = self
            .downloaded
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or("Download the update first")?;
        freergb_update::verify(&path, &update)?;
        std::process::Command::new(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
