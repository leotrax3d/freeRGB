# Error handling and notifications

The existing JSON-RPC errors, tracing logs, per-device error state and worker reconnect backoff remain the foundation. Failed user actions retain their inline diagnostics and now also produce a central toast. Superseded slider writes are deliberately silent; failed writes release the local preview so subsequent device state is authoritative. Commands are not automatically replayed: repeating macros, installations or writes could have unintended effects.

## Service incidents

A service-owned journal records device worker outages, plugin initialization failures, audio/screen initialization failures, USB discovery/open errors, failed settings persistence and macro keyboard-input failures. It logs the first occurrence of each active source and stores up to 64 incidents in memory. A source is rearmed when recovery is observed. Existing device workers and hardware scanning continue their established retry behavior independently of the GUI. Worker status is sampled each engine tick; a failure that appears and recovers entirely between ticks can still exist only in the existing worker logs.

`diagnostics.get` adds `incident_session` (opaque process-session identity) and `incidents` (ordered objects containing `id`, `source`, `message`, `details`). Existing fields and error codes remain unchanged. Reads do not acknowledge or consume incidents, so multiple GUI clients receive the same history. The existing desktop poller carries these fields without another connection or polling loop. Older services without these fields remain supported.

History survives closing either GUI, but intentionally does not survive restarting the service. Logs remain the source for earlier process sessions. The current history can contain recovered incidents, and both interfaces label it accordingly. Technical details stay local and are excluded from the existing redacted diagnostic export.

## Presentation

The desktop store observes connection transitions and incident IDs. Repeated polls and snapshot/listener races do not replay incidents; a service-session change resets the cursor. More than three incoming incidents become one summary with all details. Connection loss produces a warning and recovery a confirmation. Startup IPC failures show a notification and retry connection after five seconds; failed subscriptions are cleaned up first. Optional-slice setup failures are caught instead of becoming unhandled promise rejections.

Toasts use the existing surface, typography, accent and danger tokens, at the bottom right. At most three are visible, with a bounded queue. Identical messages are suppressed for 30 seconds. Successes close after 3.5 seconds and other tones after 10 seconds; hover, keyboard focus and expanded details pause dismissal. Each toast has a dismiss button, Escape handling, a polite live region and reduced-motion support. Detailed service history remains available under Settings. The Classic GUI uses a compact expandable, scrollable history with diagnostic hover text, keeping its existing inline error handling.

Existing setup/download operations use `useAction`, so failures also reach the shared notification path. RPC errors keep their original codes and technical text; user-facing action errors provide a short explanation and expandable details. Warnings/info/success/error tones are supported without a new UI dependency.

## Verification

- Rust journal tests cover deduplication, recovery, bounded storage and simultaneous sources.
- A JSON-RPC test verifies history recorded without a GUI, technical details, stable session identity and non-consuming reads.
- Frontend tests cover simultaneous events, duplicate polling, offline backlog summaries, temporary disconnects, service restarts, older services and superseded writes.
- Run `cargo test -p freergb-daemon --lib`, `cargo check -p freergb-gui`, and, from `apps/desktop`, `npm test`, `npm run typecheck`, `npm run build`.

Physical USB failures and capture permissions still require testing on the corresponding hardware/OS. Optional feature RPC failures retain the existing unavailable-state behavior; they are not all promoted to service-wide outages or notifications.
