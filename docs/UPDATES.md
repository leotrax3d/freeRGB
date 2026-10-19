# Installation, releases and updates

Windows setup is built from `packaging/windows/freergb.iss` using
`build-installer.ps1`. The GitHub **Package** workflow builds/tests binaries and
creates the installer as a workflow artifact. A `v<workspace version>` tag creates
a **draft** release; no automatic publication. Manual workflow runs only upload
artifacts. The installer filename is `freeRGB-<version>-windows-x86_64-setup.exe`.

The application and libraries remain Rust; installer packaging uses Inno Setup.
Setup installs per user under `%LOCALAPPDATA%/Programs/freeRGB`, adds start menu
and uninstall entries, optionally a desktop shortcut and login startup (unchecked).
It does not install a system service/driver or change firmware. The launcher starts
the daemon and GUI; the Demo shortcut requests a virtual device if no daemon exists.
Setup requests daemon shutdown before upgrade and uses Restart Manager to close
running executables. Uninstall preserves profiles/config under `%APPDATA%/freeRGB`.
The setup is not automatically installed during development.

The Updates tab uses the fixed `leotrax3d/FreeRGB` GitHub repository. It checks stable
semantic versions, rejects downgrades/prereleases and requires the exact Windows
installer plus GitHub's asset SHA-256 digest. Download size is bounded to 256 MiB;
content is hashed before staging and again before starting setup. No Git checkout
is modified by the updater. Checks/downloads run on a background worker.

Automatic check/download is opt-in (every six hours while the GUI runs, plus startup).
Installation is a visible user action using the normal installer. Preference is
stored as `automatic-updates.json`; installer downloads live under the per-user
config `updates/` directory. There is no unattended replacement of running binaries.

The repository is private and currently has no published release. Set
`FREERGB_GITHUB_TOKEN` in the application environment for authorized read access;
the application does not store that token. Without access/no release, the UI reports
that condition. No repository visibility change or release publication is implied.
There has not yet been a successful live version-to-version update test.

GitHub API reference: https://docs.github.com/en/rest/releases/assets
Digest validation provides transport/artifact integrity against the selected release,
not independent publisher signing. Authenticode signing and rollback remain open.
