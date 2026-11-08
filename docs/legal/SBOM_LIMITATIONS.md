# SBOM: Umfang und Grenzen

Die CycloneDX-Datei ist ein **unvollständiges Quellen-/Lockfileinventar**, keine vollständige Build-/Distributions-SBOM. Sie enthält 1502 nach Ökosystem/Name/Version/Quelle unterschiedene Komponenten über 14 Branches und 5 Tags. Mehrfachversionen, Dev- und Plattformalternativen sind enthalten. Diese Anzahl bezeichnet weder ausgelieferte Bibliotheken noch festgestellte Schwachstellen.

Rust-Lockfiles führen keine Lizenztexte; unbekannte Lizenzen bleiben NOASSERTION. npm-Lizenzen sind deklarierte Lockmetadaten, keine unabhängig geprüften Lizenztexte. Repository-Manifeste wurden gelesen. Die LHM-Lizenz wurde in der gepinnten Version gelesen und direkte NuGet-Referenzen zusätzlich erfasst; deren transitive Auflösung und weitere eingebettete/native/Systembibliotheken bleiben offen. Registrychecksummen stammen aus Lockfiles und sind kein Beleg hier heruntergeladener Fremdpakete.

Es wurden keine vollständigen Cargo-Zielplattformgraphen oder NuGet-assets-Dateien erzeugt, keine vollständigen Abhängigkeitsbeziehungen behauptet, keine Binärpakete extrahiert und kein vollständiger Lizenz-/Advisoryscan durchgeführt. Die Datei behauptet daher weder Lizenzkonformität noch Schwachstellenfreiheit. Für eine Release-SBOM sind beide Rust-Workspaces, alle npm-Apps, Laufzeitpayload, eingebettete Driver/Runtime-/Fontkomponenten und die tatsächlichen Zielplattformen aufzulösen. Unbenutzte Lockeinträge müssen herausgefiltert werden.

Parserprobleme: [].

Inventarisierte Lockdateien: 86 Referenz-/Pfadpaare. Gleiche Blobs wurden über ihre SHAs erkannt; scopes stehen in observed_in.

Die kurzen IDs L001 usw. in observed_in werden in LOCKFILE_SCOPES.json auf Git-Blob und sämtliche Referenz-/Pfadpaare aufgelöst. Sie bezeichnen Lockdateiquellen, keine ausgelieferten Teilprodukte. FILE_COVERAGE.csv enthält pro eindeutigem Blob eine exakte path_to_refs_json-Zuordnung und unterscheidet Abruf/Hashprüfung und mechanischen Scan.


## Update 2026-11-07 (scripts/gen-notices.py)

`scripts/gen-notices.py --sbom` fills the `licenses` field of cargo and npm components whose name and version appear in the resolved, shipped normal-dependency graph, and marks them with the property `freergb:in_shipped_normal_graph`. The licence string is the one declared in the package metadata (`cargo metadata --locked`, `package-lock.json`); it is not a lawyer-verified conclusion and may be an `OR`/`AND` expression that needs a human choice. The inventory itself is unchanged (still 1502 components).

What is now better: 581 components (all with the property) carry a declared licence instead of NOASSERTION; the licence texts found in the crate packages and npm packages are collected in `THIRD_PARTY_NOTICES.md`, which is regenerated from the locked graph for `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, for the root workspace and the Tauri desktop workspace, normal dependencies only (proc-macro crates included, dev and build dependencies excluded, `modules/*` excluded).

What is still open:

- Components outside that graph (other branches/tags, dev and build dependencies, platform alternatives) still read NOASSERTION where no lock metadata exists; 11 NuGet and 2 binary entries are unchanged.
- The SBOM is still not a payload SBOM: no extraction of the real installer, Debian package or AppImage, no resolved dependency edges, no Linux system libraries, no LHM/PawnIO runtime closure.
- Some Rust crates ship no licence file in their package (listed in `THIRD_PARTY_NOTICES.md`); only the declared expression is known for them.
- Declared licences come from upstream metadata and were not compared with source headers or upstream repositories.
- The graph depends on the toolchain and lockfile at generation time; regenerate with `python scripts/gen-notices.py --sbom` after any lockfile change.
