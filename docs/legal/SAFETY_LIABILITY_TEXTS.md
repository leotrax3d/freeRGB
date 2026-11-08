# Safety and liability texts (DRAFT, EN/DE)

> **Status: DRAFT for lawyer review. AI-generated assessment, not legal advice.**
> Nothing in this file has been approved by a lawyer or by the owner. Do not
> ship any of these texts as final before decision D5 in `OWNER_DECISIONS.md`
> is taken. Every passage marked `[LAWYER]` needs explicit review.

## Integration status (DRAFT, pending lawyer review)

- (a) integrated: `packaging/windows/disclaimer-en.txt` / `disclaimer-de.txt`
  (generated from section (a), DRAFT/`[LAWYER]` markers removed from the
  user-facing text) are the Inno `LicenseFile` of the English/German
  `[Languages]` entries in `packaging/windows/freergb.iss`; the installer cannot
  continue without accepting. The Beta label stays in `AppName`.
- (b1) integrated in Settings → About (`settings.about.disclaimer`,
  `settings.about.risk`, `settings.about.offByDefault`, `settings.about.legal`,
  EN+DE).
- (b2) integrated as English wizard text `SAFETY_TEXT_VERSION = 3` in
  `safety.ts` and `onboarding.rs` (the wizard stays English-only by design, see
  `docs/I18N.md`). Users who accepted version 2 are asked again.
- All of this stays DRAFT until decision D5 is taken. The DRAFT status is
  documented here only, not in user-facing strings.

## Basis and constraints

- Publisher (owner decision D1, as given): one private natural person;
  strictly non-commercial (no sales, paid features, support contracts,
  advertising or data monetisation); software provided free of charge.
- Defaults (D4/D8, as given): online/community features **off**; telemetry
  and download counting **off**.
- No donation, sponsoring or "support the project" wording anywhere in these
  texts (a payment relationship could change the legal assessment). `[LAWYER]`
- Every factual safety statement below is limited to the claims-to-evidence
  table in `docs/COOLING_SAFETY.md` ("Safety claims by backend") and the code
  and tests it cites. The in-app wording must stay consistent with
  `apps/desktop/src/features/onboarding/safety.ts` (`SAFETY_TEXT_VERSION = 2`)
  and `crates/freergb-daemon/src/onboarding.rs`. Adopting text (b) in the app
  is a material text change and requires bumping `SAFETY_TEXT_VERSION` in both
  places (separate code task, not done here).
- Forbidden words in all texts: "safe", "certified", "tested for board X",
  "protects your hardware", "guaranteed".

### Liability standard (AI assessment, German law) `[LAWYER]`

- Gratuitous provision of software is probably assessed by analogy to gift law
  (Schenkung). Under § 521 BGB the donor is liable only for intent and gross
  negligence; under § 524 BGB liability for defects is limited to fraudulently
  concealed defects (arglistiges Verschweigen). Whether §§ 521, 524 BGB apply
  to open-source downloads, and whether they also cover consequential damage
  to other property (e.g. hardware) or only the performance interest, is
  disputed and must be assessed by a lawyer.
- Liability that cannot be excluded or limited: intent; gross negligence
  (in standard terms, § 309 Nr. 7 b BGB); injury to life, body or health
  (§ 309 Nr. 7 a BGB); fraudulently concealed defects (§ 444 / § 524 BGB);
  the Product Liability Act (ProdHaftG, § 14), to the extent applicable;
  guarantees, if any were given (none are given).
- The texts therefore **do not** exclude liability. They state the statutory
  gratuitous standard "to the extent permitted by law" and expressly preserve
  mandatory liability. A general "no liability whatsoever" clause would likely
  be void in standard terms (§ 307, § 309 Nr. 7 BGB) and is not used.
- Whether the revised EU Product Liability Directive (2024/2853, software as
  product) applies to free, non-commercial open-source software outside a
  commercial activity: likely excluded by its recital on non-commercial FOSS,
  but this needs review. `[LAWYER]`
- The MIT/Apache-2.0/GPL warranty disclaimers in the licences remain in force
  as far as permitted; these texts do not replace them.

---

## (a) Installer disclaimer (must be accepted)

Shown as a license/info page in the installer (`packaging/windows/freergb.iss`,
Linux package post-install notice or first-run on Linux). The installer must not
continue without acceptance. Integration is a separate code task.

