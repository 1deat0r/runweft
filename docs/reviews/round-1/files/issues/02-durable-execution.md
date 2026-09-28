# 02-durable-execution

Status: Proposed
Spec revision: S1
Type: implementation/verification. Phase: 1. Blocked by: 01.
What/why: ledger, outbox, attempts, leases and effect reconciliation.
Method: EP1's 10 crash boundaries × 1,000 seeds, including corrupt state, disk full, orphan artifacts, backup/GC leases, restart and cancellation.
Acceptance: 10,000 reproducible injected crashes; acknowledged state retained; opaque ambiguous effects enter unknown_effect; expired grants cannot dispatch.
Rule: DIES if any acknowledged transition is lost or an opaque effect is blindly replayed; SURVIVES if all 10,000 cases satisfy invariants; otherwise WEAKENS.
Falsifier/null: a database migration alone does not make external effects safe. Risk: power-loss assumptions and filesystem behavior. Owner: runtime lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
