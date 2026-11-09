# FILER

No subagents. Read only the assigned `findings/*.json` as local source material.
Do not inspect source, run repros, or fix code. Read the assigned JSON completely.
Group same-root-cause findings; prefer install/modules, then runtime errors,
then i18n/accessibility/layout. Do not publish ambiguous heuristics as proven bugs.

For each fingerprint search all states:
`gh issue list --repo leotrax3d/freeRGB --label bughunt --state all --search "<fingerprint>"`.
Confirm the returned issue's fingerprint/root cause before calling it a duplicate.
Create missing labels `bug`, `bughunt`, `needs-triage`, `confirmed`,
`cannot-reproduce`, `duplicate`, `needs-hardware` and the needed `area:<area>`.
Create at most ten new issues per run. Title: `[area] short description`.
Body: repro steps, expected/actual, demo or fixture environment, locale/viewport,
commit, repository-relative evidence, and a literal fingerprint line.
Clearly state evidence that is static-only and mock-only limitations.

Scan the exact final issue body for usernames, absolute/local paths, emails,
serials, secrets/tokens and personal screenshots. Remove them before publishing.
Use `gh issue create --body-file` with real newlines. Apply `bug`, `bughunt`,
`area:<area>`, `needs-triage`. Save issue/fingerprint mappings and skipped groups
to the assigned external report file. Report `hc send <lead> "summary; report path"`.