### EN

> **freeRGB – Beta software. Please read before installing.** *(DRAFT)*
>
> freeRGB is free, non-commercial, experimental open-source software provided
> by a private individual at no charge. There is no sale, no paid feature, no
> support contract and no warranty or guarantee of any kind.
>
> freeRGB can change lighting and, after a separate confirmation in the app,
> lower fan and pump speeds. Wrong settings, wrong header assignments, untested
> hardware, driver or firmware behaviour, or software faults can cause
> overheating, data loss, hardware damage or destruction, fire and, in extreme
> cases, serious injury or death. Fan and pump control has not been confirmed
> on real hardware for most boards and devices.
>
> Online and community features, telemetry and download counting are switched
> off by default.
>
> You install and use freeRGB at your own risk and responsibility. Because the
> software is provided free of charge, the developer is liable, to the extent
> permitted by law, only for intent and gross negligence and for defects that
> were fraudulently concealed. This does not limit liability that cannot be
> excluded or limited by law, in particular for injury to life, body or health,
> for intent or gross negligence, or under mandatory product liability law.
> `[LAWYER: §§ 521, 524 BGB analogy; scope for consequential damage]`
>
> The licence terms (MIT / Apache-2.0, and GPL-2.0-or-later for modules) apply
> in addition, including their warranty disclaimers to the extent permitted by
> law.
>
> ☐ I have read and accept these terms.

### DE

> **freeRGB – Beta-Software. Bitte vor der Installation lesen.** *(ENTWURF)*
>
> freeRGB ist kostenlose, nicht kommerzielle, experimentelle Open-Source-Software,
> die von einer Privatperson unentgeltlich bereitgestellt wird. Es gibt keinen
> Verkauf, keine kostenpflichtigen Funktionen, keinen Supportvertrag und keine
> Garantie oder Zusicherung irgendeiner Art.
>
> freeRGB kann die Beleuchtung ändern und – nach einer gesonderten Bestätigung
> in der App – Lüfter- und Pumpendrehzahlen senken. Falsche Einstellungen,
> falsche Header-Zuordnungen, ungetestete Hardware, Treiber- oder
> Firmwareverhalten oder Softwarefehler können zu Überhitzung, Datenverlust,
> Beschädigung oder Zerstörung von Hardware, Brand und im Extremfall zu schweren
> Verletzungen oder zum Tod führen. Die Lüfter- und Pumpensteuerung ist für die
> meisten Mainboards und Geräte nicht auf echter Hardware bestätigt.
>
> Online- und Community-Funktionen, Telemetrie und Download-Zählung sind
> standardmäßig ausgeschaltet.
>
> Installation und Nutzung erfolgen auf eigene Gefahr und Verantwortung. Da die
> Software unentgeltlich überlassen wird, haftet der Entwickler, soweit
> gesetzlich zulässig, nur für Vorsatz und grobe Fahrlässigkeit sowie für
> arglistig verschwiegene Mängel. Unberührt bleibt die Haftung, die gesetzlich
> nicht ausgeschlossen oder beschränkt werden kann, insbesondere für Schäden
> aus der Verletzung des Lebens, des Körpers oder der Gesundheit, für Vorsatz
> und grobe Fahrlässigkeit sowie nach zwingendem Produkthaftungsrecht.
> `[ANWALT: Analogie §§ 521, 524 BGB; Reichweite für Folgeschäden]`
>
> Zusätzlich gelten die Lizenzbedingungen (MIT / Apache-2.0 sowie
> GPL-2.0-or-later für Module) einschließlich ihrer Gewährleistungsausschlüsse,
> soweit gesetzlich zulässig.
>
> ☐ Ich habe diese Bedingungen gelesen und akzeptiere sie.

---

## (b) App texts

### (b1) About dialog

#### EN

> freeRGB {version} – **Beta** *(draft text)*
>
> Free, non-commercial open-source software by a private developer. Provided
> free of charge, without warranty. Not affiliated with any hardware vendor.
>
> Fan and pump control is experimental and not confirmed on most hardware.
> Use at your own risk. Liability is limited as described in the installation
> terms, only to the extent permitted by law; liability for injury to life, body
> or health, intent, gross negligence and under mandatory product liability law
> remains unaffected.
>
> Online/community features, telemetry and download counting: off by default.
>
> Licences and third-party notices: {link}

#### DE

