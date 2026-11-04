//! `freergb-module-tool`: release side of the hardware module system.
//! See docs/HARDWARE_MODULES.md ("Build, sign and release").
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use freergb_modules::catalog::{Artifact, Catalog, DeviceMatch, ModuleEntry, Origin, Vendor};
use freergb_modules::package::{self, PackageManifest};
use freergb_modules::trust::{self, TrustedKeys};
use sha2::Digest;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "freergb-module-tool", version)]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Creates a new Ed25519 signing key (hex) and prints its public key.
    Keygen {
        #[arg(long)]
        out: PathBuf,
    },
    /// Prints the public key for a signing key file.
    Pubkey {
        #[arg(long)]
        key: PathBuf,
    },
    /// Packs a module executable with its license and provenance files.
    Pack {
        #[arg(long)]
        id: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        family: String,
        #[arg(long)]
        license: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        executable: PathBuf,
        /// JSON array of device matches (e.g. from `<module> --list-devices`).
        #[arg(long)]
        devices: PathBuf,
        /// Extra files (license text, PROVENANCE.md, NOTICE...).
        #[arg(long = "file")]
        files: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Packs the complete corresponding source for a module build.
    Source {
        /// Repository root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Paths relative to root that are needed to build the module.
        #[arg(long = "path")]
        paths: Vec<PathBuf>,
        /// Stored as the archive's root Cargo.toml (standalone workspace).
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Adds or replaces a module entry in a catalog (creates the file).
    Add {
        #[arg(long)]
        catalog: PathBuf,
        #[arg(long)]
        name: String,
        /// Package zips (one per target), named as uploaded.
        #[arg(long = "package")]
        packages: Vec<PathBuf>,
        #[arg(long)]
        source_package: PathBuf,
        #[arg(long)]
        origin_project: String,
        #[arg(long)]
        origin_url: String,
        #[arg(long)]
        origin_commit: String,
        #[arg(long = "vendor", value_parser = parse_vendor)]
        vendors: Vec<(u16, String)>,
    },
    /// Signs catalog.json, writing catalog.json.sig next to it.
    Sign {
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        catalog: PathBuf,
    },
    /// Verifies a catalog and every referenced file in a release directory.
    Verify {
        #[arg(long)]
        pubkey: String,
        #[arg(long)]
        dir: PathBuf,
    },
}

fn parse_vendor(text: &str) -> Result<(u16, String), String> {
    let (vid, name) = text
        .split_once('=')
        .ok_or("use VID=Name, e.g. 1532=Razer")?;
    Ok((
        u16::from_str_radix(vid.trim_start_matches("0x"), 16).map_err(|e| e.to_string())?,
        name.into(),
    ))
}

fn read_key(path: &Path) -> Result<SigningKey> {
    let bytes: [u8; 32] = hex::decode(std::fs::read_to_string(path)?.trim())?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signing key must be 32 bytes hex"))?;
    Ok(SigningKey::from_bytes(&bytes))
}

fn artifact(path: &Path, target: &str) -> Result<Artifact> {
    let bytes = std::fs::read(path).with_context(|| path.display().to_string())?;
    Ok(Artifact {
        target: target.into(),
        url: path.file_name().unwrap().to_string_lossy().into_owned(),
        size: bytes.len() as u64,
        sha256: hex::encode(sha2::Sha256::digest(&bytes)),
    })
}

fn read_manifest(path: &Path) -> Result<PackageManifest> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let entry = zip.by_name(package::MANIFEST)?;
    Ok(serde_json::from_reader(entry)?)
}

fn collect(root: &Path, rel: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let path = root.join(rel);
    if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(&path)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name();
            if name == "target" || name == "node_modules" || name.to_string_lossy().starts_with('.')
            {
                continue;
            }
            collect(root, &rel.join(name), out)?;
        }
    } else if path.is_file() {
        out.push(rel.to_path_buf());
    } else {
        bail!("{} does not exist", path.display());
    }
    Ok(())
}

