# Monitoring and sensor warnings

The Monitoring page displays cached device readings and bounded in-memory histories.
No invented CPU/GPU values or additional USB polling are used. LINK channel identity
is not yet confirmed; identify the physical channel before interpreting its value.

Under each sensor, **Configure sensor warning** adds a local rule. Open **Sensor
warnings** to choose a minimum, maximum and hysteresis in the sensor's displayed
unit. Defaults are placeholders, not recommended hardware limits. Valid edits take
effect immediately; **Save sensor warning rules** persists them in the per-user
configuration directory as `sensor-alerts.json`. At most 64 rules are supported.
Invalid rules are inactive and cannot be saved.

A value below the minimum or above the maximum must remain outside the range for
three seconds before raising a warning. Recovery also takes three seconds; a low
alarm requires minimum plus hysteresis, and a high alarm maximum minus hysteresis.
Brief excursions reset the timer. Missing/nonfinite values, disconnected devices,
changed units and sensor ages above 3000 ms are treated as unavailable. Snapshots
that stop advancing for more than five seconds also count as unavailable.

A banner on every page opens Monitoring when an unacknowledged warning is active.
Acknowledging hides the banner for that condition; the warning remains in the list.
A new condition, or recurrence after confirmed recovery, clears acknowledgement.
Rules survive missing devices and reconnects. Runtime states and acknowledgement
reset at GUI startup. Removing a rule removes its state; save to persist removal.

These are advisory warnings while the GUI is open. There are no desktop, sound,
email or network notifications, no temperature source installation and no hardware
control. They are not an independent cooling watchdog. Histories and warnings are
not included in exported diagnostic reports.

Regression tests use synthetic cached JSON readings and a simulated monotonic
clock. They cover stale/disconnected/changed-unit readings, rule validation,
debounce, hysteresis and acknowledgement recovery. No fan writes or real hardware
experiments are performed by these tests.
