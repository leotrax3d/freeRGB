#!/usr/bin/env python3
"""Generate THIRD_PARTY_NOTICES.md from the locked dependency graphs.

Rust: `cargo metadata --locked` for the root workspace and the desktop (Tauri)
workspace, filtered to the shipped target platforms, following normal
dependency edges only (no dev or build dependencies). Crates that live in this
repository (except vendor/) are the project itself and are not listed.

npm: apps/desktop/package-lock.json, production entries only (no `dev`). License
texts are read from apps/desktop/node_modules when installed
(`npm ci --omit=dev --ignore-scripts --prefix apps/desktop`).

This is an engineering inventory, not legal advice or a compliance statement.

Usage:
  python scripts/gen-notices.py            # rewrite THIRD_PARTY_NOTICES.md
  python scripts/gen-notices.py --check    # exit 1 if the file is out of date
  python scripts/gen-notices.py --sbom     # also refresh docs/legal/SBOM.cdx.json
"""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "THIRD_PARTY_NOTICES.md"
SBOM = ROOT / "docs/legal/SBOM.cdx.json"
PLATFORMS = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
WORKSPACES = [
    ("root workspace", ROOT / "Cargo.toml", "crates/"),
    ("desktop workspace", ROOT / "apps/desktop/src-tauri/Cargo.toml", ""),
]
LICENSE_NAMES = ("LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE", "UNLICENSE", "AUTHORS", "OFL", "UFL")
LICENSE_DIRS = {"license", "licenses", "licence", "licences", "fonts", "third_party", "third-party"}
FALLBACK_TEXTS = {"@fontsource-variable/inter": ROOT / "packaging/licenses/Inter-OFL-1.1.txt"}


def run_metadata(manifest, platform):
    out = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", platform,
         "--manifest-path", str(manifest)],
        check=True, capture_output=True, text=True, cwd=ROOT,
    ).stdout
    return json.loads(out)


def is_license_file(name):
    upper = name.upper()
    return any(upper == base or upper.startswith(base + "-") or upper.startswith(base + ".")
               or upper.startswith(base + "_") for base in LICENSE_NAMES)


def read_text(path):
    return path.read_bytes().decode("utf-8", errors="replace").replace("\r\n", "\n").strip("\n")


def rust_packages():
    """Return {(name, version, source): info} for the shipped normal-dependency closure."""
    found = {}
    for label, manifest, member_prefix in WORKSPACES:
        for platform in PLATFORMS:
            meta = run_metadata(manifest, platform)
            pkgs = {p["id"]: p for p in meta["packages"]}
            nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
            roots = []
            for pid in meta["workspace_members"]:
                rel = Path(pkgs[pid]["manifest_path"]).resolve().relative_to(ROOT).as_posix()
                if rel.startswith("modules/"):
                    continue  # downloadable GPL modules ship separately (SOURCE_OFFER.md)
                if member_prefix and not rel.startswith(member_prefix):
                    continue
                roots.append(pid)
            seen, stack = set(), list(roots)
            while stack:
                pid = stack.pop()
                if pid in seen:
                    continue
                seen.add(pid)
                for dep in nodes[pid]["deps"]:
                    if any(k["kind"] is None for k in dep["dep_kinds"]):
                        stack.append(dep["pkg"])
            for pid in seen:
                pkg = pkgs[pid]
                manifest_dir = Path(pkg["manifest_path"]).resolve().parent
                try:
                    rel = manifest_dir.relative_to(ROOT).as_posix()
                except ValueError:
                    rel = None
                if rel is not None and not rel.startswith("vendor/") and pkg["source"] is None:
                    continue  # this project's own crate
                key = (pkg["name"], pkg["version"], pkg["source"] or "path:" + str(rel))
                info = found.setdefault(key, {
                    "name": pkg["name"], "version": pkg["version"],
                    "license": pkg["license"] or "",
                    "license_file": pkg.get("license_file"),
                    "authors": pkg["authors"], "repository": pkg["repository"] or pkg["homepage"] or "",
                    "dir": manifest_dir, "source": pkg["source"] or "path", "platforms": set(),
                })
                info["platforms"].add(platform)
    return sorted(found.values(), key=lambda p: (p["name"].lower(), p["version"]))


def crate_texts(info):
    """License-like files at the top of the crate directory (plus manifest license_file)."""
    files = []
    d = info["dir"]
    if d.is_dir():
        for child in sorted(d.iterdir()):
            if child.is_file() and is_license_file(child.name):
                files.append(child)
            elif child.is_dir() and child.name.lower() in LICENSE_DIRS:
                files += [f for f in sorted(child.iterdir())
                          if f.is_file() and f.suffix.lower() in ("", ".txt", ".md")]
        lf = info.get("license_file")
        if lf and (d / lf).is_file() and (d / lf) not in files:
            files.append(d / lf)
    return [(f.relative_to(d).as_posix(), read_text(f)) for f in files]


