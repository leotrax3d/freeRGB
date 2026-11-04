# Cooling plugins (API 3)

Third parties can add temperature/RPM sensors and fan or pump controls (AIOs,
fan hubs, laptop ECs) without changing the core, like FanControl's plugin
ecosystem. A cooling plugin is a hardware module package of **schema 3** that
speaks **protocol API 3**. It reuses the module system unchanged
([HARDWARE_MODULES.md](HARDWARE_MODULES.md)): signed catalog, verified atomic
install, rollback, re-hash before every start, separate process with an empty
environment, daemon-owned device handle and brokered reports.

| Piece | Where |
|---|---|
| Protocol, policy, SDK | `crates/freergb-module-api/src/cooling.rs`, `cooling/sdk.rs` |
| Package schema 3, catalog schema 3, store `install_cooling`/`verified_cooling` | `crates/freergb-modules/src/{package,catalog,store,setup}.rs` |
| Host (policy, lease, floor, pump, freshness, fail-safe) | `crates/freergb-modules/src/cooling_host.rs` |
| Daemon supervisor and `cooling.plugins.*` API | `crates/freergb-daemon/src/cooling_plugins.rs` |
| Release tool | `freergb-module-tool pack --cooling-policy policy.json` |
| Reference plugins | `modules/cooling-virtual` (CI), `modules/corsair-link-cooling` |
| GUI | Devices → *Cooling plugins* (install/enable), Cooling → *Plugin fans and sensors* |

## Signed policy

The package manifest (`module.json`, schema 3, `api: [3]`, no `devices`)
carries a `cooling` policy. It is covered by the package digest in the signed
catalog, so a plugin cannot widen it.

```json
{
  "capability": "sensor | control | both",
  "sensors":  [{"id":"liquid","name":"Liquid","unit":"celsius|rpm|percent","min":0,"max":100}],
  "channels": [{"id":"fan1","name":"Fan 1","role":"fan|pump|unknown","floor_percent":40,"tachometer":"fan1_rpm"}],
  "io": {"transport":"none"} | {"transport":"hid","devices":[{"vid":…,"pid":…,"interface"?,"usage_page"?}],
          "rules":[{"op":"write|read|send_feature|get_feature","prefix":[…],"max_len":N}],
          "failsafe":[{"op":"write|send_feature","data":[…]}]},
  "sample_interval_ms": 250..5000,
  "max_age_ms": sample_interval..10000
}
```

Validation (`Policy::validate`): ids unique across sensors and channels;
sensor ranges inside the unit's physical bounds; fan floors 40..100 (plugins get
no unrestricted tier); **pump and unknown channels must declare floor 100**;
a tachometer must name an RPM sensor; ≤64 sensors, ≤32 channels, ≤64 rules.
Any write-capable rule requires at least one fail-safe report, and every
fail-safe report must itself pass the rules. `transport: none` plugins are
*virtual*: they never receive a device handle and are labelled as such.

## Protocol

JSON lines on stdin/stdout as in v1, `"v":3`:

| Host request | Answer | Plugin I/O |
|---|---|---|
| `hello` | `{id,version,family,api:3}` — must match the manifest | no |
| `describe` | `{sensors,channels}` — must equal the signed policy | no |
| `sample` | `{sensors:[{id,value|null}]}` — exactly every declared sensor | yes |
| `set_duty {channel,duty_percent,sequence,lease_ms}` | `true` | yes |
| `release {channel}` | `true` (back to firmware/automatic) | yes |
| `shutdown` | `true`, then exit | yes |

Transport calls are the v1 shape with `"v":3`
(`{"v":3,"call":N,"op":"write","data":"<base64>"}`), strictly increasing call
ids, ≤512 per request. Deadlines: 2 s per request, 4 s for a HID `sample`.
The SDK (`cooling::sdk::run`) calls `shutdown` on EOF, so a vanished daemon still
releases the device.

## Host safety (same rules as built-in channels)

* **Policy-limited I/O.** Every call is checked against the rules before it
  reaches the device; I/O during `hello`/`describe`, an oversized or
  non-matching report, a replayed call id or a non-JSON line faults the plugin.
  The plugin never sees paths or serial numbers and gets no device permissions.
* **Lease.** Each duty command is a `LeaseGuard` lease (`LEASE_MS` = 2000 ms,
  `freergb-control::cooling_safety`) issued at the host's clock. The daemon
  renews it every 500 ms; if renewals stop, `poll` sends 100 % and then
  `release` (firmware control).
* **Floor and pumps.** Duty below the declared floor is refused; pumps and
  unknown channels accept only 100 %. The daemon additionally clamps every
  composite output to `floor..100` and uses 100 % for a missing output.
* **Freshness.** Readings older than `max_age_ms` are reported missing. A
  channel whose tachometer is stale cannot be reduced, and an active reduction
  falls back to 100 % and firmware control.
* **Malformed output.** Undeclared, duplicate or missing sensors, non-finite or
  out-of-range values and invalid JSON fault the plugin; nothing partial is
  published.
