# Operator adjudication — S1 to S2

All S1 reviewers completed before normative edits. The S1 manifest was independently
checked against all 73 live and frozen files before revision. No exported code symbol
changed; a call-graph blast-radius audit is not applicable to this documentation revision.

| Finding | Ruling and source evidence | Resolution in S2 |
|---|---|---|
| SYS-1: restoring an older backup can revive effects/grants and forget command outcomes | **CONFIRMED.** S1 DUR-03 verifies hashes; DUR-02/05 and SEC-05 assume available durable facts. No text defines rollback across later effects/revocations. | DATA-01 incarnation fields; DUR-07 quarantine, fresh identities, reconciliation, stale-command rejection, loss disclosure and reauthorization; DUR-08 ownership fencing; TM2 and ticket 02 explicit restore sequence. |
| SEC-1: producer can falsely classify arbitrary output | **CONFIRMED.** S1 SEC-04 explicitly covers summaries and missing labels, but does not establish authoritative provenance or taint of arbitrary process outputs. | SEC-04a coordinator-owned monotone restrictions across every exposed input and output, mounted data/reused state, no automatic declassification; TM2 and tickets 03/05 fixtures. |
| Systems note: ownership enforcement | **CONFIRMED, nonblocking clarification.** ARCH-01 states singleton intent, while enforcement was not named. | DUR-08 and ticket 02 exclusive ownership/current-generation checks. |
| Systems note: ticket 01 lifecycle deliverables | **CONFIRMED, nonblocking checklist gap.** S1 STATE-03 requires the table/union; ticket 01 only names interoperability fixtures. | Ticket 01 explicitly freezes table, error union and incarnation/generation fields; provider round trip explicitly stubbed. |
| Build note: independent literal status assertions | **CONFIRMED, nonblocking enhancement.** Current schema/code are scaffold/false; equality test alone would permit coordinated drift. | Deferred to future contract-test strengthening; disabled-execution integration test and code inspection remain valid current evidence. No runtime code changed. |
| Build note: Node 24 / remote CI evidence absent | **CONFIRMED, disclosed limitation.** Verification receipt names only Node 26 and explicitly lists untested environments. | No unearned claim added; CI remains a recipe. |
| Evaluation note: historical cross-references | **CONFIRMED, editorial.** EP1 referenced prior design §12 and hyphenated script. | Project EP1 copy points to spec §11 and underscore script; provenance records reference-only changes. Sampling/analysis/decision semantics remain byte-identical apart from these references. |
| Evaluation scope: local ticket DIES vs comparative utility | **CONFIRMED, not a blocker.** S1 §11 separately blocks associated release claims on operational failures. | Preserve the registered utility rule; stronger restore/ownership gates apply independently to ticket 02 and release eligibility. |

S2 closure verification: systems and security reviewers quoted the resolving clauses
and returned PASS for every prior material blocker. The operator checked eight key
closure quotes directly against frozen S2, with whitespace normalization only, and
verified all 73 live/frozen file hashes before final metadata changes. Systems also
closed its ownership/lifecycle notes. Build and evaluation repeated BUILD on S2;
the latter quote-verified the reference cleanup. The new product cold reader returned
BUILD without seeing any earlier review or peer output. Final tally: **5 BUILD / 0
CONDITIONAL / 0 REJECT**, no open material blockers.

Cold-reader note PROD-N1: **CONFIRMED, nonblocking.** The Archify JSON contains logical
worker/verification links, while S2 §4 explicitly limits diagram authority and WORK-02
requires independent acceptance checks. The logical map is retained as historical;
clarify the arrow before using it as a detailed implementation guide. This is recorded
without pretending a new visual check occurred. Phase-specific delivery remains
visible in the eight Proposed tickets.
