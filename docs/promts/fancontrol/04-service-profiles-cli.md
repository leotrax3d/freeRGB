# Prompt 04 — Boot-time service, cooling profiles and CLI

Read `00-common-rules.md` first. Branch: `cooling/service-profiles`.
Depends on prompt 02.

## Goal
Fans follow the configured curves from boot without a logged-in user or GUI,
configurations can be switched and shared, and everything is scriptable.

## Tasks
1. **Boot-time execution:** the protected service (today C#, see
   `docs/COOLING_RUNTIME.md`) runs the complete controller set from prompt 02,
   incl. composite curves, not only simple curves. Decide and document whether
   evaluation moves into the Rust daemon running as a service or stays in the
   C# worker; no duplicated curve logic (one evaluator, shared via the existing
   bridge). Login/service modes selectable; headless operation.
2. **Fault supervision & recovery acceptance prep:** independent supervisor
   covers service/boot/driver hang; maintenance drain for upgrade/uninstall/
   rollback (`docs/COOLING_MAINTENANCE.md`). Simulated tests for each path.
3. **Cooling profiles:** complete configuration (curves, sensors, bindings,
   limits) as profiles; switch manually, by tray, by hotkey and by application
   rules (reuse RGB app-profile infrastructure); import/export JSON with schema
   version and identity remapping on another board.
4. **CLI:** `freergb cooling ...` for list/status/set manual/apply profile/
   import/export/stop-all, scriptable output (`--json`).

## Done when
Simulated boot path test passes; profile switching and import/export covered by
tests; CLI documented in `docs/LOCAL_API.md`; owner test steps for
reboot/upgrade/uninstall listed for prompt 07.