* **Fail-safe on crash/hang/violation.** The host kills the process first,
  then writes the policy's fail-safe reports *itself* on the daemon-owned
  handle (`FailsafeOutcome` in the status). A hung or malicious plugin cannot
  block this. Assignments of a faulted plugin are dropped.
* **Restarts.** The daemon restarts a faulted plugin with 2..60 s back-off; after
  5 consecutive failures it stays stopped until the user re-enables it.
* **Device ownership.** A HID plugin only gets an interface that no built-in
  adapter and no configured hardware module drives.

Not covered (same as hardware modules): no OS sandbox — a plugin runs with the
user's privileges and could open devices itself; authenticity rests on the
signing key. A daemon crash leaves fail-safe to the plugin's own EOF handling.

## Daemon API

| Method | Params | Result |
|---|---|---|
| `cooling.plugins.list` | – | `{started, plugins:[{id,version,family,license,installed,enabled,capability,virtual,device_found,status,error,install,failures,sensors,channels:[{channel,assignment,applied_percent,fault}]}]}`; catalog entries not yet installed have `installed:false` |
| `cooling.plugins.install` | `{id}` | `true`; background download + verified install from the signed catalog; outcome in `install` |
| `cooling.plugins.enable` | `{id, enabled}` | `{enabled}`; persisted in `cooling-plugins.json`; re-enabling clears the restart limit |
| `cooling.plugins.set` | `{plugin, channel, mode:"manual", percent}` or `{…, mode:"composite", output}` | `true`; requires safety onboarding |
| `cooling.plugins.release` | `{plugin?, channel?}` | `{released}` |

Plugin sensors appear in `monitoring.sensors.list` and the curve source catalog
as `provider:"plugin"`, id `plugin:<plugin>/<sensor>`; plugin channels appear
in `cooling.channels.list` with `provider:"plugin"` and
`physical_mapping:"plugin_declared"`. The GUI shows them with a
*Plugin · &lt;id&gt;* source badge.

## Writing a plugin

1. Write the policy JSON (or generate it, as `corsair-link-cooling` does).
2. Implement `cooling::sdk::CoolingPlugin` (`info`, `describe` = policy,
   `sample`, optional `set_duty`/`release`, `shutdown`) and call
   `cooling::sdk::run`. Use only the brokered `Hid`.
3. Pack: `freergb-module-tool pack --cooling-policy policy.json …` (no
   `--devices`), then `add`/`sign` as for hardware modules; the catalog
   becomes schema 3.

## Reference plugins

* **`cooling-virtual`** — liquid loop with one fan (floor 40) and one pump; no
  I/O; its own lease self-expiry returns to a simulated firmware 60 %. Runs in
  CI through the full signed-catalog → install → verified start path
  (`cargo test -p freergb-modules --test cooling_e2e -- --ignored`).
* **`corsair-link-cooling`** — CORSAIR iCUE LINK hub (`1b1c:0c3f`), sensor-only:
  RPM and temperatures of 16 child slots. Built from the existing
  `freergb-protocols::corsair_bragi::link_hub` builders. Each sample opens a
  software window only for the endpoint reads and verifiably returns to
  hardware mode; the policy allows exactly those nine packets plus reads, and
  its fail-safe is the hardware-mode write. Simulation-tested only. The
  built-in LINK adapter is opt-in (`enable_link_hub`); while it is active the
  plugin does not get the hub.

## Verification

| Area | Evidence |
|---|---|
| Policy validation, allowlist, wire format | `cargo test -p freergb-module-api` |
| Real child processes: healthy cycle, pump/floor/stale tachometer, lease expiry, crash (sample/set), hang deadline, malformed output (range, undeclared, garbage), policy violations (denied report, I/O in `describe`, policy mismatch), host fail-safe written | `cargo test -p freergb-modules --test cooling_host` |
| Signed schema-3 catalog → download → install → verified virtual plugin | `cargo test -p freergb-modules --test cooling_e2e -- --ignored` (CI) |
| LINK plugin against a policy-enforcing simulated hub | `cargo test -p freergb-cooling-corsair-link` |
| GUI state, duty choices, badges | `npm test` in `apps/desktop` |

### Owner hardware steps (prompt 07)

*Not performed yet.* With the owner present, iCUE closed, `enable_link_hub`
off:

1. Build and locally sign `corsair-link-cooling` (`pack --cooling-policy
   modules/corsair-link-cooling/cooling-policy.json`, `add`, `sign`), start
   `freergbd --module-source <dir> --module-dev-key <hex>`, install and enable
   it in Devices → Cooling plugins.
2. Confirm the plugin starts, the hub is matched, and slot RPM/temperatures
   match `freergb link-probe --software-window` and iCUE's values (pump RPM,
   liquid temperature, each fan).
3. Confirm the hub returns to hardware mode after every sample (fans/pump keep
   following the hub's own curves, lighting unaffected); read the mode back with
   `link-probe`.
4. Kill the plugin process during a sample (Task Manager) and confirm the host
   reports `failsafe 1/1` and the hub is in hardware mode afterwards.
5. Unplug/replug the hub: plugin restarts with back-off, readings return.
