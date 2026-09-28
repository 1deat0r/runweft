# Contributing to Runweft

**Policy date: 2026-09-28.** The public repository at
<https://github.com/1deat0r/runweft> is the canonical record. Work in the linked
local checkout is welcome, but every change after the one-time bootstrap commits
must reach `main` through GitHub. Runweft is maintained by one person with
autonomous expert agents; independent agent reviews and evaluations are required
for each pull request.

## Before implementation

1. Read `README.md`, `STATE.md`, `spec.md`, and the relevant ticket under `issues/`.
2. Find or open a GitHub issue describing the problem, intended scope, acceptance
   evidence, risks, and affected requirements. Link the issue in the pull request.
3. Get user authorization for the proposed implementation scope. Issues, prior
   board votes, and passing CI do not authorize new runtime work. In particular,
   tickets 01–08 remain proposed until the user authorizes them.

## Branch and pull request

- Branch from the current `main` using `feat/<issue>-<slug>`, `fix/<issue>-<slug>`,
  `docs/<issue>-<slug>`, or `security/<issue>-<slug>`.
- Every human-authored pull request must identify its GitHub issue with `Closes #N`,
  `Fixes #N`, `Resolves #N`, or `References #N`; CI rejects PR descriptions without
  a standalone reference outside fenced code blocks. CI verifies that the referenced
  number is an issue in this repository; reviewers verify that the issue authorizes
  the change. Dependabot update PRs may omit an issue reference, but remain subject
  to all CI and agent-review requirements.
- Keep a pull request focused and link its issue with `Closes #<number>` when the
  change fully resolves that issue. Explain scope, contract changes, evidence,
  risks, and anything not checked using the pull request template.
- Do not commit or push directly to `main`. The protected branch requires the Node
  24 and 26 checks, the structured agent-review receipt check, an up-to-date branch,
  resolved conversations, and linear history. GitHub human approval count is zero
  to support the solo maintainer. The review receipt check is a completeness check,
  not proof of agent identity or execution; independent agent runs and recorded
  reports are required by project policy. The repository has no trusted agent
  attestation integration. Administrators are included in branch protection, and
  project policy forbids bypassing it. GitHub administrators can technically change
  the rules; any policy or settings change must itself follow this issue-and-PR
  process and include an audit record.
- Squash merge pull requests after required agent reviews and CI pass. GitHub deletes
  the merged feature branch.

## Independent review

The implementation agent cannot count as an independent reviewer. Every PR needs at
least two distinct reviewer agents: implementation / developer experience and
evaluation / confounds. Before requesting review, freeze the PR head commit and
identify additional seats based on risk:

- systems and durability;
- adversarial security;
- implementation and developer experience;
- evaluation and confounds;
- product and scope.

Review the changed area with all required seats and record reviewer agent IDs,
role, verdict, summary, and the frozen commit SHA in the PR's `agent-review` JSON
receipt. The evaluation reviewer records acceptance evidence or explains why
evaluation is not applicable, plus evaluation scope and potential impact. Explain
why any risk seat was omitted. Every reviewer works from the same frozen commit
with a separate prompt and reports independently; do not share peer verdicts or
findings before votes. Record why every omitted seat is not relevant. Record
findings by stable ID; blockers include exact file/line references, impact, and a
testable closure condition. Re-check each closure against the final diff and evidence.
Any new commit invalidates all previous review receipts. Changes to
normative spec or architecture, this review protocol, CI workflows, or branch
protection require all five seats and use the complete frozen-snapshot process in
`spec.md` §12. Approval of an implementation plan does not imply runtime acceptance.

The GitHub Actions `agent-review-record` check validates the public receipt's
structure, reviewer distinctness, required seats, and exact head SHA. A receipt is
not cryptographic proof that agents ran; Runweft currently has no trusted agent
attestation service. Because pull-request workflows and the validator are part of
the candidate diff, a PR can alter the check it runs. Workflow, receipt-validator,
and branch-rule changes therefore require the full five-seat review, including
adversarial review. The owner or orchestrating agent must independently launch the
reviewers, preserve their separate reports, and publish a sanitized summary only
after all votes. Keep vulnerability details private per [SECURITY.md](SECURITY.md).

## Verification and evidence

For scaffold changes, run:

```sh
npm ci --ignore-scripts
npm run check
```

GitHub Actions runs the same scaffold check on Node 24 and Node 26. Report local
tool versions, commands, outcomes, and omissions. Do not call live providers, read
credentials, or run arbitrary tools for verification. Use isolated fixtures for
future integration work. A green scaffold workflow proves only the checks it ran;
it does not establish runtime security, durability, usability, or performance.

Keep contracts schema-first. Update the source schema and run `npm run generate`;
never hand-edit generated bindings. Preserve the scaffold's disabled execution,
the Rust `unsafe` prohibition, and the rule that adapters cannot own durable state
or permissions. Do not present placeholders or planned components as implemented.

## Reporting a vulnerability

Do not put vulnerability details in a public issue or pull request. Use GitHub's
private vulnerability reporting for this repository; see [SECURITY.md](SECURITY.md).

For GitHub's documented behavior, see [protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
and [secure use of GitHub Actions](https://docs.github.com/en/actions/reference/security/secure-use).
