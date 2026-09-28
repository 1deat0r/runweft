# 08-remote-workers

Status: Proposed
Spec revision: S2
Type: optional extension. Phase: 4. Blocked by: successful local release and renewed board review.
What/why: scale or isolate execution without changing run semantics.
Method: 1,000 network-partition/fencing scenarios; effect-boundary credential revocation; per-tenant canary files and egress checks.
Acceptance: 0 duplicate unsafe effects, stale-owner dispatches or cross-tenant accesses in the complete suite; reconnect preserves accepted state.
Rule: DIES if any of those violations occurs; SURVIVES if all 1,000 scenarios pass plus security review; otherwise WEAKENS.
Falsifier/null: remote workers add operational cost without useful workload demand. Risk: distributed effects and tenancy. Owner: platform lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
