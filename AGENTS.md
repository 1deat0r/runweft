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

## GitHub development policy (adopted 2026-09-28)

The public GitHub repository `1deat0r/runweft` is the canonical source of truth.
This project folder is a working checkout of that repository. All changes intended
for Runweft must be recorded on GitHub through an issue and pull request; local
editing is allowed, but local-only completion and direct commits or pushes to
`main` are not. The initial public bootstrap commits establish this workflow;
the PR-only rule applies to every subsequent change.

- Start from the matching GitHub issue or existing numbered ticket, create a
  focused branch, and link the issue in the pull request. An issue, a board vote,
  or CI success does not grant implementation authority. The user must authorize
  the scope; proposed runtime tickets stay proposed until then.
- Main is protected. A human-authored pull request needs a linked GitHub issue and
  passing `check (24)` and `check (26)`. Dependabot version-update PRs are exempt
  from the issue-reference step, but must pass the same checks and review gate.
  Every PR also needs one approval from a GitHub reviewer other than the latest
  pusher, resolved review conversations, and a current base branch. Stale approvals
  are dismissed. Admins follow the same rules; do not bypass them. Merge with squash
  to keep history linear. If no independent reviewer is available, leave the PR open.
- The author/implementer does not review their own change. Select independent
  review roles based on the change: systems/durability, adversarial security,
  implementation/developer experience, evaluation/confounds, and product/scope.
  Record which roles reviewed and why any seat was not relevant. Normative spec
  or architecture decisions use the frozen-snapshot process in `spec.md` §12.
  Reviewer findings cite exact text, impact, and closure conditions; verify each
  blocker against the changed artifact before merging.
- Run `npm ci --ignore-scripts` followed by `npm run check` for scaffold changes.
  Report the exact commands and outcomes. Never use live providers, read
  credentials, or treat passing scaffold checks as evidence of runtime security.
- Public issues and pull requests must contain no credentials, private user data,
  or vulnerability details. Follow `SECURITY.md` for private reports.
