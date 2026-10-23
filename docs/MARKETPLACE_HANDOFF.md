# Marketplace implementation handoff

Update 2026-10-23: Appwrite is now provisioned and the adapter, download receipts,
sorting and Settings feedback/hardware preview are implemented in the
`appwrite-community` worktree. See [APPWRITE_SETUP.md](APPWRITE_SETUP.md) for the
live resources and contract. The rest of this handoff records the earlier state.

The marketplace implementation is on branch `work`. No cloud rating service has
been provisioned by this implementation.

## Implemented

- Sandboxed WASM effect host in Rust, marketplace package installation and
  activation, bundled Rust-built examples, and desktop marketplace UI.
- Publisher information, star-rating controls, and a prepared Supabase anonymous
  ratings adapter with SQL access policies. This adapter is not configured or live.
- Error-report preview with reduced device information and a GitHub issue composer;
  users submit the issue themselves.
- Contact links: freergbteam@gmail.com and
  https://www.youtube.com/channel/UCh-B-9nUgKeCNJm3ESrpR8w.

## Next work requested by the user

The user selected **Appwrite**, authorized provisioning, and reports that its
connector works in another chat. This chat had no Appwrite tools available.
Replace the prepared Supabase integration with the actual Appwrite service;
preserve anonymous app-user identity, shared averages, and per-user vote ownership.
Do not present unconfigured or fixture ratings as live data.

Also requested, not yet implemented:

- Settings action to send hardware information voluntarily, with preview before
  sending and admin-only access to submitted information.
- Feedback contact form in Settings.
- Marketplace download counts based on successful downloads, with protection
  against accidental duplicate counting and a documented update/reinstall policy.

## Validation and limitations

Rust workspace tests, workspace Clippy, native desktop tests and Clippy, frontend
tests/build, daemon process smoke tests, and PostgreSQL rating access-policy tests
passed during development. The final expanded browser automation remains
incomplete: its star-radio label locator timed out. Recheck the full user flow
after replacing the provider. No feedback, email, or GitHub issue was submitted.

See EFFECT_MARKETPLACE.md for architecture and SUPABASE_SETUP.md for the prepared
alternative that should be superseded when Appwrite is integrated.
