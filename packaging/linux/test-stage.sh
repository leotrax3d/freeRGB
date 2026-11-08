#!/usr/bin/env bash
# Synthetic payload only: never executes a daemon, capture, service or installer.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
mkdir -p "$scratch/bin" "$scratch/out" "$scratch/extract"
for executable in freergbd freergb freergb-gui freergb-screen-capture; do
    printf '#!/bin/sh\nexit 97\n' > "$scratch/bin/$executable"
done
FREERGB_ARCH=x86_64 FREERGB_BIN_DIR="$scratch/bin" FREERGB_PACKAGE_DIR="$scratch/out" \
    bash "$root/packaging/linux/package.sh" stage
tar -C "$scratch/extract" -xzf "$scratch/out/"*-stage.tar.gz
for executable in freergbd freergb freergb-gui freergb-screen-capture; do
    cmp "$scratch/bin/$executable" "$scratch/extract/usr/bin/$executable"
done
grep -qx 'ExecStart=/usr/bin/freergbd' "$scratch/extract/usr/lib/systemd/user/freergbd.service"
test -f "$scratch/extract/usr/lib/udev/rules.d/60-freergb.rules"
test ! -f "$scratch/extract/usr/lib/udev/rules.d/70-freergb-input.rules"
test -f "$scratch/extract/usr/share/doc/freergb/70-freergb-input.rules.example"
test -f "$scratch/extract/usr/share/applications/freergb.desktop"
test -f "$scratch/extract/usr/share/icons/hicolor/256x256/apps/freergb.png"
test -f "$scratch/extract/usr/share/licenses/freergb/LICENSE-MIT"
test -f "$scratch/extract/usr/share/licenses/freergb/LICENSE-APACHE"
for notice in NOTICE THIRD_PARTY_NOTICES.md SOURCE_OFFER.md; do
    cmp "$root/$notice" "$scratch/extract/usr/share/licenses/freergb/$notice"
done
echo 'Synthetic Linux package staging passed'
