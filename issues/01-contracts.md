# 01-contracts

Status: Ready
Spec revision: S2
Type: design/spike. Phase: 0. Blocked by: none (S2 is approved; owner authorization is recorded in GitHub issue #4).
What/why: define one command/event contract and prove the Rust/TypeScript split is practical.
Method: generated schemas; replay fixtures; one stubbed provider/tool round trip with no live credentials; record contributor workflow and IPC overhead. Freeze the transition table, error union, profile incarnation and coordinator generation fields before execution work.
Acceptance: the lifecycle table and error union are complete, and 100% of 50 versioned protocol fixtures agree across Rust and TypeScript, including unknown critical variants, stale incarnations and malformed payloads.
Rule: DIES if any unauthorized critical variant executes; SURVIVES if the lifecycle/error/incarnation contract is frozen and 50/50 fixtures pass; otherwise WEAKENS.
Falsifier/null: split adds complexity without independent authority. Risk: overgeneralized schemas. Owner role: runtime lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
