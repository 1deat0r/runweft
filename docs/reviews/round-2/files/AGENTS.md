# Runweft contributor instructions

Read `README.md`, `STATE.md`, `spec.md`, then the relevant ticket under `issues/`.
The inherited `/mnt/data/AGENTS.md` disk contract applies. Canonical path:
`/mnt/data/AI Agents/Runweft`; the user-facing `/run/media/its1deat0r/Projects` path is the same verified SSD.

This repository is a scaffold. Do not claim the scheduler, database, broker,
sandbox, provider execution or live inspector exists. The default CLI and daemon
must not gain execution as a side effect of scaffold maintenance.

`spec.md` and its named normative documents govern future work. Historical research
and earlier board approvals under `docs/research/` do not approve new changes.
Follow user-authorized scope. A board approval is not deployment/publishing permission.

Use schema-first contracts. `schemas/runtime-status.schema.json` generates the two
scaffold status bindings via `npm run generate`; do not hand-edit generated files.
This narrow generator is not the future production schema compiler.
Keep Rust free of unsafe code unless an explicitly reviewed design changes that rule.
Adapters cannot become authority for durable state or permissions.

Run `npm run check` for scaffold changes after `npm ci --ignore-scripts`.
Do not call live providers, read credentials, or run arbitrary repository tools in
verification. Future integration work must use isolated fixtures first. Report
which checks actually ran; green scaffold tests do not establish runtime security.

Board reviews use frozen snapshots and private per-seat prompts. Never modify
reviewed files during a round or share peer verdicts before reviewers vote.
Record numbered blockers and quote-verified closures. Use additional agents only
when the user authorizes them or the applicable review skill requires them.
