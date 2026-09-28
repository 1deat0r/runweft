# 02-durable-execution

Status: Proposed
Spec revision: S2
Type: implementation/verification. Phase: 1. Blocked by: 01.
What/why: ledger, outbox, attempts, leases and effect reconciliation.
Method: EP1's 10 crash boundaries × 1,000 seeds, including corrupt state, disk full, orphan artifacts, backup/GC leases, restart and cancellation. Also freeze explicit DUR-07/08 fixtures: take a backup, complete an opaque effect and revoke a grant, restore the older backup, retry the old client command and submit a stale-worker result; test two coordinators competing for ownership. These additional restore/ownership fixtures do not replace any EP1 case.
Acceptance: 10,000 reproducible injected crashes plus every added restore/ownership fixture pass; ordinary crash recovery retains acknowledged state; opaque ambiguous effects enter unknown_effect; expired grants cannot dispatch. Restore stays quarantined in a fresh incarnation, rejects old commands/credentials, never reactivates restored grants/intents and requires the cutoff/loss report, reconciliation and explicit authorization specified in DUR-07. Only the exclusive current coordinator can dispatch.
Rule: DIES if any ordinary-recovery acknowledged transition is lost, an opaque effect is blindly replayed, or rollback activates stale authority or conceals acknowledgement loss; SURVIVES if all stated acceptance criteria pass; otherwise WEAKENS. Explicitly confirmed backup rollback is evaluated under DUR-07, not misreported as zero-loss recovery.
Falsifier/null: a database migration alone does not make external effects safe. Risk: power-loss assumptions and filesystem behavior. Owner: runtime lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
