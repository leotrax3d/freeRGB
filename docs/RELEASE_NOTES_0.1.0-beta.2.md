# freeRGB 0.1.0-beta.2 — release candidate, not published

This candidate follows the existing beta.1 tag, whose Package run failed before
producing an installer. No existing tag or published asset is replaced.

- Modern and Classic GUI, local background agent, built-in effects and sandboxed
  effect marketplace; hardware modules use a compiled-in Ed25519 trust anchor.
- Fixed update elevation, beta-channel selection, concurrent installer staging,
  Unix daemon admission and setup failure/retry behavior.
- Hardened elevated installer migration and exact-path tray shutdown. Legacy
  per-user installs must be removed through Windows Apps before machine-wide setup.
- Backported the official GLib iterator soundness fix without changing GTK ABI;
  patched TLS, ZIP and Tokio dependencies.

Setup installs under Program Files and asks for elevation. Interactive setup also
prepares motherboard prerequisites; silent setup skips them unless `/WITHMAINBOARD`
is explicitly supplied. If original-user detection or maintenance fails, setup
does not silently grant the elevated administrator control of another user's service.
Uninstall preserves per-user profiles. Driver installation/UAC cancellation and
cross-version upgrade still require acceptance on disposable Windows systems.

## Release limits

- Fan/pump control remains experimental and behind the existing safety gates.
  Simulation demonstrates decisions and transport faults, not physical cooling.
- The final candidate has not been installed on native Windows in this audit.
  See `BETA_READINESS_AUDIT.md` for the explicit release gates and evidence.
- The owner approved distributing the initial beta without publisher Authenticode signing; unsigned beta
  installers may trigger SmartScreen. SHA-256/GitHub provenance is not publisher signing.
- The public hardware-module catalog must be available and signed by the compiled
  release key before advertising downloadable module support as ready.
- Community ratings/reports use Appwrite; live deployment acceptance is separate
  from the mock backend tests. Linux remains source-only for this beta.
