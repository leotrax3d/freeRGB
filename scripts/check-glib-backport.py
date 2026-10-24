#!/usr/bin/env python3
"""Check the exact upstream glib UB backport and both workspace resolutions."""
import hashlib
from pathlib import Path
import tomllib

root = Path(__file__).resolve().parent.parent
source = root / "vendor/glib/src/variant_iter.rs"
expected = "a0f5ee8acb8faa089bcdfbc9a57372609fce7654026ccef7d9a224d05a654ccc"
if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
    raise SystemExit("glib iterator differs from the verified upstream security backport")
for manifest in (root / "Cargo.toml", root / "apps/desktop/src-tauri/Cargo.toml"):
    config = tomllib.loads(manifest.read_text())
    patched = manifest.parent / config["patch"]["crates-io"]["glib"]["path"]
    if patched.resolve() != (root / "vendor/glib").resolve():
        raise SystemExit(f"{manifest}: glib backport is not selected")
    lock = tomllib.loads(manifest.with_name("Cargo.lock").read_text())
    packages = [p for p in lock["package"] if p["name"] == "glib"]
    if len(packages) != 1 or packages[0]["version"] != "0.18.5" or "source" in packages[0]:
        raise SystemExit(f"{manifest}: lockfile does not resolve the local glib backport")
print("PASS: exact upstream glib backport selected in both locked workspaces")
