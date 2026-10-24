# freeRGB 0.1.0-beta.3

Bug-fix beta on top of [0.1.0-beta.2](RELEASE_NOTES_0.1.0-beta.2.md).

## Fixed

- Setup no longer skips the motherboard fan and sensor components with
  "The original signed-in user could not be identified". The elevated setup
  now resolves the signed-in user of its session itself (owner of the
  session's `explorer.exe`) instead of asking a non-elevated helper to write
  into Inno Setup's admin-only temporary folder.
