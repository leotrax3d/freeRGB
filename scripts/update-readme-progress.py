#!/usr/bin/env python3
"""Update the README's plain-text progress from docs/STATUS.md."""
import argparse
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
START = "<!-- progress:start -->"
END = "<!-- progress:end -->"


def render(status):
    tasks = []
    fenced = False
    for line in status.splitlines():
        if line.lstrip().startswith(("```", "~~~")):
            fenced = not fenced
            continue
        if not fenced:
            match = re.match(r"^- \[([ xX])\] (.+)$", line)
            if match:
                tasks.append(match.group(1).lower() == "x")
    if not tasks:
        raise ValueError("STATUS.md must contain delivery tasks")
    done, total = sum(tasks), len(tasks)
    return (
        f"**Delivery checklist: {100 * done / total:.1f}% complete "
        f"— {done} of {total} milestones; {total - done} remaining.**\n\n"
        "[View completed and remaining work](docs/STATUS.md). "
        "Automatically calculated from the delivery checklist; milestones are "
        "equally weighted, not an estimate of effort or release readiness."
    )


def update(readme, status):
    if readme.count(START) != 1 or readme.count(END) != 1:
        raise ValueError("README must contain exactly one progress marker pair")
    before, rest = readme.split(START)
    _, after = rest.split(END)
    return before + START + "\n" + render(status) + "\n" + END + after


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    path = ROOT / "README.md"
    original = path.read_text(encoding="utf-8")
    result = update(original, (ROOT / "docs/STATUS.md").read_text(encoding="utf-8"))
    if args.check and result != original:
        parser.exit(1, "README progress is stale; run python scripts/update-readme-progress.py\n")
    if not args.check and result != original:
        path.write_text(result, encoding="utf-8")
    print(render((ROOT / "docs/STATUS.md").read_text(encoding="utf-8")).splitlines()[0])


if __name__ == "__main__":
    main()

