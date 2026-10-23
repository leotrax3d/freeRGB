//! Where catalog and packages come from, and bounded, cancellable downloads.
//!
//! Default: the rolling `modules` release of github.com/leotrax3d/FreeRGB.
//! Assets are fetched through the GitHub API so a private repository works
//! with `FREERGB_GITHUB_TOKEN`. A local directory or a plain HTTPS base URL
//! can be configured instead (mirrors, offline repositories, tests).
use crate::catalog::Artifact;
use crate::{invalid, net, Result};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const DEFAULT_SOURCE: &str = "github:leotrax3d/FreeRGB@modules";
pub const CATALOG_FILE: &str = "catalog.json";
pub const SIGNATURE_FILE: &str = "catalog.json.sig";
const ATTEMPTS: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    GitHub {
        owner: String,
        repo: String,
        tag: String,
    },
    Https(String),
    Directory(PathBuf),
}

impl Source {
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix("github:") {
            let (repo, tag) = rest
                .split_once('@')
                .ok_or_else(|| invalid("github source needs @tag"))?;
            let (owner, repo) = repo
                .split_once('/')
                .ok_or_else(|| invalid("github source needs owner/repo"))?;
            let ok = |s: &str| {
                !s.is_empty()
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            };
            if !(ok(owner) && ok(repo) && ok(tag)) {
                return Err(invalid("invalid github source"));
            }
            return Ok(Self::GitHub {
                owner: owner.into(),
                repo: repo.into(),
                tag: tag.into(),
            });
        }
        if text.starts_with("https://") {
            return Ok(Self::Https(text.trim_end_matches('/').to_owned()));
        }
        if text.starts_with("http://") {
            return Err(invalid("module sources must use https"));
        }
        Ok(Self::Directory(PathBuf::from(
            text.strip_prefix("file://").unwrap_or(text),
        )))
    }

    pub fn describe(&self) -> String {
        match self {
            Self::GitHub { owner, repo, tag } => format!("github:{owner}/{repo}@{tag}"),
            Self::Https(base) => base.clone(),
            Self::Directory(path) => path.display().to_string(),
        }
    }
}

/// Live byte counter for one download, read by the setup state.
pub struct Progress<'a> {
    pub cancel: &'a AtomicBool,
    pub on_bytes: &'a dyn Fn(u64),
}

fn client(timeout: Duration) -> Result<Client> {
    Client::builder()
        .user_agent(concat!("freeRGB/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .build()
        .map_err(invalid)
}

fn github(client: &Client, url: String, accept: &str) -> reqwest::blocking::RequestBuilder {
    let request = client
        .get(url)
        .header("Accept", accept)
        .header("X-GitHub-Api-Version", "2022-11-28");
    match std::env::var("FREERGB_GITHUB_TOKEN") {
        Ok(token) if !token.is_empty() => request.bearer_auth(token),
        _ => request,
    }
}

pub struct Fetcher {
    pub source: Source,
    /// GitHub asset name -> asset id, filled when the release is listed.
    assets: std::collections::BTreeMap<String, u64>,
}

fn cancelled() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Interrupted, "cancelled")
}

