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
- Main is protected. Human-authored PRs need a linked GitHub issue and passing
  `check (24)`, `check (26)`, and `agent-review-record` checks. Dependabot updates
  may omit an issue reference, but must pass all checks and agent reviews. Every PR
  must go through GitHub, use the current base, resolve review conversations, and
  squash merge to keep history linear. The branch rule requires no GitHub-human
  approvals; autonomous agent reviews are the project review gate. Direct pushes,
  local-only completion, and bypassing the project process are prohibited.
- Each PR requires distinct, independent agent reviewers for implementation /
  developer experience and evaluation / confounds. Select additional seats based on
  change risk from systems / durability, adversarial security, and product / scope.
  Changes to normative specifications or architecture, this review protocol, CI
  workflows, or branch-protection policy require all five seats on one frozen commit.
  The implementer cannot review its own work. Record each agent identifier, seat,
  verdict, reviewed commit SHA, evaluation evidence, numbered findings, and
  quote-verified blocker closures in the PR's structured review receipt. List all
  five seats there and state why any omitted seat is not relevant. Any code change
  after review invalidates the receipt and requires a fresh review of the new head.
  Never share peer verdicts before reviewers vote.
- `agent-review-record` checks receipt shape, seat coverage, and SHA consistency; it
  does not prove that an agent actually ran. Reviewers must be separate agent
  invocations, and the project maintainer must not treat a self-authored receipt as
  evidence of an independent review. GitHub has no trusted agent-attestation
  integration configured, and candidate PRs can edit the workflow and validator
  that produce this check. Those controls require full five-seat review. Project
  policy changes require an issue, PR, review, and audit record too; GitHub
  administrators can technically change repository rules.
  The project's no-bypass rule is a process requirement, not a claim that the
  administrator is technically unable to change settings.
- Normative spec or architecture decisions use the complete frozen-snapshot board
  process in `spec.md` §12. Reviewer findings cite exact text, impact, and closure
  conditions; verify every blocker against the changed artifact before merging.
- Run `npm ci --ignore-scripts` followed by `npm run check` for scaffold changes.
  Report the exact commands and outcomes. Never use live providers, read
  credentials, or treat passing scaffold checks as evidence of runtime security.
- Public issues and pull requests must contain no credentials, private user data,
  or vulnerability details. Follow `SECURITY.md` for private reports.
