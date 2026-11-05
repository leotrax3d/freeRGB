//! No installer, child process, UAC, service or hardware is invoked here.
use super::*;
use std::io;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "freergb-recovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn metadata(bytes: &[u8]) -> Update {
    Update {
        version: "0.2.0".into(),
        asset_id: 42,
        sha256: format!("{:x}", Sha256::digest(bytes)),
        size: bytes.len() as u64,
    }
}
struct Interrupted(bool);
impl Read for Interrupted {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.0 {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "fixture disconnect",
            ));
        }
        self.0 = true;
        out[..3].copy_from_slice(b"new");
        Ok(3)
    }
}
struct FullDisk<'a>(&'a mut std::fs::File);
impl Write for FullDisk<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Simulate a real short disk write followed by ENOSPC.
        if self.0.metadata()?.len() > 0 {
            return Err(io::Error::other("fixture disk full"));
        }
        self.0.write(&bytes[..bytes.len().min(2)])
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[test]
fn interrupted_short_oversized_and_corrupt_downloads_never_replace_staged_version() {
    let root = Fixture::new();
    let destination = root.0.join("installer.exe");
    let bytes = b"new complete fixture installer";
    let update = metadata(bytes);
    std::fs::write(&destination, b"old verified installer").unwrap();
    assert!(stage_verified(Interrupted(false), &update, &destination).is_err());
    for invalid in [
        &bytes[..4],
        b"new complete fixture installeX".as_slice(),
        &[b'x'; 128],
    ] {
        assert!(stage_verified(invalid, &update, &destination).is_err());
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"old verified installer"
        );
        assert_eq!(std::fs::read_dir(&root.0).unwrap().count(), 1);
    }
    let error = stage_transfer(&destination, |file| {
        copy_verified(bytes.as_slice(), &mut FullDisk(file), &update)
    })
    .unwrap_err();
    assert!(error.contains("disk full"));
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"old verified installer"
    );
    assert_eq!(std::fs::read_dir(&root.0).unwrap().count(), 1);
}

#[test]
fn verified_download_cannot_be_launched_after_tampering_or_partial_restart() {
    let root = Fixture::new();
    let bytes = b"complete fixture";
    let update = metadata(bytes);
    let destination = root.0.join("installer.exe");
    // A power loss can leave a .part, but it is never a verified destination.
    std::fs::write(destination.with_extension("part"), &bytes[..4]).unwrap();
    assert!(verify(&destination, &update).is_err());
    stage_verified(bytes.as_slice(), &update, &destination).unwrap();
    verify(&destination, &update).unwrap();
    std::fs::write(&destination, vec![0; bytes.len()]).unwrap();
    assert!(verify(&destination, &update).is_err());
}

// A disk-backed installer MODEL, deliberately not wired to install_verified.
// The platform installer owns real Program Files replacement and recovery.
struct InstallModel {
    root: Fixture,
    current: Update,
    previous: Option<Update>,
}
impl InstallModel {
    fn new() -> Self {
        let root = Fixture::new();
        let bytes = b"version one complete image";
        std::fs::write(root.0.join("active"), bytes).unwrap();
        Self {
            root,
            current: Update {
                version: "0.1.0".into(),
                ..metadata(bytes)
            },
            previous: None,
        }
    }
    fn start(&self) -> Result<(), String> {
        verify(&self.root.0.join("active"), &self.current)
    }
    fn swap(&mut self, staged: &Path, next: Update, start_ok: bool) -> Result<(), String> {
        // Fail closed before touching the current image.
        verify(staged, &next)?;
        let active = self.root.0.join("active");
        let backup = self.root.0.join("previous");
        std::fs::rename(&active, &backup).map_err(|e| e.to_string())?;
        if let Err(error) = std::fs::rename(staged, &active) {
            std::fs::rename(&backup, &active).unwrap();
            return Err(error.to_string());
        }
        let prior = std::mem::replace(&mut self.current, next);
        if !start_ok || self.start().is_err() {
            std::fs::remove_file(&active).unwrap();
            std::fs::rename(&backup, &active).unwrap();
            self.current = prior;
            return Err("fixture failed start: previous version restored".into());
        }
        self.previous = Some(prior);
        Ok(())
    }
}
#[test]
fn cross_version_swap_failed_start_rolls_back_exact_bytes_then_retry_succeeds() {
    let mut install = InstallModel::new();
    let bytes = b"version two complete image";
    let update = metadata(bytes);
    let stage = install.root.0.join("stage");
    std::fs::write(&stage, &bytes[..5]).unwrap();
    assert!(install.swap(&stage, update.clone(), true).is_err());
    install.start().unwrap();
    std::fs::remove_file(&stage).unwrap();
    stage_verified(bytes.as_slice(), &update, &stage).unwrap();
    assert!(install.swap(&stage, update.clone(), false).is_err());
    install.start().unwrap();
    assert_eq!(install.current.version, "0.1.0");
    assert_eq!(
        std::fs::read(install.root.0.join("active")).unwrap(),
        b"version one complete image"
    );
    stage_verified(bytes.as_slice(), &update, &stage).unwrap();
    install.swap(&stage, update, true).unwrap();
    install.start().unwrap();
    assert_eq!(install.current.version, "0.2.0");
    assert_eq!(install.previous.unwrap().version, "0.1.0");
}