def npm_packages():
    lock = json.loads((ROOT / "apps/desktop/package-lock.json").read_text(encoding="utf-8"))
    out = []
    for path, entry in lock["packages"].items():
        if not path or entry.get("dev") or entry.get("link"):
            continue
        name = path.split("node_modules/")[-1]
        texts = []
        pdir = ROOT / "apps/desktop" / path
        if pdir.is_dir():
            texts = [(f.name, read_text(f)) for f in sorted(pdir.iterdir())
                     if f.is_file() and is_license_file(f.name)]
        if not texts and name in FALLBACK_TEXTS:
            texts = [(FALLBACK_TEXTS[name].name, read_text(FALLBACK_TEXTS[name]))]
        out.append({
            "name": name, "version": entry["version"], "license": entry.get("license", ""),
            "resolved": entry.get("resolved", ""), "optional": bool(entry.get("optional")),
            "texts": texts,
        })
    return sorted(out, key=lambda p: (p["name"].lower(), p["version"]))


def anchor(h):
    return "text-" + h[:10]


def cell(s):
    return (s or "").replace("|", "\\|").replace("\n", " ").strip()


def render(rust, npm):
    texts = {}  # sha -> (text, [users])

    def register(user, name_text):
        refs = []
        for fname, text in name_text:
            h = hashlib.sha256(text.encode("utf-8")).hexdigest()
            texts.setdefault(h, (text, []))[1].append(user)
            refs.append((fname, h))
        return refs

    lines = [
        "# Third-party notices",
        "",
        "<!-- Generated by scripts/gen-notices.py. Do not edit by hand. -->",
        "",
        "This file lists third-party components that are linked into or bundled with",
        "freeRGB binaries and installers, with their declared licenses and the license",
        "texts shipped in each package. It is generated from `Cargo.lock`",
        "(`cargo metadata --locked`, normal dependencies only, targets",
        "`x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`,",
        "`aarch64-unknown-linux-gnu`) and `apps/desktop/package-lock.json`",
        "(production entries only).",
        "",
        "Status: engineering inventory for review. It is not legal advice and makes no",
        "statement that every obligation is met. Declared license strings come from",
        "package metadata and were not independently verified. Components not covered by",
        "this generator are listed under [Not covered](#not-covered).",
        "",
        "freeRGB's own license is in `LICENSE-MIT` and `LICENSE-APACHE`; see `NOTICE`.",
        "",
        f"## Rust crates ({len(rust)})",
        "",
        "| Crate | Version | License | Authors | Repository | License texts |",
        "|---|---|---|---|---|---|",
    ]
    rust_missing = []
    for p in rust:
        refs = register(f"{p['name']} {p['version']}", crate_texts(p))
        if not refs:
            rust_missing.append(p)
        links = ", ".join(f"[{cell(f)}](#{anchor(h)})" for f, h in refs) or "none in package"
        authors = "; ".join(a.split("<")[0].strip() for a in p["authors"]) or "(not declared)"
        lines.append(f"| {cell(p['name'])} | {p['version']} | {cell(p['license']) or 'NOT DECLARED'} | "
                     f"{cell(authors)} | {cell(p['repository'])} | {links} |")
    lines += ["", f"## npm packages, desktop frontend ({len(npm)})", "",
              "| Package | Version | License | Resolved from | License texts |", "|---|---|---|---|---|"]
    npm_missing = []
    for p in npm:
        refs = register(f"{p['name']} {p['version']} (npm)", p["texts"])
        if not refs:
            npm_missing.append(p)
        links = ", ".join(f"[{cell(f)}](#{anchor(h)})" for f, h in refs) or "not captured"
        lines.append(f"| {cell(p['name'])} | {p['version']} | {cell(p['license']) or 'NOT DECLARED'} | "
                     f"{cell(p['resolved'])} | {links} |")

    lines += ["", "### Fonts", "",
              "The desktop GUI bundles the Inter variable font through the npm package",
              "`@fontsource-variable/inter`, declared **OFL-1.1**. The font license text is",
              "reproduced below (copy kept in `packaging/licenses/Inter-OFL-1.1.txt`).",
              "", "## Not covered", "",
              "- Libraries linked from the host system on Linux (for example GLib, udev,",
              "  GTK/WebKitGTK, X11, PulseAudio client libraries). They are separate packages",
              "  supplied by the distribution; the Debian package declares them as dependencies.",
              "- The LibreHardwareMonitor runtime and its NuGet dependencies, bundled by the",
              "  Windows installer build with its own notices (see `packaging/windows/`).",
              "- Downloadable hardware modules (GPL-2.0-or-later). They are not part of the",
              "  installer; see `SOURCE_OFFER.md` and each module's `PROVENANCE.md`.",
              "- Build tools, dev dependencies and the Inno Setup compiler.",
              "- The vendored `glib` backport under `vendor/glib` is listed with the Rust",
              "  crates above and also installed as `licenses/glib-LICENSE.txt` on Windows."]
    if rust_missing:
        lines += ["", "### Crates without a license file in the package", "",
                  "These crates ship no license-like file at the package root; only the declared",
                  "license expression above is available. A reviewer should fetch the upstream",
                  "repository text.", ""]
        lines += [f"- {p['name']} {p['version']} ({p['license'] or 'NOT DECLARED'})" for p in rust_missing]
    if npm_missing:
        lines += ["", "### npm packages without captured license text", "",
                  "Run `npm ci --omit=dev --ignore-scripts --prefix apps/desktop` and regenerate to capture texts.", ""]
        lines += [f"- {p['name']} {p['version']} ({p['license'] or 'NOT DECLARED'})" for p in npm_missing]

    lines += ["", "## License texts", "",
              "Identical texts are printed once, followed by the packages that carry them.", ""]
    order = sorted(texts.items(), key=lambda kv: (sorted(kv[1][1])[0].lower(), kv[0]))
    for h, (text, users) in order:
        users = sorted(set(users), key=str.lower)
        lines += [f'<a id="{anchor(h)}"></a>', "", f"### {users[0]}" + (f" and {len(users) - 1} more" if len(users) > 1 else ""),
                  "", "Applies to: " + ", ".join(users), "", "~~~text", text, "~~~", ""]
    return "\n".join(lines).rstrip("\n") + "\n", rust, npm


