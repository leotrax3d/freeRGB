#!/usr/bin/env bash
# Build only; never installs files or starts services.
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo build --release --locked -p freergb-daemon -p freergb-cli -p freergb-gui -p freergb-screen --bins
bash packaging/linux/package.sh "${1:-deb}"
