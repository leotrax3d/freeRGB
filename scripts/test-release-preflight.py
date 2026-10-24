#!/usr/bin/env python3
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import hashlib

spec = importlib.util.spec_from_file_location("release", Path(__file__).with_name("check-release.py"))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleasePreflight(unittest.TestCase):
    def test_actual_version_and_tag_guard(self):
        version = release.versions()
        self.assertEqual(release.versions(tag="v" + version), version)
        with self.assertRaises(ValueError):
            release.versions(tag="v0.0.0")

    def test_tampering_dirty_provenance_and_stale_artifacts_are_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            version = "0.1.0-beta.2"
            name = f"freeRGB-{version}-windows-x86_64-setup.exe"
            data = b"synthetic bytes; never executed"
            (root / name).write_bytes(data)
            digest = hashlib.sha256(data).hexdigest()
            (root / (name + ".sha256")).write_text(f"{digest}  {name}\n")
            path = root / f"freeRGB-{version}-windows-x86_64-build.json"
            manifest = {"version": version, "commit": "a" * 40, "source_dirty": False,
                        "installer": {"file": name, "sha256": digest, "size": len(data)},
                        "payload": {"freergbd.exe": "b" * 64}}
            path.write_text(json.dumps(manifest))
            release.artifacts(root, version)
            with self.assertRaises(ValueError):
                release.artifacts(root, version, "c" * 40)
            (root / name).write_bytes(b"tampered")
            with self.assertRaises(ValueError):
                release.artifacts(root, version)
            (root / name).write_bytes(data)
            manifest["source_dirty"] = True
            path.write_text(json.dumps(manifest))
            with self.assertRaises(ValueError):
                release.artifacts(root, version)
            manifest["source_dirty"] = False
            path.write_text(json.dumps(manifest))
            (root / "stale.exe").write_bytes(b"old build")
            with self.assertRaises(ValueError):
                release.artifacts(root, version)


if __name__ == "__main__":
    unittest.main()
