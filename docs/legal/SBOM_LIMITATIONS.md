# SBOM: Umfang und Grenzen

Die CycloneDX-Datei ist ein **unvollständiges Quellen-/Lockfileinventar**, keine vollständige Build-/Distributions-SBOM. Sie enthält 1502 nach Ökosystem/Name/Version/Quelle unterschiedene Komponenten über 14 Branches und 5 Tags. Mehrfachversionen, Dev- und Plattformalternativen sind enthalten. Diese Anzahl bezeichnet weder ausgelieferte Bibliotheken noch festgestellte Schwachstellen.

Rust-Lockfiles führen keine Lizenztexte; unbekannte Lizenzen bleiben NOASSERTION. npm-Lizenzen sind deklarierte Lockmetadaten, keine unabhängig geprüften Lizenztexte. Repository-Manifeste wurden gelesen. Die LHM-Lizenz wurde in der gepinnten Version gelesen und direkte NuGet-Referenzen zusätzlich erfasst; deren transitive Auflösung und weitere eingebettete/native/Systembibliotheken bleiben offen. Registrychecksummen stammen aus Lockfiles und sind kein Beleg hier heruntergeladener Fremdpakete.

Es wurden keine vollständigen Cargo-Zielplattformgraphen oder NuGet-assets-Dateien erzeugt, keine vollständigen Abhängigkeitsbeziehungen behauptet, keine Binärpakete extrahiert und kein vollständiger Lizenz-/Advisoryscan durchgeführt. Die Datei behauptet daher weder Lizenzkonformität noch Schwachstellenfreiheit. Für eine Release-SBOM sind beide Rust-Workspaces, alle npm-Apps, Laufzeitpayload, eingebettete Driver/Runtime-/Fontkomponenten und die tatsächlichen Zielplattformen aufzulösen. Unbenutzte Lockeinträge müssen herausgefiltert werden.

Parserprobleme: [].

Inventarisierte Lockdateien: 86 Referenz-/Pfadpaare. Gleiche Blobs wurden über ihre SHAs erkannt; scopes stehen in observed_in.

Die kurzen IDs L001 usw. in observed_in werden in LOCKFILE_SCOPES.json auf Git-Blob und sämtliche Referenz-/Pfadpaare aufgelöst. Sie bezeichnen Lockdateiquellen, keine ausgelieferten Teilprodukte. FILE_COVERAGE.csv enthält pro eindeutigem Blob eine exakte path_to_refs_json-Zuordnung und unterscheidet Abruf/Hashprüfung und mechanischen Scan.
