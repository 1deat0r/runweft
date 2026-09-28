# 05-context-and-edits

Status: Proposed
Spec revision: S2
Type: product/runtime. Phase: 2. Blocked by: 02–04.
What/why: retrieve attributable context and integrate code without losing user changes.
Method: stale-memory and restricted-summary disclosure fixtures; delete/reindex tests; dirty-tree imports; conflict and changed-base scenarios; independent checks after integration. Verify SEC-04a label propagation across code, arguments, artifacts, logs, encoded outputs and reused context/cache state; adversarial producers omit dependencies or submit false labels, and only explicit user declassification broadens recipients.
Acceptance: 0 lost user changes in 100 merge/conflict fixtures; 100% of sourced memory entries carry scope and provenance; no accepted receipt for a different artifact digest.
Rule: DIES on lost changes, cross-scope disclosure or stale-artifact acceptance; SURVIVES if every acceptance check passes; otherwise WEAKENS.
Falsifier/null: more memory or more agents need not improve outcomes. Risk: invalidation complexity. Owner: context/workspace lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
