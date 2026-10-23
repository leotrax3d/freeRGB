# Proposed bot protection

These are proposed limits, not enabled firewall rules. Existing defenses are
verified anonymous JWTs, private tables, deterministic vote/download IDs, bounded
payloads and no desktop API secrets. They do not stop bots creating many identities.

1. Add an Appwrite Firewall rule limiting community Function execution by IP,
   initially 60 requests/minute with small bursts. Cover both Function-domain
   traffic and `/functions/community-api/executions` through the project API;
   do not assume that a Function-domain rule alone covers the execution API.
   Preview matching traffic before enabling. Shared networks need enough margin.
2. Limit anonymous-account creation separately, initially 10/hour/IP. Standard
   Appwrite auth limits already exist, but verify the desired custom policy and
   monitor false positives. Avoid blanket challenges on native JSON API routes.
3. Add persistent, atomic application quotas after JWT verification: start with
   10 vote writes/minute and 5 feedback/hardware reports/hour/identity. Use a
   separate private quota store with expiry. Function process memory is not a
   reliable distributed limiter. A WAF `userId` rule alone is insufficient for
   the current custom JWT execution header: enforce quotas on the verified user.
4. Cache public summaries for 30–60 seconds, limit requested effects to the
   published catalog, and eventually materialize aggregates instead of scanning
   every vote on each anonymous request. This reduces scraping and cost pressure.
5. Add a server-verified, single-use bot challenge for suspicious identity creation
   or submissions. For a desktop app, use a controlled HTTPS browser/WebView flow;
   do not ship the verification secret. Avoid requiring challenges for every vote.
6. Alert on execution spikes, anonymous-account growth and cost/usage thresholds.
   Keep payloads/tokens out of operational logs. Add report retention and a way to
   block abusive identities without granting public read access to submissions.

Download deduplication currently trusts the client's successful-install receipt.
This prevents accidental duplicates, not fabricated downloads. Stronger integrity
requires server-issued, expiring tickets tied to an authorized artifact/version,
idempotent redemption and delivery evidence. Anonymous identities still cannot
guarantee one human per vote without stronger identity/attestation tradeoffs.

References:
- https://appwrite.io/docs/products/firewall
- https://appwrite.io/docs/products/firewall/actions
- https://appwrite.io/docs/advanced/security/rate-limits
- https://developers.cloudflare.com/turnstile/get-started/server-side-validation/
