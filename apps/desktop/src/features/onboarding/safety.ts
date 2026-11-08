/**
 * Safety and liability notice shown in the first-run wizard. Must match the
 * daemon's `SAFETY_TEXT_VERSION` (crates/freergb-daemon/src/onboarding.rs);
 * bump both when the text changes materially so users confirm again.
 * Version 2: the universal "every channel returns to firmware" claim was
 * replaced by per-backend statements (docs/COOLING_SAFETY.md, claims table).
 *
 * Not legal advice. The wording limits liability only to the extent permitted
 * by law and keeps mandatory liability (e.g. intent, gross negligence, injury to
 * life, body or health, product liability) intact. Have it reviewed before a
 * stable release.
 */
export const SAFETY_TEXT_VERSION = 2;

export const SAFETY_TITLE = "Read this before freeRGB controls your fans";

export const SAFETY_RISKS: { title: string; text: string }[] = [
  {
    title: "Hardware damage and overheating",
    text: "freeRGB can lower fan and pump speeds. Wrong settings, wrong header assignments, untested hardware or software faults can make components overheat and be damaged or destroyed.",
  },
  {
    title: "Data loss",
    text: "Overheating can cause crashes, sudden shutdowns and corrupted files. Keep backups of important data.",
  },
  {
    title: "Fire",
    text: "Overheated or failing components, power supplies or batteries can catch fire.",
  },
  {
    title: "Serious injury and, in extreme cases, death",
    text: "A fire, burns from hot components or electrical faults can seriously injure people and, in extreme cases, be fatal.",
  },
];

export const SAFETY_TERMS: string[] = [
  "freeRGB is free, experimental open-source software provided \"as is\". Motherboard fan control is new and has not been confirmed on most hardware.",
  "freeRGB has safeguards that differ by backend and require working software, drivers and hardware. Normal fan floors are 40 %; unrestricted settings can bypass floors, including pump protection. Missing, stale or critical temperature requests 100 % in supported curve and telemetry guards. Linux hwmon and GPU fixed-speed modes also request 100 % without a fresh usable temperature or after a controller loop gap, when the controller runs again. GPU fixed-speed control currently has no production temperature source and therefore requests 100 %. Linux hwmon and GPU release, pause, normal shutdown and write-fault paths attempt firmware or driver restoration; crash journals attempt recovery only when freeRGB next starts. Failed returns remain recorded and prevent reacquisition of affected channels. Lower-tier Windows motherboard faults attempt firmware return. A verified Windows motherboard emergency attempts 100 % and keeps the reservation until explicitly resolved. No recovery or actual fan speed is guaranteed after a process, operating system, driver or power failure, or when hardware ignores commands. These safeguards reduce risk; they cannot exclude it.",
  "You use the fan and pump control at your own risk and responsibility. Watch temperatures, especially after changes, and do not leave a system unattended while testing.",
  "To the extent permitted by applicable law, the freeRGB developers are not liable for damage arising from the use of this software. This does not limit liability that cannot be excluded or limited by law, in particular for intent or gross negligence, for injury to life, body or health, or under mandatory product liability law.",
];

export const SAFETY_CONFIRMATIONS = [
  "I have read and understood these risks.",
  "I use freeRGB's fan and pump control at my own risk and responsibility, within the limits described above.",
] as const;
