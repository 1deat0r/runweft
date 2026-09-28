# Independent board record

**Final result: 5/5 BUILD on design r2; zero unresolved material blockers.** Date: 28 September 2026. Approval means the architecture is suitable to implement and test. It is not evidence that an unbuilt runtime meets its proposed performance, security or reliability gates.

Five isolated reviewer agents were used in parallel batches, up to three concurrently. They received separate briefs and did not see peer reasoning before voting. The final reviewers read a frozen copy of the design body without the adjudication appendix. The product/visual seat was a cold read, with no earlier round history. All revised objections were verified by direct quotation of the resolving text. Different available model variants were used; all are from the same provider, so independence is procedural and limited, not cross-vendor or human certification.

| Seat | Model | First vote | Final r2 vote | Scope |
|---|---|---|---|---|
| A: domain/migration | gpt-6-astra | BUILD | BUILD | Pinned OmO source, fair comparisons, languages, migration |
| B: systems/data | gpt-6-sol | BUILD | BUILD | Transactions, outbox, artifacts, backup, leases, remote evolution |
| C: adversarial security | gpt-6-astra | REJECT | BUILD | Extension containment, secrets, child authority, disclosure |
| D: evaluation/statistics | gpt-6-sol | CONDITIONAL | BUILD | Sampling, confounds, executable decision rules, operational workloads |
| E: product/visual, cold read | gpt-6-astra | Final round only | BUILD | Usability, architecture fidelity, language rationale, visual evidence |

## Material blockers and verified closures

**C1, extension-process authority — closed.** The security seat required an OS enforcement boundary for untrusted Node hosts, rather than process separation alone. Final verification quoted §6: “Extension containment is mandatory in secure mode” and “only its broker IPC handle.” The revision excludes ambient home/ledger/secret/network access, adds explicit credential-adapter trust, and requires compromised-host tests. C returned PASS.

**C2, delegated context disclosure — closed.** A child with limited tool authority could still receive a broader parent prompt. Final verification quoted §6: “Resolving every artifact, memory entry, cached result and context handle requires a read authorization for the receiving subject.” Provider submission is checked again; summaries retain source restrictions, and fallback/cache/delegation cannot erase labels. C returned PASS.

**D1, task selection — closed.** EP1 freezes the sampling frame and eligibility decisions before selection, defines seeded selection and independent acceptance criteria, excludes pilot repositories and records overlap/contamination limitations. D quoted EP1 lines 7–11 and returned PASS.

**D2, statistical procedure — closed.** EP1 plus `analyze-evaluation.py` fixes 10,000 paired whole-repository resamples, quantiles, undefined-ratio handling, conservative invalid-input outcomes and joint-coverage intent. D read the entire script and independently ran the synthetic self-test successfully. D quoted EP1 lines 40–42 and design §12; PASS.

**D3, comparator and attempts — closed.** EP1 specifies fresh attempts, identical inputs and resources, randomized paired execution, provider-version records, cohort invalidation for known drift/outages, and no selective reruns. D quoted EP1 lines 13–17; PASS.

**D4, operational workloads — closed.** EP1 specifies ten crash boundaries × 1,000 schedules, fourteen policy classes with permit/deny controls, and a latency workload with exact mix, sample counts, timeout treatment and p95 computation. D quoted EP1 lines 48–52; PASS.

## Advisory fixes and final dissent record

A confirmed that the importer now explicitly excludes live DAGs, executable hooks and credential transfer, preserves the original installation, and only promises fixture-backed version support. It also confirmed that the language table credits OmO's shared core code instead of asserting blanket duplication.

B confirmed backup/GC leases and digest-verified restoration, plus adapter lease checks before requests. It emphasized that the backup's actual database high-water mark must match the pinned artifact set during implementation.

E found no material visual or product blocker. It directly inspected the 1440×900 light and 2048×1320 dark screenshots and checked the JSON and receipts. It noted small secondary annotations (7.67px minimum projected text in the laptop receipt), simplified logical node categories, and uncertain schedule estimates. These are retained limitations; the diagram is not an accessibility or presentation-size certification. Automated containment thresholds do not prove comfortable typography for every reader.

No dissent remains on building/testing r2. No seat approved measured speed gains, real crash results, secure sandbox behavior, completed migration, or interactive viewer controls. Those remain unverified or future gates.

## Evidence binding

`review-integrity.json` records SHA-256 values for the reviewed body, tickets, EP1, analysis script and diagram. The final design body (sections 1–13) matches the frozen review copy exactly. The status header and board appendix were updated only after all votes completed; no substantive design change was introduced after approval.

Archify deterministic delivery passed 9/9 showcase checks, with zero errors/warnings. Its separate browser receipt passed all four requested desktop sizes. The primary agent inspected all four endpoint screenshots in both themes. Optional in-app navigation to the local HTML was blocked by browser URL policy; no workaround was attempted. Search, focus and export interactions remain untested; that does not change the already completed, artifact-bound automated measurements.
