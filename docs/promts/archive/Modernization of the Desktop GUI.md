# freeRGB – Modernisierung der Desktop-GUI

## 1. Zielsetzung

Ich möchte die bestehende Benutzeroberfläche von **freeRGB** grundlegend modernisieren.

freeRGB ist ein in **Rust** entwickeltes Projekt, das aktuell **egui** als GUI-Framework verwendet. Ziel ist die Entwicklung einer modernen, hochwertigen und performanten Desktop-Oberfläche, die sich hinsichtlich Designqualität, Klarheit und Benutzerfreundlichkeit an modernen Anwendungen wie ChatGPT orientiert.

Die neue GUI soll sich wie ein ausgereiftes Softwareprodukt anfühlen und eine eigenständige visuelle Identität entwickeln.

Besonders wichtig sind:

- Minimalistisches, modernes und konsistentes Design
- Hochwertiger Dark Mode
- Klare visuelle Hierarchie und intuitive Navigation
- Angenehme Abstände und moderne Typografie
- Dezente Animationen und flüssige Interaktionen
- Übersichtliche Darstellung von Geräten, Farben und Effekten
- Hohe Performance und geringer Ressourcenverbrauch
- Langfristige Wartbarkeit und Erweiterbarkeit

**Wichtig:** Du hast ausdrücklich großen kreativen und technischen Freiraum. Die Oberfläche soll keine bestehende Anwendung exakt kopieren. Entwickle eigenständig ein durchdachtes Design, das zu freeRGB passt.

---

## 2. Technologiestack

Als bevorzugter Technologiestack ist folgende Kombination vorgesehen:

| Bereich            | Technologie                          |
| ------------------ | ------------------------------------ |
| Desktop-Framework  | Tauri 2                              |
| Frontend           | React                                |
| Programmiersprache | TypeScript                           |
| UI-Komponenten     | shadcn/ui                            |
| Styling            | Tailwind CSS                         |
| Icons              | Lucide React                         |
| Animationen        | CSS-Transitions, optional Motion     |
| Backend            | Bestehender Rust-Service             |
| Kommunikation      | Vorhandene Schnittstellen bevorzugen |

### Verwendung von shadcn/ui

Nutze **shadcn/ui als bevorzugte Grundlage für die UI-Komponenten**.

shadcn/ui soll eine konsistente, zugängliche und moderne Komponentenbasis bereitstellen. Verwende die Komponenten jedoch nicht zwangsläufig in ihrer Standardgestaltung.

Du darfst Komponenten:

- Individuell gestalten und erweitern
- An die Anforderungen von freeRGB anpassen
- Kombinieren und abstrahieren
- Durch eigene Komponenten ersetzen, wenn dies sinnvoll ist

Das Ziel ist ausdrücklich **keine generische shadcn-Dashboard-Vorlage**, sondern eine eigenständige, hochwertige Desktop-Anwendung.

Verwende Tailwind CSS und zentrale Design-Tokens beziehungsweise CSS-Variablen, um ein konsistentes und wartbares Designsystem aufzubauen.

Andere Technologien oder Anpassungen am vorgeschlagenen Stack sind möglich, wenn sie einen klaren technischen oder gestalterischen Vorteil bieten. Begründe solche Abweichungen.

---

## 3. Design und Benutzererlebnis

Orientiere dich an der Qualität moderner Desktop- und Webanwendungen, insbesondere an deren:

- Klarheit und visueller Ruhe
- Konsistenz
- Intuitiver Bedienbarkeit
- Hochwertiger Typografie
- Sinnvoller Informationshierarchie
- Reduzierter, funktionaler Gestaltung

**ChatGPT dient als Inspiration für die Designqualität, nicht als Vorlage für eine identische Benutzeroberfläche.**

Entwickle ein Design, das speziell auf die Anforderungen einer RGB-Steuerungssoftware zugeschnitten ist.

### Designspezifikation

Analysiere die bereits vorhandene Designspezifikation und verwende sie als Ausgangspunkt.

Die bisher festgelegte Farbpalette umfasst:

**Akzentfarben**

| Farbe       | Hex       |
| ----------- | --------- |
| Grün        | `#10A37F` |
| Rot         | `#EF4444` |
| Blau        | `#3B82F6` |
| Orange/Gelb | `#F59E0B` |
| Violett     | `#A855F7` |

**Dark-Mode-Farben**

| Verwendung         | Hex       |
| ------------------ | --------- |
| Hintergrund        | `#000000` |
| Oberfläche         | `#212121` |
| Erhöhte Oberfläche | `#303030` |
| Primärer Text      | `#ECECEC` |

Diese Farben dienen als Grundlage, dürfen aber sinnvoll ergänzt und in ihrer Verwendung angepasst werden.

Entwickle daraus ein zusammenhängendes Designsystem mit:

- Semantischen Farb-Tokens
- Einheitlichen Abständen
- Konsistenten Rundungen
- Typografischer Hierarchie
- Einheitlichen Komponenten-Zuständen
- Dezenten Hover- und Fokus-Effekten
- Verständlichen Lade-, Fehler- und Leerzuständen
- Barrierearmen Bedienelementen und Tastaturnavigation

