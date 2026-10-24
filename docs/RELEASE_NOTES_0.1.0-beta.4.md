# freeRGB 0.1.0-beta.4

Bug-fix beta on top of [0.1.0-beta.3](RELEASE_NOTES_0.1.0-beta.3.md).

## Fixed

- ASUS Aura motherboards whose firmware reports 12V RGB headers but zero
  fixed LEDs (for example PRIME X870-P WIFI, `0b05:19af`) no longer fail with
  "unsupported Aura header/LED configuration". The RGB headers are driven on
  the fixed channel and the ARGB headers are available again.