impl Fetcher {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            assets: Default::default(),
        }
    }

    fn list_release(&mut self) -> Result<()> {
        let Source::GitHub { owner, repo, tag } = &self.source else {
            return Ok(());
        };
        if !self.assets.is_empty() {
            return Ok(());
        }
        let client = client(Duration::from_secs(20))?;
        let response = github(
            &client,
            format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}"),
            "application/vnd.github+json",
        )
        .send()
        .map_err(net)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!(
                "no release \"{tag}\" is accessible in {owner}/{repo}. The repository may be private (set FREERGB_GITHUB_TOKEN) or the release is not published yet."
            )));
        }
        let response = response.error_for_status().map_err(net)?;
        let mut text = String::new();
        response.take(2 * 1024 * 1024).read_to_string(&mut text)?;
        let release: serde_json::Value = serde_json::from_str(&text).map_err(invalid)?;
        for asset in release["assets"].as_array().into_iter().flatten() {
            if let (Some(name), Some(id)) = (asset["name"].as_str(), asset["id"].as_u64()) {
                self.assets.insert(name.to_owned(), id);
            }
        }
        Ok(())
    }

    /// Small file (catalog/signature), bounded by `limit`.
    pub fn small(&mut self, name: &str, limit: usize) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let cancel = AtomicBool::new(false);
        let progress = Progress {
            cancel: &cancel,
            on_bytes: &|_| {},
        };
        self.stream(name, limit as u64, &progress, &mut out)?;
        Ok(out)
    }

    fn stream(
        &mut self,
        name: &str,
        limit: u64,
        progress: &Progress,
        out: &mut dyn Write,
    ) -> Result<u64> {
        let source = self.source.clone();
        let mut reader: Box<dyn Read> = match &source {
            Source::Directory(dir) => {
                if !crate::package::safe_name(name) {
                    return Err(invalid("invalid asset name"));
                }
                Box::new(std::fs::File::open(dir.join(name))?)
            }
            _ if name.starts_with("https://") => Box::new(
                client(Duration::from_secs(300))?
                    .get(name)
                    .send()
                    .map_err(net)?
                    .error_for_status()
                    .map_err(net)?,
            ),
            Source::Https(base) => Box::new(
                client(Duration::from_secs(300))?
                    .get(format!("{base}/{name}"))
                    .send()
                    .map_err(net)?
                    .error_for_status()
                    .map_err(net)?,
            ),
            Source::GitHub { owner, repo, .. } => {
                self.list_release()?;
                let id = *self
                    .assets
                    .get(name)
                    .ok_or_else(|| invalid(format!("release has no asset {name}")))?;
                let client = client(Duration::from_secs(300))?;
                Box::new(
                    github(
                        &client,
                        format!("https://api.github.com/repos/{owner}/{repo}/releases/assets/{id}"),
                        "application/octet-stream",
                    )
                    .send()
                    .map_err(net)?
                    .error_for_status()
                    .map_err(net)?,
                )
            }
        };
        let mut buffer = [0u8; 32 * 1024];
        let mut total = 0u64;
        loop {
            if progress.cancel.load(Ordering::SeqCst) {
                return Err(cancelled());
            }
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                return Ok(total);
            }
            total += n as u64;
            if total > limit {
                return Err(invalid("download exceeds its expected size"));
            }
            out.write_all(&buffer[..n])?;
            (progress.on_bytes)(total);
        }
    }

    /// Downloads `artifact` into `dir`, verifying size and SHA-256 from the
    /// caller-validated catalog. Retries transient failures with a short back-off;
    /// a cancel removes the partial file.
    pub fn download(
        &mut self,
        artifact: &Artifact,
        dir: &Path,
        progress: &Progress,
    ) -> Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let destination = dir.join(format!("{}.zip", artifact.sha256));
        if destination.exists() && crate::package::sha256_file(&destination)? == artifact.sha256 {
            (progress.on_bytes)(artifact.size);
            return Ok(destination);
        }
        let partial = destination.with_extension("part");
        let mut last = None;
        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(500 << attempt));
            }
            (progress.on_bytes)(0);
            let result = (|| {
                let mut file = std::fs::File::create(&partial)?;
                let mut hasher = HashingWriter {
                    inner: &mut file,
                    hash: Sha256::new(),
                };
                let size = self.stream(&artifact.url, artifact.size, progress, &mut hasher)?;
                let digest = hex::encode(hasher.hash.finalize());
                file.sync_all()?;
                if size != artifact.size || digest != artifact.sha256 {
                    return Err(invalid("downloaded package does not match the catalog"));
                }
                Ok(())
            })();
            match result {
                Ok(()) => {
                    std::fs::rename(&partial, &destination)?;
                    return Ok(destination);
                }
                Err(error) => {
                    let _ = std::fs::remove_file(&partial);
                    let fatal = error.kind() == std::io::ErrorKind::Interrupted
                        || error.kind() == std::io::ErrorKind::InvalidData
                        || error.kind() == std::io::ErrorKind::NotFound;
                    last = Some(error);
                    if fatal {
                        break;
                    }
                }
            }
        }
        Err(last.unwrap_or_else(|| invalid("download failed")))
    }
}

struct HashingWriter<'a, W: Write> {
    inner: &'a mut W,
    hash: Sha256,
}

impl<W: Write> Write for HashingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hash.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