### Gestalterischer Freiraum

Du darfst Navigation, Layout, Komponenten und Interaktionskonzepte eigenständig entwickeln.

Insbesondere die Darstellung von Geräten, RGB-Zonen, Farben, Effekten und Profilen soll sich intuitiv und hochwertig anfühlen.

Vermeide unnötige visuelle Komplexität, übermäßige Animationen und typische generische Admin-Dashboard-Layouts.

Die Anwendung soll modern, funktional und angenehm zu bedienen sein.

### Grundlegende Anforderungen

- Der Background-Service läuft unabhängig von der GUI.
- Die GUI wird nur gestartet, wenn der Benutzer sie öffnet.
- Beim Schließen wird die GUI vollständig beendet.
- Das Schließen der GUI darf den Background-Service nicht beenden.
- Zeitkritische RGB-Steuerung verbleibt vollständig im Rust-Service.
- Die GUI verwendet bevorzugt die bereits vorhandenen Kommunikationsschnittstellen.
- Bestehende Funktionen und Protokolle sollen möglichst unverändert bleiben.

Analysiere die vorhandene Architektur, bevor du Änderungen vornimmst.

Falls Anpassungen an den Schnittstellen notwendig sind, halte sie minimal, nachvollziehbar und möglichst abwärtskompatibel.

Die neue GUI darf keine zweite, konkurrierende Geräteverwaltung oder RGB-Steuerungslogik implementieren.

---

## 5. Bestehende egui-Oberfläche

Die bisherige egui-Oberfläche soll zunächst als **Legacy-Version** erhalten bleiben.

Sie dient als funktionierende Referenz und als Rückfalloption während der Entwicklung. außerdem soll sie für resourcenahme pcs verwendet werden.

Die neue GUI soll schrittweise und möglichst unabhängig davon entstehen.

Achte darauf, dass:

- Die Legacy-GUI weiterhin funktioniert.
- Bestehende Funktionen nicht verloren gehen.
- Beide Oberflächen während der Übergangsphase getrennt entwickelt und getestet werden können.
- Bestehende Konfigurationen und gespeicherte Einstellungen möglichst kompatibel bleiben.



---

## 6. Performance und Ressourceneffizienz

Die neue GUI soll trotz moderner Webtechnologien möglichst effizient sein.

Wichtige Anforderungen:

- Schneller Anwendungsstart
- Geringer RAM-Verbrauch
- Niedrige CPU-Auslastung im Leerlauf
- Flüssige Benutzerinteraktionen
- Effiziente Aktualisierung von Gerätezuständen
- Keine unnötigen Hintergrundprozesse
- Keine dauerhaften Render-Schleifen ohne funktionalen Bedarf
- Keine unnötigen Netzwerkzugriffe

### Frontend-Optimierung

Verwende nur tatsächlich benötigte Komponenten und Abhängigkeiten.

Achte insbesondere auf:

- Effiziente React-Komponenten
- Vermeidung unnötiger Re-Renders
- Sinnvolle Aufteilung des Anwendungszustands
- Ereignisbasierte Kommunikation, sofern die vorhandenen Schnittstellen dies ermöglichen
- Sparsame Animationen
- Bedarfsgerechtes Laden komplexer Ansichten
- Vermeidung überdimensionierter Bibliotheken

Animationen sollen die Benutzererfahrung verbessern, ohne dauerhaft CPU- oder GPU-Ressourcen zu beanspruchen.

Die Anwendung soll auch bei vielen Geräten, RGB-Zonen und Effekten reaktionsschnell bleiben.

---

## 7. Architektur und Wartbarkeit

Entwickle eine verständliche und langfristig wartbare Frontend-Architektur.

Trenne nach Möglichkeit:

- Darstellung und UI-Komponenten
- Anwendungszustand
- Kommunikation mit dem Rust-Service
- Datenmodelle und Typdefinitionen
- Designsystem und Styling

Vermeide unnötige Abstraktionen, komplexe Architekturmuster und zusätzliche Abhängigkeiten ohne erkennbaren Mehrwert.

Die Architektur soll ausreichend flexibel sein, um später neue Geräte, Funktionen und Ansichten zu integrieren.

Bevorzuge einfache, nachvollziehbare Lösungen gegenüber unnötigem Overengineering.

---

## 8. Vorgehensweise

Arbeite strukturiert und orientiere dich am tatsächlichen Zustand des Projekts.

### Phase 1 – Bestandsaufnahme

Analysiere zunächst:

1. Die bestehende Projektstruktur
2. Die aktuelle egui-Oberfläche und ihre Funktionen
3. Den Rust-Background-Service
4. Die Kommunikation zwischen GUI und Service
5. Die vorhandene Designspezifikation
6. Bestehende Konfigurationen, Profile und Zustandsmodelle
7. Relevante Build-, Packaging- und Startmechanismen

Identifiziere vorhandene Funktionen, technische Einschränkungen und mögliche Integrationsprobleme.