def sbom_update(rust, npm):
    sbom = json.loads(SBOM.read_text(encoding="utf-8"))
    rust_lic = {(p["name"], p["version"]): p["license"] for p in rust if p["license"]}
    npm_lic = {(p["name"], p["version"]): p["license"] for p in npm if p["license"]}
    filled = 0
    for comp in sbom["components"]:
        eco = next((x["value"] for x in comp.get("properties", []) if x["name"] == "freergb:ecosystem"), "")
        lic = {"cargo": rust_lic, "npm": npm_lic}.get(eco, {}).get((comp["name"], comp.get("version")))
        if not lic:
            continue
        names = [x["name"] for x in comp["properties"]]
        if "freergb:in_shipped_normal_graph" not in names:
            comp["properties"].append({"name": "freergb:in_shipped_normal_graph", "value": "true"})
        if comp.get("licenses"):
            continue
        comp["licenses"] = [{"expression": lic}]
        for prop in comp["properties"]:
            if prop["name"] == "freergb:license_review":
                prop["value"] = "declared in Cargo package metadata (cargo metadata --locked); text not verified by a lawyer"
        filled += 1
    note = ("INCOMPLETE build/payload SBOM; lockfile and manifest inventory over 14 branches and 5 tags; "
            "dev/platform alternatives included; transitive NuGet/system/runtime dependencies incomplete; "
            "no resolved dependency edges. Cargo and npm licenses are filled from package metadata where the "
            "component is in the shipped normal-dependency graph (freergb:in_shipped_normal_graph); "
            "licence texts are in THIRD_PARTY_NOTICES.md; declared licences are not independently verified")
    for prop in sbom["metadata"]["properties"]:
        if prop["name"] == "freergb:completeness":
            prop["value"] = note
    with SBOM.open("w", encoding="utf-8", newline="\n") as fh:
        fh.write(json.dumps(sbom, indent=2, ensure_ascii=False))
    return filled


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--sbom", action="store_true", help="fill NOASSERTION cargo licenses in docs/legal/SBOM.cdx.json")
    args = ap.parse_args()
    content, rust, npm = render(rust_packages(), npm_packages())
    if args.check:
        current = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if current != content:
            print("THIRD_PARTY_NOTICES.md is out of date; run scripts/gen-notices.py", file=sys.stderr)
            return 1
        print("THIRD_PARTY_NOTICES.md is up to date")
        return 0
    OUT.write_text(content, encoding="utf-8", newline="\n")
    print(f"wrote {OUT.name}: {len(rust)} crates, {len(npm)} npm packages")
    if args.sbom:
        print(f"SBOM: filled {sbom_update(rust, npm)} license fields")
    return 0


if __name__ == "__main__":
    sys.exit(main())
