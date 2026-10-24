#!/usr/bin/env python3
"""Normalize a generated source archive's workspace lock without dependency drift."""
from pathlib import Path
import subprocess
import sys
import tomllib

root = Path(__file__).resolve().parent.parent
source = Path(sys.argv[1]).resolve()
if source == root.resolve():
    raise SystemExit("Use a generated source directory, not the project checkout")
original = tomllib.loads((root / "Cargo.lock").read_text())
subprocess.run(["cargo", "update", "--workspace", "--offline", "--manifest-path", str(source / "Cargo.toml")], check=True)
selected = tomllib.loads((source / "Cargo.lock").read_text())

def identity(package):
    return package["name"], package["version"], package.get("source"), package.get("checksum")

if not {identity(p) for p in selected["package"]}.issubset({identity(p) for p in original["package"]}):
    raise SystemExit("Source archive dependency versions/checksums drifted from the original build")
print(f"PASS: source workspace frozen to {len(selected['package'])} original package identities")
