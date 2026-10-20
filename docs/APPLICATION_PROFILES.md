# Profiles by foreground application

Windows: under Lighting → Profiles → **Profiles by foreground application**, add
an executable filename such as `game.exe`, select an existing saved profile and
save the rules. Matching is case-insensitive, by filename only; different programs
with the same filename share a rule. Paths and wildcards are rejected. At most
64 rules can be saved in the per-user `application-profiles.json` file.

Enable the feature explicitly for the current GUI session. Activation is never
persisted. The status bar provides a pause button; manual profile loading also
pauses automation. Rules can be edited only while paused. Closing the GUI stops
automation. The last lighting profile remains active; no automatic restore or
fallback profile is applied on disable, unmatched applications or exit.

Foreground detection runs on a bounded background worker at most once a second.
The same executable must be observed for two seconds before choosing its profile.
Brief focus changes do not load profiles. A stable executable is handled once, so
manual adjustments are retained until another application becomes stable. While
a macro is reported running, selection is deferred. Missing focus resets the
stability timer. Missing profiles produce a local error without repeated loads
on every poll. Daemon disconnection disables the feature; explicitly enable again
after reconnecting.

Before an automatic load the API worker checks that no macro is reported running,
the same executable is still foreground and the session remains enabled. Pausing
invalidates queued work; an API call already dispatched can finish. Regulator/editor
values are synchronized after the daemon confirms the profile load, rather than
assuming the next unrelated snapshot contains the new profile.

The Windows adapter uses `active-win-pos-rs`. Its native result includes window
metadata; freeRGB extracts the executable filename and discards the rest. There
is no activity history, title/path logging, upload, process launch or keyboard
injection. Only user-created executable/profile rules are persisted. Detection
can be unavailable for protected windows, permission boundaries or unusual apps;
these cases do not choose a guessed application. Linux activation is disabled.

Tests use simulated executable observations and time to cover rapid focus changes,
stable focus, re-entry, running-macro deferral, missing focus, duplicate/invalid rules
and invalidated automatic commands. Tests do not inspect real foreground windows
or load profiles into real hardware. Interactive native acceptance remains open.
