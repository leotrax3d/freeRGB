# FanControl parity — prompt series

Goal: full FanControl (V284) functionality with motherboard coverage comparable
to FanControl ("almost every board"), although the owner can test on **one**
board only: ASUS PRIME X870-P WIFI, Nuvoton NCT6701D (see
[COOLING_ACCEPTANCE.md](../../COOLING_ACCEPTANCE.md)).

## How breadth is reached without many test boards

FanControl itself does not validate every board; it relies on
LibreHardwareMonitor (LHM) chip support plus its user base. freeRGB does the same:

1. **Chip families, not boards.** Support is defined per SuperIO/EC chip family
   that LHM (Windows) or a kernel hwmon driver (Linux) can drive. A board is
   supported when its chip is.
2. **Simulation per family.** Every chip family gets a faithful fake (register
   or LHM-control model incl. quirks) so all control paths are tested in CI.
3. **One real board for end-to-end acceptance.** The owner's board proves the
   whole chain (service, supervision, recovery, GUI) on real hardware.
4. **Community evidence.** A redacted hardware report lets other users confirm
   their board; confirmed reports promote a chip/board in the generated matrix.

Status words stay honest: `hardware-verified` (owner board or confirmed report),
`simulated`, `unsupported`. Never mark a board verified from simulation.

## Order

| # | Prompt | Depends on | Can run parallel with |
|---|---|---|---|
| 1 | [01-coverage-matrix-and-simulation.md](01-coverage-matrix-and-simulation.md) | — | 3 |
| 2 | [02-controller-features.md](02-controller-features.md) | 1 | 3 |
| 3 | [03-sensors-and-gpu.md](03-sensors-and-gpu.md) | — | 1, 2 |
| 4 | [04-service-profiles-cli.md](04-service-profiles-cli.md) | 2 | 3 |
| 5 | [05-gui-and-guided-setup.md](05-gui-and-guided-setup.md) | 2, 4 | — |
| 6 | [06-cooling-plugins.md](06-cooling-plugins.md) | 3 | 5 |
| 7 | [07-hardware-acceptance.md](07-hardware-acceptance.md) | 1–6 (repeat after each) | — (owner present) |

Paste [00-common-rules.md](00-common-rules.md) in front of every prompt (or tell
the agent to read it first).
