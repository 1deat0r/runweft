# Runweft contributor instructions

Read `README.md`, `STATE.md`, `spec.md`, then the relevant ticket under `issues/` when one applies. The inherited `/mnt/data/AGENTS.md` disk contract applies. Canonical path: `/mnt/data/AI Agents/Runweft`; the user-facing `/run/media/its1deat0r/Projects` path is the same verified SSD.

This repository is a scaffold. Do not claim the scheduler, database, broker, sandbox, provider execution or live inspector exists. The default CLI and daemon must not gain execution as a side effect of scaffold maintenance.

`spec.md` and its named normative documents govern future product work. Historical research and earlier board approvals under `docs/research/` do not approve new changes. Follow the task's user-authorized scope. A board approval is not deployment or publishing permission.

Use schema-first contracts. `schemas/runtime-status.schema.json` generates the two scaffold status bindings; `schemas/run-protocol.schema.json` generates the v1 Rust/TypeScript protocol bindings, metadata, transition allow-list, and transition docs via `npm run generate`. Do not hand-edit generated files. These are wire contracts and validators, not a production schema compiler or runtime coordinator. Keep Rust free of unsafe code unless a reviewed design changes that rule. Adapters cannot become authority for durable state or permissions.

## Local-first development

Runweft is developed primarily by autonomous coding agents in the local Git checkout. The normal loop is: understand the task and repository, implement, run `npm run check`, inspect the complete diff, and make a small atomic commit. Do not create an Issue, branch, pull request, or wait for remote CI for routine work. Local commits are durable progress; never rewrite or destroy unrelated work.

`npm run check` is the canonical `VERIFY` command. Run it before committing unless a documented constraint prevents a relevant check; record exact commands, outcomes, and omissions. Run `npm ci --ignore-scripts` only after a fresh checkout or when dependencies or the lockfile require it. Keep generated bindings in sync through their generator. Before committing, inspect staged and unstaged changes for accidental files, secrets, debug output, and generated artifacts.

GitHub Issues are optional. Use one when it materially helps track deferred or multi-session work, dependencies, significant architectural work, external reports, or coordination. Pull requests are optional. Use one when independent remote review, concurrent work, clean-environment feedback, an external contribution, a major or risky change, or repository settings make it useful. Work on the normal development branch when that is safe and permitted; use branches or worktrees for parallelism, isolation, or a valuable PR. Never bypass an enforced GitHub protection or claim a local commit was synced when it was not.

GitHub Actions runs clean-environment checks on every push and pull request. The Node 24/26 matrix adds compatibility and reproducibility evidence, but it is not the local inner loop or a commit/push gate. Run `npm run check` before committing and address remote failures promptly; continue local work while remote checks run. GitHub cannot enforce this local procedure: a writer can push an unchecked commit directly to `main`, and remote jobs report results after the push. If remote CI finds a failure after a push, repair it with a follow-up commit or revert. Main retains admin enforcement, linear history, force-push and deletion protection, and conversation resolution for PRs. Keep private vulnerability reporting.

## Agent review and evaluation

Self-review the diff and evidence on every change. Add independent agent reviews when consequence or uncertainty warrants them; choose reviewers for the actual risk, such as implementation, evaluation, systems/durability, adversarial security, or product/scope. Changes to security controls, CI workflows, the review-receipt validator, branch-protection settings, or trust-boundary policy require an independent adversarial security review. Do not make a fixed seat count or receipt a prerequisite for routine commits. For high-impact changes, freeze the reviewed revision, keep first-pass reviewer reports independent, record numbered findings and evidence, and verify each blocker closure against the final artifact. Preserve a concise review/evidence record for security-control changes in a PR, commit description, or scoped audit note. A structured PR review receipt is optional and is checked for completeness when supplied; it cannot attest that an agent actually ran.

For changes to agent behavior or evaluations, define success and failure cases before implementation. Prefer deterministic assertions and fixed fixtures. Retain representative traces when tool choice, handoffs, or guardrails are under test. An LLM judge alone is not acceptance evidence. Do not call live providers, read credentials, or use external services during scaffold verification. Future integration work must use isolated fixtures first. A green scaffold check does not establish runtime security, durability, usability, or performance.

Ask the owner for authorization before production deployment, publication/release, production credential access, destructive actions, or scope beyond the task. Public issues and pull requests must contain no credentials, private user data, or vulnerability details. Follow `SECURITY.md` for private reports.

## Agent skills

These references configure the installed Matt Pocock engineering skills for this repository. Their default publishing or GitHub workflows yield to the local-first rules above.

### Issue tracker

GitHub Issues are the optional durable backlog; a user task and the local working tree are the routine development context. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default Matt Pocock triage state labels when a GitHub Issue is being triaged. See `docs/agents/triage-labels.md`.

### Domain docs

Use a single shared project context and root ADR directory; create domain notes lazily when decisions need them. See `docs/agents/domain.md`.