> freeRGB {version} – **Beta** *(Textentwurf)*
>
> Kostenlose, nicht kommerzielle Open-Source-Software eines privaten
> Entwicklers. Unentgeltlich und ohne Gewähr bereitgestellt. Keine Verbindung zu
> Hardwareherstellern.
>
> Die Lüfter- und Pumpensteuerung ist experimentell und auf den meisten Geräten
> nicht bestätigt. Nutzung auf eigene Gefahr. Die Haftung ist wie in den
> Installationsbedingungen beschrieben beschränkt, nur soweit gesetzlich
> zulässig; die Haftung für Verletzung von Leben, Körper oder Gesundheit, für
> Vorsatz, grobe Fahrlässigkeit und nach zwingendem Produkthaftungsrecht bleibt
> unberührt.
>
> Online-/Community-Funktionen, Telemetrie und Download-Zählung: standardmäßig
> aus.
>
> Lizenzen und Hinweise zu Drittsoftware: {link}

### (b2) Before enabling fan/pump control (wizard, text version 3 proposal)

Replaces/extends `safety.ts` version 2. Each factual sentence carries its
evidence row from `docs/COOLING_SAFETY.md` (ID in brackets, not shown to users).

Evidence IDs:
- **E1** floors 40 % / pump 100 % / unrestricted bypass (hwmon, GPU, Windows MB)
- **E2** hwmon release/pause/shutdown/fault attempt firmware restore
- **E3** GPU release/pause/shutdown/write fault attempt driver auto
- **E4** crash journal: recovery only when freeRGB next starts; failures stay journaled
- **E5** missing/stale/critical temperature requests 100 % (hwmon/GPU curves, Windows telemetry guard)
- **E6** verified Windows tier: fault requests 100 %, keeps reservation, no firmware return until resolved
- **E7** lower Windows tiers: fault attempts firmware return
- **E8** Manual mode: 100 % on missing/invalid/stale/critical temp or loop gap, when evaluation runs; GPU Manual has no production temperature, so requests 100 %
- **E9** durable journal required; failed restore blocks reacquisition
- **E10** "Not promised" paragraph and hardware-acceptance status

#### EN

> **Read this before freeRGB controls your fans and pumps** *(DRAFT)*
>
> **Risks.** freeRGB can lower fan and pump speeds. Wrong settings, wrong
> header assignments, untested hardware or software faults can make components
> overheat and be damaged or destroyed, cause crashes and data loss, fire and,
> in extreme cases, serious injury or death. Keep backups.
>
> **What freeRGB attempts – and what it does not promise.**
> - Normal settings request at least 40 % fan speed (or a higher backend
>   minimum) and 100 % for pumps. "Unrestricted" settings can bypass these
>   limits, including pump protection. [E1]
> - In supported curve modes, a missing, stale or critical temperature requests
>   100 %. In fixed-speed (Manual) mode on Linux hwmon and GPU, a missing,
>   invalid, stale or critical temperature, or a controller loop gap, requests
>   100 % – only when the controller runs again. GPU fixed-speed mode currently
>   has no temperature source and therefore requests 100 %. [E5, E8]
> - On Linux hwmon and GPU, release, pause, normal shutdown and write faults
>   attempt to return control to firmware or driver automatic mode. After a
>   crash, recovery is attempted only when freeRGB next starts. Failed returns
>   stay recorded and block reuse of the affected channels. [E2, E3, E4, E9]
> - On Windows motherboards, lower-tier channels attempt a return to firmware on
>   a fault. Verified-tier channels instead request 100 % and stay reserved until
>   the fault is explicitly resolved. [E6, E7]
> - A request is not proof: freeRGB cannot confirm the actual fan speed or
>   cooling. No recovery is promised while freeRGB stays stopped, after an
>   operating system, driver or power failure, or when drivers, firmware or
>   hardware ignore commands. Motherboard fan control has not been confirmed on
>   real hardware for most boards. These measures may reduce risk; they cannot
>   exclude it. [E10]
>
> **Your responsibility.** You use fan and pump control at your own risk. Watch
> temperatures after every change and do not leave the system unattended while
> testing.
>
> **Liability.** freeRGB is provided free of charge by a private individual.
> To the extent permitted by law, liability is limited to intent and gross
> negligence and to fraudulently concealed defects. Liability for injury to
> life, body or health, for intent or gross negligence, and under mandatory
> product liability law is not limited. `[LAWYER]`
>
> ☐ I have read and understood these risks.
> ☐ I use freeRGB's fan and pump control at my own risk, within the limits
> described above.
> [Enable fan control] (disabled until both boxes are ticked)

