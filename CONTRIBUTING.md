# Contributing to Runweft

**Policy date: 2026-09-28.** The public repository at
<https://github.com/1deat0r/runweft> is the canonical record. Work in the linked
local checkout is welcome, but every change after the one-time bootstrap commits
must reach `main` through GitHub.

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
- Every human-authored pull request must reference a GitHub issue; CI blocks PRs
  with no `#<number>` reference in the description. Dependabot update PRs are the
  only issue-reference exception and remain subject to all checks and review gates.
- Keep a pull request focused and link its issue with `Closes #<number>` when the
  change fully resolves that issue. Explain scope, contract changes, evidence,
  risks, and anything not checked using the pull request template.
- Do not commit or push directly to `main`. The protected branch requires both
  Node matrix checks, an up-to-date branch, resolved conversations, linear history,
  and an independent approval from a GitHub user with write access who did not
  make the latest push. Stale approvals are dismissed, and administrators are
  subject to the same gate. There is no bypass path. If a second authorized reviewer
  is not available, keep the PR open rather than weakening the gate.
- Squash merge approved pull requests. GitHub deletes the merged feature branch.

## Independent review

The person or agent implementing a change cannot count as its independent reviewer.
Before requesting review, freeze the PR diff and identify the relevant seats from
the project review board:

- systems and durability;
- adversarial security;
- implementation and developer experience;
- evaluation and confounds;
- product and scope.

Review the changed area with the relevant seats and record the reviewers and scope
in the PR. Explain why an omitted seat is not relevant. Each reviewer works from
the same frozen diff and reports independently; do not share peer verdicts before
votes. Record blockers with exact file/line references, impact, and a testable
closure condition. Re-check each closure against the final diff and evidence. Changes
to normative spec or architecture use the complete frozen-snapshot board process in
`spec.md` §12; approval of the implementation plan does not imply runtime acceptance.

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
