# Prompt 05 — Cooling GUI parity and guided setup

Read `00-common-rules.md` first. Branch: `cooling/gui-parity`.
Depends on prompts 02 and 04.

## Goal
Using freeRGB for fans is as convenient as FanControl, in the Tauri GUI
(primary) and with the essentials in the Classic GUI.

## Tasks
1. **Guided setup wizard:** detect board/chip/channels, explain tier, run the
   guided channel test per channel, identify fans by spinning one at a time,
   pair tachometers (manual + automatic correlation), name channels, mark
   pumps, run calibration, propose a default curve per channel.
2. **Cooling dashboard:** cards per channel and curve (live duty/RPM/temp,
   tier badge, fault state), drag curve editor for all curve types incl.
   composite and adaptive, sensor graph editor, resonance ranges, limits.
3. **Tray:** current profile, quick profile switch, stop-all, fault
   notifications.
4. **Classic GUI:** composite-curve editor and profile switch (STATUS open item);
   keep it minimal, no second full implementation of the Tauri UX.
5. Mock transport data for every new view (browser preview/website demo).
6. Accessibility and screenshots in `docs/screenshots/`.

## Done when
All views work against the mock daemon and the real daemon with simulators;
`apps/desktop` lint/type/test pass; screenshots updated; owner UI walkthrough
steps listed for prompt 07.
