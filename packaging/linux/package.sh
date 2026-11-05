#!/usr/bin/env bash
# Requires built Linux binaries. Stages/packages only; no host installation/hooks.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
format=${1:-deb}
case "$format" in deb|rpm|appimage|stage) ;; *) echo 'Usage: package.sh deb|rpm|appimage|stage' >&2; exit 2;; esac
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -n1)
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][a-zA-Z0-9.]+)?$ ]] || { echo 'Invalid version' >&2; exit 2; }
semver=${version%%+*}
debversion=${semver/-/~}
arch=${FREERGB_ARCH:-$(uname -m)}
case "$arch" in x86_64) debarch=amd64;; aarch64) debarch=arm64;; *) echo 'Supported architectures: x86_64, aarch64' >&2; exit 2;; esac
bin=${FREERGB_BIN_DIR:-$root/target/release}
out=${FREERGB_PACKAGE_DIR:-$root/target/linux-packages}
mkdir -p "$out"
work=$(mktemp -d "$out/stage.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
app="$work/AppDir"
for executable in freergbd freergb freergb-gui freergb-screen-capture; do
    install -Dm0755 "$bin/$executable" "$app/usr/bin/$executable"
done
install -Dm0644 "$root/packaging/linux/60-freergb.rules" "$app/usr/lib/udev/rules.d/60-freergb.rules"
install -Dm0644 "$root/packaging/linux/70-freergb-input.rules" "$app/usr/share/doc/freergb/70-freergb-input.rules.example"
# Package unit differs from the source-install unit only in its binary path.
mkdir -p "$app/usr/lib/systemd/user"
sed 's|ExecStart=%h/.cargo/bin/freergbd|ExecStart=/usr/bin/freergbd|' \
    "$root/packaging/linux/freergbd.service" > "$app/usr/lib/systemd/user/freergbd.service"
install -Dm0644 "$root/packaging/linux/freergb.desktop" "$app/usr/share/applications/freergb.desktop"
install -Dm0644 "$root/assets/icons/freergb-256.png" "$app/usr/share/icons/hicolor/256x256/apps/freergb.png"
install -Dm0644 "$root/docs/INSTALL.md" "$app/usr/share/doc/freergb/INSTALL.md"
for license in LICENSE-MIT LICENSE-APACHE; do
    install -Dm0644 "$root/$license" "$app/usr/share/licenses/freergb/$license"
done
case "$format" in
stage)
    tar -C "$app" -czf "$out/freergb-$version-$arch-stage.tar.gz" .
    ;;
deb)
    mkdir -p "$app/DEBIAN"
    # Calculate ELF dependencies on the build distro, including version floors.
    mkdir -p "$work/debian"
    printf 'Source: freergb\nSection: utils\nPriority: optional\nMaintainer: freeRGB contributors <noreply@github.com>\n\nPackage: freergb\nArchitecture: any\nDescription: freeRGB lighting\n' > "$work/debian/control"
    dependencies=$(cd "$work" && dpkg-shlibdeps -O -e"$app/usr/bin/freergbd" -e"$app/usr/bin/freergb" -e"$app/usr/bin/freergb-gui" -e"$app/usr/bin/freergb-screen-capture")
    dependencies=${dependencies#shlibs:Depends=}
    cat > "$app/DEBIAN/control" <<EOF
Package: freergb
Version: $debversion
Section: utils
Priority: optional
Architecture: $debarch
Maintainer: freeRGB contributors <noreply@github.com>
Depends: $dependencies, pulseaudio-utils
Recommends: xdg-desktop-portal, pipewire-pulse
Description: freeRGB lighting daemon, CLI and Classic desktop
 Audio monitor, portal/X11 screen capture and optional read-only evdev input.
EOF
    dpkg-deb --root-owner-group --build "$app" "$out/freergb-$version-$debarch.deb"
    ;;
rpm)
    rpmversion=${semver%%-*}
    rpmrelease=1
    if [[ "$semver" == *-* ]]; then rpmrelease="0.${semver#*-}"; fi
    top="$work/rpmbuild"
    mkdir -p "$top"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,BUILDROOT}
    tar -C "$app" -czf "$top/SOURCES/payload.tar.gz" .
    cat > "$top/SPECS/freergb.spec" <<EOF
Name: freergb
Version: $rpmversion
Release: $rpmrelease
Summary: freeRGB lighting daemon, CLI and Classic desktop
License: MIT AND Apache-2.0
Source0: payload.tar.gz
Requires: pulseaudio-utils, systemd
Recommends: xdg-desktop-portal, pipewire-pulseaudio
%description
Audio monitor, portal/X11 screen capture and optional read-only evdev input.
%prep
%build
%install
mkdir -p %{buildroot}
tar -C %{buildroot} -xzf %{SOURCE0}
%files
/usr/bin/*
/usr/lib/udev/rules.d/*
/usr/lib/systemd/user/*
/usr/share/applications/*
/usr/share/icons/hicolor/256x256/apps/*
/usr/share/doc/freergb
/usr/share/licenses/freergb
EOF
    rpmbuild --define "_topdir $top" --define '_build_id_links none' --target "$arch" -bb "$top/SPECS/freergb.spec"
    find "$top/RPMS" -name '*.rpm' -exec cp '{}' "$out/" \;
    ;;
appimage)
    # linuxdeploy must already be present, including its trusted dependencies.
    # It collects shared libraries; host portal, audio server and udev stay external.
    cat > "$app/AppRun" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
export PATH="$here/usr/bin:$PATH"
export LD_LIBRARY_PATH="$here/usr/lib:${LD_LIBRARY_PATH:-}"
case "${1:-}" in
    --daemon) shift; exec "$here/usr/bin/freergbd" "$@";;
    --cli) shift; exec "$here/usr/bin/freergb" "$@";;
    *) exec "$here/usr/bin/freergb-gui" "$@";;
esac
EOF
    chmod 0755 "$app/AppRun"
    # Host rules/user units are documented external steps, never activated here.
    rm -rf -- "$app/usr/lib/udev" "$app/usr/lib/systemd"
    cd "$out"
    OUTPUT="freeRGB-$version-$arch.AppImage" linuxdeploy --appdir "$app" \
        --executable "$app/usr/bin/freergbd" --executable "$app/usr/bin/freergb" \
        --executable "$app/usr/bin/freergb-gui" --executable "$app/usr/bin/freergb-screen-capture" \
        --desktop-file "$app/usr/share/applications/freergb.desktop" \
        --icon-file "$app/usr/share/icons/hicolor/256x256/apps/freergb.png" --output appimage
    ;;
esac
echo "Package output: $out"