#### DE

> **Bitte lesen, bevor freeRGB Ihre Lüfter und Pumpen steuert** *(ENTWURF)*
>
> **Risiken.** freeRGB kann Lüfter- und Pumpendrehzahlen senken. Falsche
> Einstellungen, falsche Header-Zuordnungen, ungetestete Hardware oder
> Softwarefehler können Bauteile überhitzen und beschädigen oder zerstören,
> Abstürze und Datenverlust, Brand und im Extremfall schwere Verletzungen oder
> den Tod verursachen. Legen Sie Sicherungskopien an.
>
> **Was freeRGB versucht – und was es nicht verspricht.**
> - Normale Einstellungen fordern mindestens 40 % Lüfterdrehzahl (oder ein
>   höheres Backend-Minimum) und 100 % für Pumpen an. „Uneingeschränkte“
>   Einstellungen können diese Grenzen umgehen, auch den Pumpenschutz. [E1]
> - In unterstützten Kurvenmodi fordert eine fehlende, veraltete oder kritische
>   Temperatur 100 % an. Im Festwert-Modus (Manual) unter Linux hwmon und GPU
>   fordern eine fehlende, ungültige, veraltete oder kritische Temperatur oder
>   eine Lücke in der Regelschleife 100 % an – erst, wenn der Regler wieder
>   läuft. Der GPU-Festwert-Modus hat derzeit keine Temperaturquelle und fordert
>   daher 100 % an. [E5, E8]
> - Unter Linux hwmon und bei GPUs versuchen Freigabe, Pause, normales Beenden
>   und Schreibfehler, die Steuerung an Firmware bzw. Treiberautomatik
>   zurückzugeben. Nach einem Absturz wird die Wiederherstellung erst beim
>   nächsten Start von freeRGB versucht. Fehlgeschlagene Rückgaben bleiben
>   gespeichert und sperren die erneute Nutzung der betroffenen Kanäle.
>   [E2, E3, E4, E9]
> - Bei Windows-Mainboards versuchen Kanäle niedrigerer Stufen bei einem Fehler
>   die Rückgabe an die Firmware. Kanäle der Stufe „verifiziert“ fordern
>   stattdessen 100 % an und bleiben reserviert, bis der Fehler ausdrücklich
>   aufgelöst wird. [E6, E7]
> - Eine Anforderung ist kein Nachweis: freeRGB kann die tatsächliche Drehzahl
>   oder Kühlung nicht bestätigen. Keine Wiederherstellung wird versprochen,
>   solange freeRGB beendet bleibt, nach Betriebssystem-, Treiber- oder
>   Stromausfall oder wenn Treiber, Firmware oder Hardware Befehle ignorieren.
>   Die Mainboard-Lüftersteuerung ist für die meisten Boards nicht auf echter
>   Hardware bestätigt. Diese Maßnahmen können das Risiko verringern, aber nicht
>   ausschließen. [E10]
>
> **Ihre Verantwortung.** Sie nutzen die Lüfter- und Pumpensteuerung auf eigene
> Gefahr. Beobachten Sie nach jeder Änderung die Temperaturen und lassen Sie das
> System beim Testen nicht unbeaufsichtigt.
>
> **Haftung.** freeRGB wird von einer Privatperson unentgeltlich bereitgestellt.
> Soweit gesetzlich zulässig, ist die Haftung auf Vorsatz und grobe
> Fahrlässigkeit sowie auf arglistig verschwiegene Mängel beschränkt. Die
> Haftung für Verletzung von Leben, Körper oder Gesundheit, für Vorsatz oder
> grobe Fahrlässigkeit und nach zwingendem Produkthaftungsrecht bleibt
> unbeschränkt. `[ANWALT]`
>
> ☐ Ich habe diese Risiken gelesen und verstanden.
> ☐ Ich nutze die Lüfter- und Pumpensteuerung von freeRGB auf eigene Gefahr im
> oben beschriebenen Rahmen.
> [Lüftersteuerung aktivieren] (erst nach beiden Häkchen aktiv)

