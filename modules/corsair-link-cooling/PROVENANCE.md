# Provenance

Original freeRGB code (MIT OR Apache-2.0). Packet builders and decoders are
the project's own `freergb-protocols::corsair_bragi::link_hub` (behaviour
evidence documented there and in `docs/PROTOCOLS.md`). No third-party code.

Status: **simulation-tested only.** The plugin is read-only: per sample it
switches the hub to software mode only for the RPM/temperature endpoint reads
and verifiably back to hardware mode (same sequence as
`freergb link-probe --software-window`). Its signed fail-safe report is the
"hardware mode" write, which the host sends itself if the plugin crashes,
hangs or violates policy. Owner hardware validation is listed for prompt 07.
