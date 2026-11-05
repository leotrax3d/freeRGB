# Installation, releases and updates

Windows setup is built from `packaging/windows/freergb.iss` using
`build-installer.ps1`. The GitHub Package workflow produces an installer, SHA-256
sidecar and build manifest. A matching `v<workspace version>` tag can create a
**draft** release; manual runs upload artifacts. Nothing automatically publishes a
release. The current `0.1.0-beta.2` candidate is unpublished; the existing beta.1
Package run failed before packaging and its tag must not be reused.

Setup installs machine-wide under Program Files and requests UAC elevation.
Shortcuts launch the modern GUI, Classic fallback or virtual demo; login startup
runs the background components. Interactive setup prepares motherboard dependencies
and the protected service. Silent setup skips that preparation unless the exact
`/WITHMAINBOARD` argument is present. Dependency downloads require pinned hashes
and Windows publisher verification. Fan control retains separate consent and
channel-identification gates; installing prerequisites does not certify hardware.

Setup refuses a legacy per-user installation before replacing files: remove it
through Windows Apps first. Elevated setup never runs HKCU vendor-uninstaller
commands. Upgrade/removal uses guarded shutdown of the installed daemon and an
exact-path tray helper; unrelated copies remain running. Failure to stop the
installed components prevents removal. Profiles under `%APPDATA%\freeRGB` survive
uninstallation. See `INSTALL.md` for lifecycle tests and `MAINBOARD_INSTALLATION.md`
for protected-service behavior.

Both GUIs use `freergb-update` and the fixed `leotrax3d/FreeRGB` GitHub repository.
Stable builds use the latest stable release. Prerelease builds examine up to 100
releases, accept only the same prerelease channel or promotion to stable, and
select by SemVer. Drafts, rolling/non-version tags, other prerelease channels and
downgrades are excluded. A newer eligible release with invalid asset metadata
fails closed rather than silently falling back. The exact versioned Windows
installer and GitHub asset SHA-256 digest are required.

HTTPS downloads are bounded to 256 MiB and use exclusively created staging files.
The installer is hashed before staging and again before launch. On Windows the
verified file handle denies modification/deletion through the synchronous launch
handoff; DOS/UNC paths support spaces and Unicode. `ShellExecuteW` with `runas`
requests UAC explicitly. Cancellation/failure returns an error; installation remains
a visible user action. Native UAC acceptance of this candidate is still required.

Classic's automatic check/download is opt-in (startup and every six hours while
running). Its preference is `automatic-updates.json`; staged files live in the
per-user config `updates/` directory. Modern Settings → Updates exposes manual
check/download/install and rejects concurrent update operations. It has no periodic
background update check. No Git checkout is modified.

`FREERGB_GITHUB_TOKEN` may supply authorized API access and is not persisted by the
application. Access/not-found errors do not imply that the repository is private.
There is no completed live version-to-version acceptance of the current candidate.

Release tooling pins Rust, Node, action commits and Inno Setup. Preflight checks
version/tag parity, release notes, trust anchors, exact artifact set, checksums and
clean source provenance. The build manifest records the source commit, tools,
installer Authenticode status and shipped executable hashes. The module source
archive is independently built with a normalized lock containing only original
package versions/checksums. This is not a claim of bit-identical executable builds.

No freeRGB publisher Authenticode certificate is configured. SHA-256 and GitHub
release metadata provide integrity within that distribution channel, not an
independent publisher signature. SmartScreen, signed publication, cross-version
upgrade/rollback and native installer/UAC validation remain release decisions or
gates described in `BETA_READINESS_AUDIT.md`.

## Failure recovery simulation (WS4)

`cargo test -p freergb-update --locked` exercises the production transfer and
verification functions with interrupted readers, short/oversized/corrupt bytes,
and a short write followed by simulated disk-full failure. Failed transfers remove
their temporary file and preserve any existing verified destination. A leftover
partial file cannot pass verification as the destination; tampering after staging
is rejected by the same verification used before launch.

A separate disk-backed installer model upgrades synthetic 0.1.0 bytes to 0.2.0,
rejects a partial image before swapping, restores the exact old bytes after a
simulated failed start, and succeeds on retry. It executes no installer or child
process. This is model evidence, not implementation or acceptance of automatic
rollback in Windows Setup. Installer SHA-256 checks are not publisher signatures;
bad Ed25519 signatures are tested for module catalogs separately.

### Owner acceptance steps

On a disposable Windows VM, retain a snapshot and install the previous released
version with test profiles. Upgrade through the visible updater to the next
version, confirm the reported version and preserved profiles, then exercise UAC
cancellation, interrupted download and insufficient disk space. Check that no
partial installer launches. With the installer maintainer, inject a failed new
start and verify the documented recovery procedure and prior profile/image state.
Record results before claiming real cross-version or rollback acceptance. No such
installation, service change or hardware write was performed for WS4.
