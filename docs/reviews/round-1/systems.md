# Systems / durability — S1

Reviewer: `/root/spec_systems`, GPT-6 Astra, high effort. Independent frozen read.
Verdict: **CONDITIONAL**.

Material blocker SYS-1: backup restore needs an explicit authority and effect recovery
contract. S1 spec lines 165–169 require hash-verified restoration but do not define
restoration of an older snapshot after subsequent dispatches, acknowledgements or
revocations. It can resurrect a pending opaque effect, lose deduplication records,
or restore revoked authority. Ordinary DUR-02–05 and SEC-05 do not resolve missing
post-backup facts. Ticket 02 does not explicitly gate this case.

Closure requested: quarantine restoration under a fresh authority generation;
invalidate worker/broker credentials; do not automatically reactivate grants/effects;
define reconciliation, reauthorization and the data-loss/deduplication guarantees.
Require a backup → opaque effect/revocation → older restore → client retry/stale worker
fixture. No runtime test is required to run during this scaffold task.

Nonblocking notes: explicitly require exclusive coordinator ownership/fencing in
ticket 02; name transition-table/error-union deliverables in ticket 01.

Source inspection supports disabled CLI/daemon behavior. No build, crash, containment
or runtime correctness tests were executed by this reviewer. Those remain unverified.

This file records the review finding and closure condition; the conversation retains
the full original response.
