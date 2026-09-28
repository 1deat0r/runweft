# 01-contracts

Status: Proposed
Spec revision: S1
Type: design/spike. Phase: 0. Blocked by: board gate.
What/why: define one command/event contract and prove the Rust/TypeScript split is practical.
Method: generated schemas; replay fixtures; one provider/tool round trip; record contributor workflow and IPC overhead.
Acceptance: 100% of 50 versioned protocol fixtures agree across Rust and TypeScript, including unknown critical variants and malformed payloads.
Rule: DIES if any unauthorized critical variant executes; SURVIVES if 50/50 fixtures pass; otherwise WEAKENS.
Falsifier/null: split adds complexity without independent authority. Risk: overgeneralized schemas. Owner role: runtime lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
