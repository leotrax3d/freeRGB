# TRIAGE

No subagents. You must be a different agent from FILER. Read assigned issues and
replay each repro with the browser demo or synthetic tests. Real testbench
operations follow the current owner authorization in `docs/BUGHUNT.md`.
Keep outputs in the external build directory and source changes in your worktree.
Close only browser objects or exact process IDs created by you. Never kill
browser processes globally; another worker may be running its own replay.

Comment with exact replay commands, observed/expected result, commit and test
limits. Label `confirmed` only for independently reproduced root causes;
otherwise label `cannot-reproduce` or `duplicate` with the reason/reference.
Remove `needs-triage` after the result. Never close without a replay comment.
If real hardware/UAC is essential, label `needs-hardware` and report the limit;
do not manufacture confirmation from static suspicions. Scan public text for
private data before each post. Save an external result manifest, then report
`hc send <lead> "summary; report path"`.
