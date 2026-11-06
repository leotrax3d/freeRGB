//! Local release orchestration. The final directory appears only after verification.
use super::*;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    root: PathBuf,
    builds: Vec<Build>,
    steps: Vec<Cmd>,
    notes: String,
    /// A previously verified release directory, including all referenced assets.
    previous: Option<PathBuf>,
    previous_pubkey: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Build {
    package: String,
    target: String,
}

fn local_asset(path: &Path) -> Result<String> {
    let name = path.to_str().context("non-UTF8 asset name")?;
    if !package::safe_name(name) {
        bail!("release outputs must be flat, safe asset names");
    }
    Ok(name.to_owned())
}

fn validate(plan: &Plan) -> Result<()> {
    if plan.builds.is_empty() || plan.notes.trim().is_empty() {
        bail!("release needs builds and release notes");
    }
    for build in &plan.builds {
        if !freergb_modules::catalog::valid_id(&build.package) || !package::safe_name(&build.target)
        {
            bail!("invalid cargo package or target");
        }
    }
    if plan.previous.is_some() != plan.previous_pubkey.is_some() {
        bail!("previous release requires its trusted public key");
    }
    let mut packages = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut consumed = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    for step in &plan.steps {
        match step {
            Cmd::Pack { out, .. } => {
                let name = local_asset(out)?;
                if !outputs.insert(name.clone()) {
                    bail!("duplicate output");
                }
                packages.insert(name);
            }
            Cmd::Source { out, .. } => {
                let name = local_asset(out)?;
                if !outputs.insert(name.clone()) {
                    bail!("duplicate output");
                }
                sources.insert(name);
            }
            Cmd::Add {
                catalog,
                packages: inputs,
                source_package,
                ..
            } => {
                if catalog != Path::new("catalog.json") || inputs.is_empty() {
                    bail!("add must target catalog.json and include packages");
                }
                for input in inputs {
                    let name = local_asset(input)?;
                    if !packages.contains(&name) || !consumed.insert(name) {
                        bail!("add must consume each previously packed asset once");
                    }
                }
                let name = local_asset(source_package)?;
                if !sources.contains(&name) {
                    bail!("source must precede add");
                }
                consumed.insert(name);
            }
            _ => bail!("release steps allow only pack, source and add"),
        }
    }
    if packages.is_empty()
        || outputs != consumed
        || outputs
            .iter()
            .any(|n| ["catalog.json", "catalog.json.sig", "RELEASE_NOTES.md"].contains(&n.as_str()))
    {
        bail!("every output must belong to a catalog entry");
    }
    Ok(())
}

fn input(root: &Path, path: &Path) -> Result<PathBuf> {
    let resolved = std::fs::canonicalize(root.join(path))?;
    if !resolved.starts_with(root) {
        bail!("release input must stay within source root");
    }
    Ok(resolved)
}

pub(super) fn run(plan_path: &Path, out: &Path, key: Option<&Path>, dry_run: bool) -> Result<()> {
    let plan: Plan = serde_json::from_slice(&std::fs::read(plan_path)?)?;
    validate(&plan)?;
    if dry_run {
        for build in &plan.builds {
            println!(
                "cargo build --release --locked -p {} --target {}",
                build.package, build.target
            );
        }
        println!("{} pack/source/add steps; sign, verify, write RELEASE_NOTES.md -> {} (no writes, no key read)", plan.steps.len(), out.display());
        return Ok(());
    }
    let root = std::fs::canonicalize(&plan.root)?;
    let key_path = std::fs::canonicalize(key.context("--key is required unless --dry-run")?)?;
    if key_path.starts_with(&root) {
        bail!("signing key must be outside the source tree");
    }
    let signing_key = read_key(&key_path)?;
    execute(plan, out, &signing_key, |build, root| {
        let status = std::process::Command::new("cargo")
            .current_dir(root)
            .args([
                "build",
                "--release",
                "--locked",
                "-p",
                &build.package,
                "--target",
                &build.target,
            ])
            .status()?;
        if !status.success() {
            bail!("cargo build failed for {}", build.package);
        }
        Ok(())
    })
}

