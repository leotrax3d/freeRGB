#!/usr/bin/env python3
"""Release metadata/artifact preflight; never publishes or executes an installer."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def versions(root=ROOT, tag=None):
    workspace = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]
    version = workspace["package"]["version"]
    source_workspace = tomllib.loads((root / "modules/razer-chroma/source-workspace.toml").read_text())["workspace"]
    if source_workspace["package"] != workspace["package"]:
        raise ValueError("Module source archive workspace metadata is stale")
    inherited = {"freergb-desktop"}
    for member in workspace["members"]:
        package = tomllib.loads((root / member / "Cargo.toml").read_text())["package"]
        if isinstance(package.get("version"), dict) and package["version"].get("workspace"):
            inherited.add(package["name"])
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[a-z]+\.\d+)?", version):
        raise ValueError("Application version must be a stable or numbered prerelease SemVer")
    for path in ("apps/desktop/package.json", "apps/desktop/package-lock.json", "apps/desktop/src-tauri/tauri.conf.json"):
        data = json.loads((root / path).read_text())
        if data["version"] != version or ("packages" in data and data["packages"][""]["version"] != version):
            raise ValueError(f"Version mismatch: {path}")
    if tomllib.loads((root / "apps/desktop/src-tauri/Cargo.toml").read_text())["package"]["version"] != version:
        raise ValueError("Tauri Rust version differs from workspace")
    for path in ("Cargo.lock", "apps/desktop/src-tauri/Cargo.lock"):
        for package in tomllib.loads((root / path).read_text())["package"]:
            if package["name"] in inherited and package["version"] != version:
                raise ValueError(f"Stale local package version in {path}: {package['name']}")
    if tag and tag != f"v{version}":
        raise ValueError("Tag differs from workspace version")
    if not (root / f"docs/RELEASE_NOTES_{version}.md").is_file():
        raise ValueError("Release notes for the candidate are missing")
    keys = [line.strip() for line in (root / "crates/freergb-modules/keys/release-keys.txt").read_text().splitlines() if line.strip() and not line.lstrip().startswith("#")]
    if not keys or any(not re.fullmatch(r"[0-9a-f]{64}", key) for key in keys):
        raise ValueError("A valid module release trust anchor must be configured")
    return version


def artifacts(directory, version, expected_commit=None):
    directory = Path(directory)
    name = f"freeRGB-{version}-windows-x86_64-setup.exe"
    manifest_name = f"freeRGB-{version}-windows-x86_64-build.json"
    expected = {name, name + ".sha256", manifest_name}
    present = {p.name for p in directory.iterdir()}
    # The detached update signature is optional here: it is produced offline with the release key.
    if present - {name + ".sig"} != expected:
        raise ValueError("Distribution directory contains missing, stale or unexpected artifacts")
    if name + ".sig" in present and (directory / (name + ".sig")).stat().st_size != 64:
        raise ValueError("Installer signature must be 64 raw bytes")
    installer = directory / name
    digest = hashlib.sha256(installer.read_bytes()).hexdigest()
    if (directory / (name + ".sha256")).read_text().strip() != f"{digest}  {name}":
        raise ValueError("Installer checksum sidecar mismatch")
    manifest = json.loads((directory / manifest_name).read_text())
    if manifest["version"] != version or not re.fullmatch(r"[0-9a-f]{40}", manifest["commit"]):
        raise ValueError("Artifact provenance/version is invalid")
    if expected_commit and manifest["commit"] != expected_commit:
        raise ValueError("Artifacts were built from another source commit")
    if manifest["source_dirty"] is not False:
        raise ValueError("Release artifacts must come from a clean committed tree")
    if manifest["installer"] != {"file": name, "sha256": digest, "size": installer.stat().st_size}:
        raise ValueError("Installer differs from its build manifest")
    if not manifest["payload"] or any(not re.fullmatch(r"[0-9a-f]{64}", value) for value in manifest["payload"].values()):
        raise ValueError("Payload fingerprints are missing or invalid")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag")
    parser.add_argument("--artifacts", type=Path)
    options = parser.parse_args()
    try:
        version = versions(tag=options.tag)
        if options.artifacts:
            commit = subprocess.check_output(["git", "-C", str(ROOT), "rev-parse", "HEAD"], text=True).strip()
            artifacts(options.artifacts, version, commit)
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f"Release preflight failed: {error}\n")
    print(f"PASS: release preflight for {version}" + (" and immutable artifacts" if options.artifacts else ""))
