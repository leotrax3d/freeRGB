# Roadmap

| # | Phase | Status |
|---|---|---|
| 1 | Werkzeuge/Umgebung feststellen | ✅ |
| 2 | SignalRGB untersuchen | ✅ (Grundinventar) |
| 3 | Reproduzierbares RE-Verfahren | ✅ `REVERSE_ENGINEERING.md` |
| 4 | Plattformunabhängiger Rust-Kern | ✅ `signal-core` |
| 5 | Virtuelles Gerät + Tests | ✅ |
| 6 | CORSAIR-Erkennung | ✅ K70 CORE, LINK Hub, LCD, RAM identifiziert |
| 7 | Erster realer RGB-Datenpfad | ✅ K70 CORE validiert |
| 8 | Hintergrunddienst + HAL | ⏭ nächster Schritt |
| 9 | GUI (egui), API, Plugin-System | offen |
| 10 | Weitere Hersteller (iCUE LINK, ASUS AURA, Razer) | offen |
| 11 | Effekte, Layout-Editor, Profile, Ambience, Audio, Effekt-Editor | offen |
| 12 | Windows/Linux-Kompatibilität (CI) | offen |

Begründete Abweichung: Discovery und Registry wurden vor dem Daemon gebaut, da sie
für die sichere Erstvalidierung (nur bekannte Interfaces beschreiben) nötig waren.