fn main() -> Result<()> {
    match Args::parse().command {
        Cmd::Keygen { out } => {
            if out.exists() {
                bail!("{} exists; refusing to overwrite a key", out.display());
            }
            let key = SigningKey::generate(&mut rand::rngs::OsRng);
            std::fs::write(&out, hex::encode(key.to_bytes()))?;
            println!("{}", hex::encode(key.verifying_key().to_bytes()));
        }
        Cmd::Pubkey { key } => {
            println!(
                "{}",
                hex::encode(read_key(&key)?.verifying_key().to_bytes())
            )
        }
        Cmd::Pack {
            id,
            version,
            family,
            license,
            target,
            executable,
            devices,
            files,
            out,
        } => {
            let devices: Vec<DeviceMatch> = serde_json::from_slice(&std::fs::read(&devices)?)?;
            let exe_name = executable
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let mut contents = vec![(exe_name.clone(), std::fs::read(&executable)?)];
            for f in files {
                contents.push((
                    f.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read(&f)?,
                ));
            }
            let manifest = PackageManifest {
                schema: 1,
                id,
                version,
                family,
                api: vec![freergb_module_api::MODULE_API],
                target,
                executable: exe_name,
                license,
                devices,
                bus: None,
                cooling: None,
                files: Default::default(),
            };
            package::build(manifest, &contents, &out)?;
            println!("{}", out.display());
        }
        Cmd::Source {
            root,
            paths,
            manifest,
            out,
        } => {
            let mut files = Vec::new();
            for p in &paths {
                collect(&root, p, &mut files)?;
            }
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&out)?);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .last_modified_time(zip::DateTime::default());
            for f in &files {
                zip.start_file(f.to_string_lossy().replace('\\', "/"), options)?;
                zip.write_all(&std::fs::read(root.join(f))?)?;
            }
            if let Some(manifest) = manifest {
                zip.start_file("Cargo.toml", options)?;
                zip.write_all(&std::fs::read(manifest)?)?;
            }
            zip.finish()?;
            println!("{} files -> {}", files.len(), out.display());
        }
        Cmd::Add {
            catalog,
            name,
            packages,
            source_package,
            origin_project,
            origin_url,
            origin_commit,
            vendors,
        } => {
            let mut doc: Catalog = if catalog.exists() {
                serde_json::from_slice(&std::fs::read(&catalog)?)?
            } else {
                Catalog {
                    schema: 1,
                    sequence: 0,
                    generated: String::new(),
                    modules: vec![],
                    relevant_vendors: vec![],
                }
            };
            let first = packages.first().context("at least one --package")?;
            let manifest = read_manifest(first)?;
            let mut artifacts = Vec::new();
            for p in &packages {
                let m = read_manifest(p)?;
                if (m.id.as_str(), m.version.as_str())
                    != (manifest.id.as_str(), manifest.version.as_str())
                {
                    bail!("packages belong to different modules");
                }
                artifacts.push(artifact(p, &m.target)?);
            }
            let entry = ModuleEntry {
                id: manifest.id.clone(),
                version: manifest.version,
                family: manifest.family,
                name,
                api: manifest.api,
                license: manifest.license,
                origin: Origin {
                    project: origin_project,
                    url: origin_url,
                    commit: origin_commit,
                },
                devices: manifest.devices,
                artifacts,
                source: artifact(&source_package, "source")?,
                requires: vec![],
            };
            doc.modules.retain(|m| m.id != entry.id);
            doc.modules.push(entry);
            for (vid, name) in vendors {
                if !doc.relevant_vendors.iter().any(|v| v.vid == vid) {
                    doc.relevant_vendors.push(Vendor { vid, name });
                }
            }
            doc.sequence += 1;
            doc.generated = std::env::var("FREERGB_CATALOG_DATE").unwrap_or_default();
            doc.validate()?;
            std::fs::write(&catalog, serde_json::to_vec_pretty(&doc)?)?;
            println!("catalog sequence {}", doc.sequence);
        }
        Cmd::Sign { key, catalog } => {
            let bytes = std::fs::read(&catalog)?;
            let doc: Catalog = serde_json::from_slice(&bytes)?;
            doc.validate()?;
            std::fs::write(
                catalog.with_extension("json.sig"),
                trust::sign(&read_key(&key)?, &bytes),
            )?;
        }
        Cmd::Verify { pubkey, dir } => {
            let keys = TrustedKeys::default().with_development_key(trust::parse_key(&pubkey)?);
            let bytes = std::fs::read(dir.join("catalog.json"))?;
            let sig = std::fs::read(dir.join("catalog.json.sig"))?;
            let catalog = Catalog::verify(&bytes, &sig, &keys)?;
            for m in &catalog.modules {
                for a in m.artifacts.iter().chain([&m.source]) {
                    let actual = artifact(&dir.join(&a.url), &a.target)?;
                    if actual.sha256 != a.sha256 || actual.size != a.size {
                        bail!("{} does not match the catalog", a.url);
                    }
                }
                println!(
                    "ok {} {} ({} packages + source)",
                    m.id,
                    m.version,
                    m.artifacts.len()
                );
            }
        }
    }
    Ok(())
}
