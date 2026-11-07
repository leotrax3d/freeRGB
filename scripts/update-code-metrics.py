#!/usr/bin/env python3
"""Update the README code-line badge using cloc 2.08.

Counts tracked Rust, TypeScript/JSX, JavaScript, Python, PowerShell, shell,
C/C++, C#, CSS and HTML, including tests. Excludes vendor dependencies,
build directories, comments and blank lines. Run with --cloc /path/to/cloc.pl.
"""

import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
LANGUAGES = "Rust,TypeScript,JavaScript,JSX,Python,PowerShell,Bourne Shell,C,C++,C#,CSS,HTML"


def render(total):
    value = f"{total:,}"
    label_width = 93
    value_width = max(50, len(value) * 7 + 16)
    width = label_width + value_width
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="20" role="img" aria-label="Lines of code: {value}">
  <title>Lines of code: {value}</title>
  <rect width="{label_width}" height="20" fill="#555"/>
  <rect x="{label_width}" width="{value_width}" height="20" fill="#10a37f"/>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="11">
    <text x="{label_width / 2}" y="14">lines of code</text>
    <text x="{label_width + value_width / 2}" y="14">{value}</text>
  </g>
</svg>
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--cloc", type=Path, help="Path to the cloc 2.08 Perl script")
    source.add_argument("--report", type=Path, help="Use an existing cloc JSON report")
    args = parser.parse_args()
    if args.report:
        report = json.loads(args.report.read_text(encoding="utf-8"))
    else:
        result = subprocess.run([
            "perl", str(args.cloc.resolve()), "--vcs=git", "--quiet", "--json",
            "--exclude-dir=vendor,node_modules,target,dist,dist-demo",
            f"--include-lang={LANGUAGES}",
        ], cwd=ROOT, check=True, capture_output=True, text=True)
        report = json.loads(result.stdout)
    total = report["SUM"]["code"]
    if not isinstance(total, int) or total <= 0:
        raise ValueError("cloc must report a positive code-line count")
    output = ROOT / "assets/badges/code-lines.svg"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(render(total), encoding="utf-8")
    print(f"{total:,} code lines across {report['SUM']['nFiles']:,} source files")


if __name__ == "__main__":
    main()
