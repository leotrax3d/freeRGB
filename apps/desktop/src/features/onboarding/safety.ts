/**
 * Safety and liability notice shown in the first-run wizard. Must match the
 * daemon's `SAFETY_TEXT_VERSION` (crates/freergb-daemon/src/onboarding.rs);
 * bump both when the text changes materially so users confirm again.
 * Version 2: the universal "every channel returns to firmware" claim was
 * replaced by per-backend statements (docs/COOLING_SAFETY.md, claims table).
 * Version 3: wording from docs/legal/SAFETY_LIABILITY_TEXTS.md (b2): gratuitous
 * provision standard (intent, gross negligence, fraudulently concealed defects)
 * instead of a general "not liable" sentence.
 *
 * Not legal advice. The wording limits liability only to the extent permitted
 * by law and keeps mandatory liability (e.g. intent, gross negligence, injury to
 * life, body or health, product liability) intact. Have it reviewed before a
 * stable release.
 */
export const SAFETY_TEXT_VERSION = 3;

export const SAFETY_TITLE = "Read this before freeRGB controls your fans and pumps";

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
  "freeRGB is free, experimental open-source software provided by a private individual, without warranty. Motherboard fan control is new and has not been confirmed on real hardware for most boards.",
  "Normal settings request at least 40 % fan speed (or a higher backend minimum) and 100 % for pumps. \"Unrestricted\" settings can bypass these limits, including pump protection.",
  "In supported curve modes, a missing, stale or critical temperature requests 100 %. In fixed-speed (Manual) mode on Linux hwmon and GPU, a missing, invalid, stale or critical temperature, or a controller loop gap, requests 100 % - only when the controller runs again. GPU fixed-speed mode currently has no temperature source and therefore requests 100 %.",
  "On Linux hwmon and GPU, release, pause, normal shutdown and write faults attempt to return control to firmware or driver automatic mode. After a crash, recovery is attempted only when freeRGB next starts. Failed returns stay recorded and block reuse of the affected channels.",
  "On Windows motherboards, lower-tier channels attempt a return to firmware on a fault. Verified-tier channels instead request 100 % and stay reserved until the fault is explicitly resolved.",
  "A request is not proof: freeRGB cannot confirm the actual fan speed or cooling. No recovery is promised while freeRGB stays stopped, after an operating system, driver or power failure, or when drivers, firmware or hardware ignore commands. These measures may reduce risk; they cannot exclude it.",
  "You use fan and pump control at your own risk. Watch temperatures after every change and do not leave the system unattended while testing.",
  "freeRGB is provided free of charge by a private individual. To the extent permitted by law, liability is limited to intent and gross negligence and to fraudulently concealed defects. Liability for injury to life, body or health, for intent or gross negligence, and under mandatory product liability law is not limited.",
];

export const SAFETY_CONFIRMATIONS = [
  "I have read and understood these risks.",
  "I use freeRGB's fan and pump control at my own risk, within the limits described above.",
] as const;