Note: version 2 says "the freeRGB developers are not liable for damage";
version 3 replaces this with the gratuitous standard. The `safety.ts` header
already lists the mandatory exceptions; the § 524 BGB element (fraudulent
concealment) is new. `[LAWYER]`

---

## (c) README warning

### EN

```markdown
> [!WARNING]
> **Beta, experimental, provided free of charge and without warranty.**
> freeRGB can lower fan and pump speeds. Wrong settings, untested hardware or
> software faults can cause overheating, data loss, hardware damage, fire and,
> in extreme cases, injury. Motherboard fan control has not been confirmed on
> real hardware for most boards; freeRGB cannot confirm actual fan speed.
> Safeguards and their limits per backend: [docs/COOLING_SAFETY.md](docs/COOLING_SAFETY.md).
> Use at your own risk. Liability is limited only as far as the law permits;
> liability for injury to life, body or health, intent, gross negligence and
> under mandatory product liability law is not affected.
> Online/community features, telemetry and download counting are off by default.
> This is a private, non-commercial project.
```

### DE

```markdown
> [!WARNING]
> **Beta, experimentell, unentgeltlich und ohne Gewähr.**
> freeRGB kann Lüfter- und Pumpendrehzahlen senken. Falsche Einstellungen,
> ungetestete Hardware oder Softwarefehler können Überhitzung, Datenverlust,
> Hardwareschäden, Brand und im Extremfall Verletzungen verursachen. Die
> Mainboard-Lüftersteuerung ist für die meisten Boards nicht auf echter Hardware
> bestätigt; freeRGB kann die tatsächliche Drehzahl nicht bestätigen.
> Schutzmaßnahmen und ihre Grenzen je Backend: [docs/COOLING_SAFETY.md](docs/COOLING_SAFETY.md).
> Nutzung auf eigene Gefahr. Die Haftung ist nur beschränkt, soweit das Gesetz
> es zulässt; die Haftung für Verletzung von Leben, Körper oder Gesundheit,
> Vorsatz, grobe Fahrlässigkeit und nach zwingendem Produkthaftungsrecht bleibt
> unberührt. Online-/Community-Funktionen, Telemetrie und Download-Zählung sind
> standardmäßig aus. Privates, nicht kommerzielles Projekt.
```

---

## (d) Beta label

| Place | EN | DE |
|---|---|---|
| Badge (title bar, About, installer title) | `Beta` | `Beta` |
| Tooltip on badge | Experimental beta. Features may be incomplete or faulty. Fan and pump control is not confirmed on most hardware. | Experimentelle Beta. Funktionen können unvollständig oder fehlerhaft sein. Die Lüfter- und Pumpensteuerung ist auf den meisten Geräten nicht bestätigt. |
| Cooling page banner | Beta: freeRGB requests fan speeds but cannot confirm what the hardware applies. Watch your temperatures. | Beta: freeRGB fordert Drehzahlen an, kann aber nicht bestätigen, was die Hardware umsetzt. Beobachten Sie die Temperaturen. |
| Release title | freeRGB {version} (beta, experimental) | freeRGB {version} (Beta, experimentell) |

---

## Open points (owner / lawyer)

1. `[LAWYER]` Applicability of §§ 521, 524 BGB to gratuitous open-source
   downloads; does the limit cover damage to the user's other property?
2. `[LAWYER]` Whether a liability clause in an installer click-through is an
   AGB (§ 305 BGB) and is validly incorporated; whether "to the extent
   permitted by law" wording is sufficiently transparent (§ 307 Abs. 1 S. 2 BGB).
3. `[LAWYER]` Product liability (ProdHaftG and Directive (EU) 2024/2853) and CRA
   applicability for a private, non-commercial publisher (D1, D5).
4. `[OWNER]` Insurance (private liability policy coverage for software) – D5.
5. `[OWNER]` Wording for non-German users / choice of law; foreign consumers may
   be protected by their own mandatory law (Rome I Art. 6). `[LAWYER]`
6. Engineering: integrate (a) into installer, (b) into `safety.ts` with
   `SAFETY_TEXT_VERSION` bump to 3 in TS and daemon, (c) into README, (d) into
   GUI; re-check every bracketed evidence ID against `docs/COOLING_SAFETY.md`
   at that time, since FR-012 (revocation drain) and later changes may add rows.
7. Hardware acceptance (D9) is still open; until then no text may state that
   any specific board or device has been confirmed.
