# Gemeinsame Sternebewertungen einrichten

Historische Alternative: Die aktuelle App verwendet [Appwrite](APPWRITE_SETUP.md).
Diese Supabase-Anleitung und die SQL-Migration werden nicht mehr von der App verwendet.

Die App-Nutzer brauchen **keine Registrierung und keinen Login**. Nur das
freeRGB-Team benötigt einmal ein Supabase-Konto, weil jemand die gemeinsame
Datenbank betreiben und verwalten muss. Es wurde in dieser Aufgabe kein Konto,
Projekt oder kostenpflichtiger Dienst angelegt. Ohne Projekt bleibt die
Bewertungsfunktion sichtbar, aber deaktiviert; es werden keine Bewertungen erfunden.

## Einmalig für dich als Betreiber

1. Öffne [Supabase](https://supabase.com/dashboard), melde dich an und lege ein
   Projekt für freeRGB an. Prüfe den gewünschten Tarif und die geltenden Limits;
   ein kostenpflichtiger Tarif ist durch diese Implementierung nicht beauftragt.
2. Öffne den SQL Editor deines Projekts und führe den Inhalt von
   [`supabase/migrations/202610230001_effect_ratings.sql`](../supabase/migrations/202610230001_effect_ratings.sql)
   aus. Dadurch entstehen die Effekttabelle, Bewertungen und Zugriffsregeln.
3. Aktiviere in Authentication die **Anonymous Sign-Ins**. Damit erhält die
   App beim ersten Bewerten eine anonyme Kennung. E-Mail, Passwort und GitHub
   werden dafür nicht gebraucht. Siehe
   [Supabases Anleitung](https://supabase.com/docs/guides/auth/auth-anonymous).
4. Kopiere aus den Projekteinstellungen die **Project URL** und den
   **Publishable Key** (`sb_publishable_…`). Der ältere öffentliche `anon`-Key
   wird ebenfalls unterstützt. **Nicht** den Secret Key oder `service_role`-Key
   verwenden. Solche privilegierten Schlüssel gehören niemals in die App.
5. Trage im offiziellen GitHub-Repository unter Settings → Secrets and variables
   → Actions → **Variables** diese beiden öffentlichen Werte ein:
   - `FREERGB_SUPABASE_URL`
   - `FREERGB_SUPABASE_PUBLISHABLE_KEY`
   Die CI- und Package-Workflows übernehmen sie beim Rust-Build. Ein neu gebautes
   Installationspaket enthält damit die öffentliche Verbindungskonfiguration;
   App-Nutzer müssen nichts selbst einrichten.

Für einen lokalen Entwicklungslauf können dieselben Variablennamen vor dem Start
des Daemons gesetzt werden. Falls die Cloud-Umgebung ausgehende Verbindungen
beschränkt, muss die **konkrete Projekt-Domain** aus der Project URL freigegeben
werden. Es wurde keine unbekannte Domain oder pauschale Supabase-Freigabe hinzugefügt.

Die Konfiguration ist erst nach Einrichtung und erfolgreichem Live-Test
betriebsbereit. Die komplette lokale SQL-/HTTP-Integration wurde geprüft; ein
Live-Test mit einem echten Supabase-Projekt ist ohne dessen URL und öffentlichen
Schlüssel noch nicht möglich. Wenn das Projekt CAPTCHA für anonyme Anmeldungen
verlangt, braucht die App zusätzlich einen unterstützten CAPTCHA-Flow; aktuell
wird dieser Fehler angezeigt und die Prüfung nicht umgangen.

## Was Nutzer sehen

Im Marketplace stehen Publisher, gemeinsamer Durchschnitt und Anzahl der
Bewertungen. Ein Klick auf einen Stern speichert 1–5 Sterne. Ein weiterer Klick
auf eine andere Sternzahl aktualisiert dieselbe Bewertung. „Remove“ zieht die
eigene Bewertung zurück. Bei Serverfehlern wird kein erfolgreicher Speichervorgang
vorgetäuscht. Bereits geladene Durchschnitte werden bei Ausfällen als möglicherweise
veraltet gekennzeichnet.

Erst beim ersten Bewerten wird eine anonyme Supabase-Sitzung erstellt. Der
Rust-Dienst speichert sie im privaten Benutzerdatenverzeichnis und erneuert sie
bei Ablauf. Damit wird beim nächsten App-Start dieselbe Bewertung wiedergefunden.
Die Sitzungsdatei hat unter Unix Berechtigungen `0600`; unter Windows gilt der
Zugriffsschutz des Benutzerprofils. Die Datei ist pro Projekt getrennt, wird weder
an die GUI herausgegeben noch in Fehlerberichte aufgenommen und darf nicht in
Git eingecheckt werden.

Es zählt eine Bewertung je **anonymer Kennung und Effekt**, nicht eine garantierte
Bewertung pro Mensch. Wer App-Daten löscht oder mehrere Installationen nutzt,
kann eine neue Kennung erhalten. Ohne Benutzerkonten ist das kein vollständiger
Manipulationsschutz. Supabase-Limits und Monitoring müssen zum erwarteten Betrieb
passen; CAPTCHA oder weitere Missbrauchsmaßnahmen können später ergänzt werden.
Es werden weder Hardware-Fingerprints noch Geräteinfos für Bewertungen verwendet.

## Datenbank und Sicherheit

- `effect_catalog` enthält die offiziell bewertbaren Effekt-IDs. Neue IDs müssen
  beim Veröffentlichen eines neuen Effekts vom Team ergänzt werden.
- `effect_ratings` hat einen zusammengesetzten Primärschlüssel aus `user_id` und
  `effect_id`; `stars` ist auf ganze Zahlen von 1 bis 5 begrenzt.
- Row Level Security erlaubt angemeldeten anonymen Sitzungen nur Lesen, Ändern
  und Löschen ihrer **eigenen** Zeilen (`auth.uid() = user_id`).
- Die öffentliche Funktion `effect_rating_summary()` liefert ausschließlich
  Effekt-ID, Durchschnitt und Anzahl. Fremde Kennungen oder Einzelbewertungen
  sind öffentlich nicht lesbar.
- Der Client akzeptiert nur HTTPS-Ursprünge ohne URL-Zugangsdaten. Weiterleitungen
  sind deaktiviert; Tokens werden nur an die konfigurierte Projekt-Adresse gesendet.
- Zugangsdaten einer anonymen Sitzung bleiben im Dienst und in der privaten Datei.
  Der öffentliche Publishable Key ist keine Admin-Berechtigung; die RLS-Regeln
  sind die eigentliche Zugriffskontrolle.

Rust-HTTP-Tests prüfen anonyme Anmeldung, Wiederverwendung nach Neustart,
Token-Erneuerung, Upsert, Durchschnitt, Rücknahme, abgewiesene Schreibvorgänge
und die Ablehnung von Secret-/Service-Role-Keys. Der separate PostgreSQL-Test
prüft die tatsächlich ausgeführte Migration und RLS-Regeln:

```bash
bash scripts/test-ratings-rls.sh
```

Er startet einen temporären PostgreSQL-Container ohne Netzwerkzugriff und entfernt
ihn danach. Die SQL-Dateien unter `supabase/tests/` sind ausschließlich für diese
wegwerfbare Testdatenbank bestimmt; sie gehören **nicht** in den Supabase SQL Editor.

## Kontakt und Fehlerberichte

Unter Contact und im Marketplace sind `freergbteam@gmail.com` und der
[offizielle YouTube-Kanal](https://www.youtube.com/channel/UCh-B-9nUgKeCNJm3ESrpR8w)
hinterlegt. E-Mail öffnet das E-Mail-Programm; es wird keine Nachricht automatisch
verschickt.

Bei einem aktuellen Effekt-/Installationsfehler zeigt Marketplace einen stabilen
Fehlercode und „Review error report“. Der Dienst erstellt zunächst nur eine
Vorschau mit Fehlercode, Effekt-/App-Version, Betriebssystem und reduzierten
Geräteinfos (Hersteller, Gerätetyp, USB-IDs, Status). Namen, Seriennummern, Pfade,
Sensorwerte, rohe Fehlertexte und Bewertungszugänge sind ausgeschlossen.

Der Bericht bleibt vor „Open GitHub issue“ vollständig lokal und bearbeitbar.
Danach öffnet die App ausschließlich das offizielle GitHub-Issue-Formular mit
**genau dem geprüften Text**. GitHub-Issues sind öffentlich und erfordern zum
Abschicken ein GitHub-Konto. Dieser Schritt betrifft nur Fehlerberichte, nicht
Sternebewertungen. freeRGB schickt keine Issues im Hintergrund ab und behauptet
nach dem Öffnen des Formulars nicht, der Bericht sei bereits gesendet worden.
Die Funktion steht Nutzern und damit auch dir bzw. Administratoren zur Verfügung;
sie führt keine neue Admin-Rolle oder zusätzliche Hardware-Rechte ein.
