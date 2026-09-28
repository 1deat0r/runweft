# Current specification board

Current revision: **S2**. Status: **approved, 5/5 BUILD**, 2026-09-28.

This record covers the new project spec and scaffold. Earlier approvals in
`docs/research/` are historical and cannot substitute for this board.

Rounds use immutable file snapshots under `round-N/`, a SHA-256 manifest, and
isolated reviewer prompts. Per-seat verdicts and operator adjudication are recorded
after each round. Final approval requires every seat BUILD and quote-verified closure
of every material blocker. Model diversity remains within the available provider;
this is not human or cross-vendor certification.

| Seat | Agent / model | S1 | S2 |
|---|---|---|---|
| Systems / durability | `spec_systems` / GPT-6 Astra | CONDITIONAL | BUILD; SYS-1 quote-verified PASS |
| Adversarial security | `spec_security` / GPT-6 Astra | CONDITIONAL | BUILD; SEC-1 quote-verified PASS |
| Technical / build | `spec_build` / GPT-6 Sol | BUILD | BUILD |
| Evaluation / confounds | `spec_evaluation` / GPT-6 Sol | BUILD | BUILD; reference cleanup PASS |
| Product / scope | `spec_product` / GPT-6 Sol | Reserved for cold read | BUILD; no prior-round exposure |

All reviewers used high reasoning effort. Up to three reviewed simultaneously because
the session permits three child agents. S1 completed before edits; all five final votes
cover the same frozen S2 snapshot. There was no peer discussion or forced consensus.
These are independent review procedures, not a claim of statistically independent errors.

Material fixes: restore quarantine/incarnation/fencing and authoritative classification
of all untrusted outputs. See [adjudication](adjudication.md), [S1 manifest](round-1/manifest.json),
[S2 manifest](round-2/manifest.json), and the five per-seat S2 records in `round-2/`.
S2 full reviewed spec SHA-256:
`fd254bafe68dda434ed03074c05264d8c73014a3247186c60f89249ce07f3655`.
The delivered spec differs only in approval/record metadata; sections 1–11 are unchanged.
`handoff-integrity.json` records that boundary and final hashes.

Approval means suitable to implement/test. It does not establish runtime performance,
security, usability or superiority. All eight runtime tickets remain Proposed.
Current evidence is the local scaffold check recorded in `docs/verification/scaffold-checks.md`.
Nonblocking limits remain: Node 24/remote CI/browser interactions untested, optional
literal status regression assertions deferred, and the historical logical diagram
needs its worker/verification arrow clarified before detailed implementation use.