fn execute(
    plan: Plan,
    out: &Path,
    key: &SigningKey,
    mut build: impl FnMut(&Build, &Path) -> Result<()>,
) -> Result<()> {
    // Reserve a sibling directory. Never overwrite or clean someone else's release.
    if out.exists() {
        bail!("release destination already exists");
    }
    let staging = out.with_extension("release-staging");
    std::fs::create_dir(&staging)
        .context("reserve release staging directory (inspect stale staging manually)")?;
    // Failures intentionally retain staging for diagnosis; it is never the final release.
    let root = std::fs::canonicalize(&plan.root)?;
    if let Some(previous) = &plan.previous {
        let pubkey = plan
            .previous_pubkey
            .as_ref()
            .context("missing previous public key")?;
        super::run(Cmd::Verify {
            pubkey: pubkey.clone(),
            dir: previous.clone(),
        })?;
        let catalog: Catalog =
            serde_json::from_slice(&std::fs::read(previous.join("catalog.json"))?)?;
        let assets: BTreeSet<_> = catalog
            .modules
            .iter()
            .flat_map(|m| m.artifacts.iter().chain([&m.source]))
            .map(|a| &a.url)
            .collect();
        for name in assets {
            local_asset(Path::new(name))?;
            std::fs::copy(previous.join(name), staging.join(name))?;
        }
        std::fs::copy(previous.join("catalog.json"), staging.join("catalog.json"))?;
    }
    for item in &plan.builds {
        build(item, &root)?;
    }
    for step in plan.steps {
        let step = match step {
            Cmd::Pack {
                id,
                version,
                family,
                license,
                target,
                executable,
                devices,
                cooling_policy,
                probe_policy,
                files,
                out,
            } => Cmd::Pack {
                id,
                version,
                family,
                license,
                target,
                executable: input(&root, &executable)?,
                devices: devices.map(|p| input(&root, &p)).transpose()?,
                probe_policy: probe_policy.map(|p| input(&root, &p)).transpose()?,
                cooling_policy: cooling_policy.map(|p| input(&root, &p)).transpose()?,
                files: files
                    .into_iter()
                    .map(|p| input(&root, &p))
                    .collect::<Result<_>>()?,
                out: staging.join(out),
            },
            Cmd::Source {
                root: source_root,
                paths,
                manifest,
                out,
            } => Cmd::Source {
                root: input(&root, &source_root)?,
                paths,
                manifest: manifest.map(|p| input(&root, &p)).transpose()?,
                out: staging.join(out),
            },
            Cmd::Add {
                catalog,
                name,
                packages,
                source_package,
                origin_project,
                origin_url,
                origin_commit,
                vendors,
            } => Cmd::Add {
                catalog: staging.join(catalog),
                name,
                packages: packages.into_iter().map(|p| staging.join(p)).collect(),
                source_package: staging.join(source_package),
                origin_project,
                origin_url,
                origin_commit,
                vendors,
            },
            _ => bail!("unsupported release step"),
        };
        super::run(step)?;
    }
    let bytes = std::fs::read(staging.join("catalog.json"))?;
    std::fs::write(staging.join("catalog.json.sig"), trust::sign(key, &bytes))?;
    super::run(Cmd::Verify {
        pubkey: hex::encode(key.verifying_key().to_bytes()),
        dir: staging.clone(),
    })?;
    let catalog: Catalog = serde_json::from_slice(&bytes)?;
    // Validate every package's complete contents against the catalog, not just ZIP hashes.
    for entry in &catalog.modules {
        for asset in &entry.artifacts {
            let verify_dir = staging.join("verify-package");
            std::fs::create_dir(&verify_dir)?;
            let (manifest, _) = package::extract(&staging.join(&asset.url), &verify_dir)?;
            manifest.check_against(entry, &asset.target)?;
            std::fs::remove_dir_all(&verify_dir)?;
        }
    }
    let mut notes = format!(
        "# freeRGB modules\n\nCatalog sequence: {}\n\n{}\n\n",
        catalog.sequence, plan.notes
    );
    for entry in &catalog.modules {
        notes.push_str(&format!(
            "- {} {} ({})\n",
            entry.id, entry.version, entry.license
        ));
        for asset in entry.artifacts.iter().chain([&entry.source]) {
            notes.push_str(&format!(
                "  - {}: {} bytes, SHA-256 `{}`\n",
                asset.url, asset.size, asset.sha256
            ));
        }
    }
    std::fs::write(staging.join("RELEASE_NOTES.md"), notes)?;
    std::fs::rename(&staging, out)?;
    println!("Verified local release: {} (not published)", out.display());
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "freergb-release-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn plan(&self) -> Plan {
            std::fs::write(self.0.join("fixture.exe"), b"fixture only; never run").unwrap();
            std::fs::write(self.0.join("COPYING"), b"fixture license").unwrap();
            std::fs::write(self.0.join("PROVENANCE.md"), b"simulation only").unwrap();
            std::fs::write(self.0.join("source.rs"), b"fn main() {}").unwrap();
            std::fs::write(
                self.0.join("devices.json"),
                br#"[{"name":"Fixture strip","kind":"lighting_controller","vid":65535,"pid":1}]"#,
            )
            .unwrap();
            serde_json::from_value(serde_json::json!({
                "root": self.0, "builds": [{"package":"fixture-module", "target":"x86_64-pc-windows-msvc"}],
                "notes":"Synthetic release; no hardware acceptance.",
                "steps": [
                    {"command":"pack", "id":"fixture", "version":"0.2.0", "family":"fixture", "license":"MIT", "target":"x86_64-pc-windows-msvc", "executable":"fixture.exe", "devices":"devices.json", "cooling_policy":null, "files":["COPYING","PROVENANCE.md"], "out":"fixture-0.2.0.zip"},
                    {"command":"source", "root":".", "paths":["source.rs"], "manifest":null, "out":"fixture-source.zip"},
                    {"command":"add", "catalog":"catalog.json", "name":"Fixture module", "packages":["fixture-0.2.0.zip"], "source_package":"fixture-source.zip", "origin_project":"Fixture", "origin_url":"https://example.invalid/fixture", "origin_commit":"abcdef0", "vendors":[]}
                ]
            })).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn release_build_pack_sign_verify_and_notes_with_fake_builder() {
        let fixture = Fixture::new();
        let plan = fixture.plan();
        validate(&plan).unwrap();
        let out = fixture.0.join("release");
        let key = SigningKey::from_bytes(&[7; 32]); // Public, deterministic test key only.
        let mut builds = 0;
        execute(plan, &out, &key, |_, _| {
            builds += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(builds, 1);
        assert!(!out.with_extension("release-staging").exists());
        assert!(std::fs::read_to_string(out.join("RELEASE_NOTES.md"))
            .unwrap()
            .contains("fixture 0.2.0"));
        let keys = TrustedKeys::default().with_development_key(key.verifying_key());
        let bytes = std::fs::read(out.join("catalog.json")).unwrap();
        let sig = std::fs::read(out.join("catalog.json.sig")).unwrap();
        Catalog::verify(&bytes, &sig, &keys).unwrap();
        let mut corrupt = sig;
        corrupt[0] ^= 1;
        assert!(Catalog::verify(&bytes, &corrupt, &keys).is_err());
        std::fs::write(out.join("fixture-0.2.0.zip"), b"tampered").unwrap();
        assert!(super::super::run(Cmd::Verify {
            pubkey: hex::encode(key.verifying_key().to_bytes()),
            dir: out
        })
        .is_err());
    }

    #[test]
    fn previous_release_requires_valid_signature_and_advances_sequence() {
        let fixture = Fixture::new();
        let key = SigningKey::from_bytes(&[7; 32]);
        let previous = fixture.0.join("previous");
        execute(fixture.plan(), &previous, &key, |_, _| Ok(())).unwrap();
        let mut plan = fixture.plan();
        plan.previous = Some(previous.clone());
        plan.previous_pubkey = Some(hex::encode(key.verifying_key().to_bytes()));
        let next = fixture.0.join("next");
        execute(plan, &next, &key, |_, _| Ok(())).unwrap();
        let catalog: Catalog =
            serde_json::from_slice(&std::fs::read(next.join("catalog.json")).unwrap()).unwrap();
        assert_eq!(catalog.sequence, 2);

        std::fs::write(previous.join("catalog.json.sig"), [0; 64]).unwrap();
        let mut plan = fixture.plan();
        plan.previous = Some(previous);
        plan.previous_pubkey = Some(hex::encode(key.verifying_key().to_bytes()));
        let rejected = fixture.0.join("rejected");
        let mut built = false;
        assert!(execute(plan, &rejected, &key, |_, _| {
            built = true;
            Ok(())
        })
        .is_err());
        assert!(!built);
        assert!(!rejected.exists());
    }

    #[test]
    fn failed_build_and_invalid_plan_never_expose_a_release() {
        let fixture = Fixture::new();
        let out = fixture.0.join("release");
        let plan = fixture.plan();
        assert!(
            execute(plan, &out, &SigningKey::from_bytes(&[7; 32]), |_, _| bail!(
                "fixture build failed"
            ))
            .is_err()
        );
        assert!(!out.exists());
        let mut plan = fixture.plan();
        plan.steps.push(Cmd::Keygen {
            out: "forbidden.key".into(),
        });
        assert!(validate(&plan).is_err());
        assert!(local_asset(Path::new("../escape.zip")).is_err());
    }

    #[test]
    fn dry_run_never_builds_writes_or_reads_a_signing_key() {
        let fixture = Fixture::new();
        // Deliberately nonexistent root/key; dry-run only parses and plans.
        let config = serde_json::json!({"root":"missing-root","builds":[{"package":"fixture","target":"x86_64-pc-windows-msvc"}],"notes":"fixture","steps":[
            {"command":"pack","id":"fixture","version":"0.2.0","family":"fixture","license":"MIT","target":"x86_64-pc-windows-msvc","executable":"missing.exe","devices":"devices.json","cooling_policy":null,"files":[],"out":"fixture.zip"},
            {"command":"source","root":".","paths":[],"manifest":null,"out":"source.zip"},
            {"command":"add","catalog":"catalog.json","name":"Fixture","packages":["fixture.zip"],"source_package":"source.zip","origin_project":"Fixture","origin_url":"https://example.invalid","origin_commit":"abcdef0","vendors":[]}
        ]});
        let path = fixture.0.join("plan.json");
        std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let out = fixture.0.join("release");
        run(&path, &out, Some(Path::new("missing.key")), true).unwrap();
        assert!(!out.exists());
        assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 1);
    }
}
