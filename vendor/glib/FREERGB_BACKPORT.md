# glib 0.18.5 security backport

Unmodified crate sources from https://static.crates.io/crates/glib/glib-0.18.5.crate
were verified against the original Cargo.lock SHA-256:
`233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`.

The only code change is the two-line fix from the upstream maintainer:
https://github.com/gtk-rs/gtk-rs-core/pull/1343
(merge commit `05dff0ee696f9bcd8617cd48c4b812d046d440cb`).
In `src/variant_iter.rs`, `p` becomes mutable and GLib's variadic out-argument
receives `&mut p` rather than `&p`. This fixes RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g.
No GTK ABI, crate version or other upstream code is changed. Original licensing
and provenance files are retained.

Both freeRGB workspaces explicitly patch this crate. CI checks the patched source
hash and tests all affected iterator operations with release optimizations.
Remove this backport when both dependency graphs migrate to upstream glib >=0.20.
Do not suppress the advisory or change this crate's version to claim an upstream fix.