Triff keine grundlegenden Architekturentscheidungen, bevor du die relevanten Projektbestandteile verstanden hast.

### Phase 2 – Konzeption

Entwickle auf Basis der Analyse ein Konzept für die neue GUI.

Das Konzept soll unter anderem umfassen:

- Empfohlene technische Architektur
- Integration von Tauri 2
- Verwendung von React, TypeScript und shadcn/ui
- Kommunikationskonzept mit dem Rust-Service
- Navigationsstruktur
- Grundlegendes Layout
- Wiederverwendbare UI-Komponenten
- Designsystem
- Migrationsstrategie
- Performance-Überlegungen

Du darfst eigenständig sinnvolle Entscheidungen treffen.

Begründe wichtige technische Abweichungen von den vorgeschlagenen Technologien.

### Phase 3 – Design und Prototyp

Entwickle zunächst eine visuell überzeugende Grundlage für die neue Benutzeroberfläche.

Konzentriere dich auf:

- Grundlayout und Navigation
- Geräteübersicht
- Gerätesteuerung
- RGB-Farbauswahl
- Effektverwaltung
- Profile und Einstellungen, soweit vom Projekt unterstützt
- Konsistente Komponenten und Zustände

Nutze vorhandene Funktionen als Grundlage und erfinde keine Backend-Funktionalität, die tatsächlich nicht existiert.

Einzelne Funktionen dürfen zunächst mit klar gekennzeichneten Mock-Daten dargestellt werden, wenn dies für die Designentwicklung sinnvoll ist.

Der Prototyp soll eine realistische Vorstellung des späteren Produkts vermitteln.

### Phase 4 – Schrittweise Implementierung

Implementiere die neue GUI schrittweise.

Bevorzuge kleine, nachvollziehbare Änderungen gegenüber einer vollständigen Umstrukturierung des Projekts.

Stelle sicher, dass:

- Die bestehende Anwendung weiterhin funktionsfähig bleibt.
- Die neue GUI unabhängig entwickelt werden kann.
- Bestehende Service-Schnittstellen wiederverwendet werden.
- Neue Komponenten sauber strukturiert sind.
- Fehlerfälle angemessen behandelt werden.
- Die Implementierung getestet werden kann.

Überprüfe regelmäßig, ob Design, Bedienbarkeit und Performance den ursprünglichen Zielen entsprechen.

---

## 9. Eigenständige Entscheidungen

Du bist ausdrücklich dazu aufgefordert, eigene technische und gestalterische Entscheidungen zu treffen.

Du musst nicht jede Kleinigkeit rückfragen.

Wähle selbstständig sinnvolle:

- Layouts
- Komponenten
- Navigationskonzepte
- Interaktionen
- Animationen
- Projektstrukturen
- Technische Implementierungsdetails

Nutze bewährte Designprinzipien und moderne Entwicklungspraktiken.

Bei Entscheidungen mit größeren Auswirkungen auf die bestehende Architektur, den Background-Service, die Kompatibilität oder die Stabilität sollst du die Auswirkungen zunächst erläutern.

**Kreativer Freiraum ist ausdrücklich erwünscht. Funktionalität, Performance und Wartbarkeit haben jedoch Vorrang vor rein dekorativen Designentscheidungen.**

---

## 10. Erwartetes Ergebnis

Das langfristige Ziel ist eine moderne, performante und langfristig wartbare Desktop-GUI für freeRGB.

Die Anwendung soll:

- Wie ein hochwertiges, modernes Softwareprodukt wirken
- Eine eigenständige visuelle Identität besitzen
- Einfach und intuitiv bedienbar sein
- Die bestehenden Funktionen vollständig unterstützen
- Den Rust-Background-Service zuverlässig nutzen
- Möglichst ressourcenschonend arbeiten
- Unabhängig von der Legacy-GUI funktionieren
- Problemlos erweitert und gewartet werden können

Die Kombination aus **Tauri 2, React, TypeScript, shadcn/ui und Tailwind CSS** ist die bevorzugte Grundlage, soll aber nicht zu unnötigen technischen Einschränkungen führen.

## 11. Erste Aufgabe

**Beginne mit der Analyse des bestehenden Projekts.**

Untersuche die Projektstruktur, die aktuelle egui-Implementierung, die Kommunikation mit dem Background-Service und die vorhandene Designspezifikation.

Erstelle anschließend einen konkreten Vorschlag für die neue GUI-Architektur und das Designkonzept.

Zeige dabei auf, welche vorhandenen Funktionen übernommen werden müssen, welche technischen Herausforderungen bestehen und wie die schrittweise Migration sinnvoll umgesetzt werden kann.

**Beginne nicht unmittelbar mit einer vollständigen Neuimplementierung.** Zuerst soll eine fundierte technische und gestalterische Grundlage entstehen.

Das Ziel ist nicht einfach nur, egui durch eine Weboberfläche zu ersetzen, sondern freeRGB eine deutlich modernere und bessere Benutzererfahrung zu geben, ohne die Stabilität und Effizienz des bestehenden Systems zu gefährden.
