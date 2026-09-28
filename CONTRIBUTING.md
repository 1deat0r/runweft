# Contributing to Runweft

Runweft is a solo-maintained project developed primarily by autonomous agents. Work locally first. GitHub is the source-control backup, optional long-lived task tracker, remote clean-environment verifier, and release/distribution surface. This guide describes the default workflow; tasks, user instructions, and enforced platform protections take precedence.

## Local development loop

For ordinary work:

1. Read `README.md`, `STATE.md`, `spec.md`, and the relevant ticket when one applies. Search for existing behavior and inspect the current Git state.
2. Implement only the authorized scope. Keep changes focused and preserve unrelated work.
3. Run `npm run check`, the canonical local `VERIFY` command.
4. Review the full diff, including staged and untracked files. Check for secrets, debug output, accidental generated files, and unrelated changes.
5. Make a small, descriptive, atomic commit that leaves the repository in a valid state.

Do not create an Issue, branch, or pull request just to satisfy process. Do not wait for GitHub Actions when local evidence is sufficient to continue. Do not rewrite existing commits or discard other agent/user work without an explicit reason.

## Verification

`npm run check` is the one required pre-commit verification entry point. It checks generated contract drift, Rust formatting, Clippy warnings, Rust workspace tests, TypeScript build and tests, Python helper tests, and the offline evaluation self-check. It fails non-zero on any failed step and prints each command.

For a fresh checkout or changed dependencies, install from the lockfile first:

```sh
npm ci --ignore-scripts
npm run check
```

Do not reinstall dependencies for every change. Do not call live providers, read credentials, or run arbitrary repository tools as verification. Use isolated fixtures for future integration work. Report the exact local commands, results, versions when relevant, and any omitted check with its reason. Scaffold checks establish only the checks they run; they do not establish runtime security, durability, usability, or performance.

## Issues, branches, and pull requests

GitHub Issues are optional. Use them for durable backlog items, deferred bugs, multi-session work, dependencies, major features or architecture, external reports, or coordination that benefits from a shared record. A small task that can be implemented, verified, and committed now does not need an Issue. An Issue, spec approval, or CI result does not authorize work beyond the user's instructions.

Branches and worktrees are optional. Use the normal development branch when the task is safe to do there and repository protections permit it. Isolate work when concurrency, risky experimentation, rollback, or a useful PR makes that worthwhile.

Pull requests are optional. Use one for major architecture or policy changes, security-sensitive or risky refactors, external contributions, concurrent work needing shared review, or when repository protections require it. When opening a PR, include a concise purpose, scope, verification evidence, and known limitations. Link an Issue only when one already exists or the durable tracking value justifies creating it. Do not use a PR just to wait on CI before continuing local work.

The `main` branch remains protected against force-pushes and deletion, requires linear history and admin enforcement, and requires PR conversations to be resolved. CI runs the clean-environment Node 24/26 checks on pushes and PRs, but branch protection does not make those remote checks a commit or push gate. `npm run check` is the required pre-commit gate. This is a manual agent procedure, not a GitHub-enforced rule: a writer can push an unchecked commit to `main`, and CI reports its result after the push. Use remote results as an independent safety signal, address failures promptly with a follow-up commit or revert, and continue unrelated local work while checks run. Never bypass an active repository protection.

## Independent review and evaluation

Review the diff and run relevant local evidence on every change. Add independent agents when the risk, impact, or uncertainty makes another perspective valuable. Choose seats by the work: implementation/developer experience, evaluation/confounds, systems/durability, adversarial security, and product/scope are perspectives, not a mandatory five-person board. For agent behavior changes, define success and failure cases before implementation; prefer deterministic scenarios and assertions, and retain traces when tool selection, handoffs, or guardrails matter. Do not rely on an LLM judge alone.

For substantial or high-impact work, freeze the revision under review. Keep independent reviewers' first-pass findings separate, record numbered findings and evidence, and verify blocker closures against the final artifact. Changes to security controls, CI workflows, the review-receipt validator, branch-protection settings, or trust-boundary policy require an independent adversarial security review. Preserve a concise review/evidence record for those changes in a PR, commit description, or scoped audit note. A PR may include a structured `agent-review` receipt; it is optional and CI checks its shape and commit binding when supplied. The check is not proof of agent identity or execution. High-impact product decisions still belong to the owner; reviews do not authorize deployment, publication, production access, or destructive actions.

For changes to normative specification or architecture, scale independent review to the actual effects. Include adversarial security review for changes to authority, trust boundaries, or security policy; systems/durability review for state, persistence, protocol, or recovery; product/scope review for user-facing requirements and architecture. A complete five-perspective review is appropriate when a change spans those risks, not automatically because of a file path. Preserve a frozen snapshot and adjudicate material findings with evidence. Historical S2 review records remain historical evidence, not ongoing approval.

## Project constraints

This repository is a scaffold. Do not describe planned scheduler, database, broker, sandbox, provider execution, or live inspector behavior as implemented. Preserve disabled execution, schema-first contracts, the Rust `unsafe` prohibition, and the rule that adapters cannot own durable state or permissions. Run `npm run generate` to update generated bindings; never edit them by hand.

## Security reports

Do not put vulnerability details in a public Issue or PR. Use GitHub's private vulnerability reporting as described in `SECURITY.md`.

For platform behavior, see GitHub's documentation for [protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) and [secure use of GitHub Actions](https://docs.github.com/en/actions/reference/security/secure-use).
