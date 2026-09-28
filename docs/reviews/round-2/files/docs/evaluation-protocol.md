# EP1: Runweft evaluation protocol

Revision EP1, originally frozen with design r2. This project copy updates document references and the script filename only; the sampling, analysis and decision rules are unchanged. The governing utility rule is `spec.md` §11. No real runs have been collected. Before collection, freeze and hash the concrete task, model, tool, workload and analysis manifests. Changing this protocol requires a new revision and a new untouched holdout; changes cannot retroactively validate a failed comparison.

## Comparative cohort

The sampling frame is eligible public, permissively licensed repositories that build in the recorded Linux environment, plus issue/task descriptions for which maintainers or independent evaluators can define objective acceptance. Freeze the entire candidate list and eligibility decisions before selection. Exclude credentials/payments/production writes, unavailable dependencies or unverifiable outcomes based on initial fixtures alone, never on either agent's result. Report the excluded frame and reasons.

Select 24 repositories and five tasks from each (120 total), constrained to 30 bug fixes, 30 features, 30 refactors and 30 investigations. Use a seeded random shuffle (seed 20260928) of lexicographically sorted eligible IDs, selecting the first complete feasible 24-repository allocation meeting category counts; the allocation algorithm and candidate manifest must be published and hashed before use. If no feasible allocation exists, expand the candidate frame before any benchmark exposure or mark the cohort invalid; do not hand-pick replacements after results. These tasks represent the frozen frame, not all software engineering.

Holdout repositories must be disjoint from pilot/tuning repositories. Deduplicate tasks, issue variants and near-duplicate acceptance fixtures before freezing. Record known public benchmark overlap and potential training-data exposure; public tasks cannot prove absence of model training contamination. Two evaluators who did not author the agent prompts write/review acceptance criteria before either system sees tasks. For investigations, use a source-referenced answer rubric and blinded independent scoring; disagreement goes to a third adjudicator under the frozen rubric. Expose required user-facing goals equally, keeping independent acceptance fixtures isolated from agent-editable workspaces. Record all deviations.

Each system gets three fresh attempts per task. Start from byte-identical source snapshots and authorized dirty overlays, fresh process state, empty run memory/caches except a specified equal static cache, identical tool versions, CPU/memory quotas, network policy, wall-clock cap and API-spend cap. Freeze concrete caps per task category before exposure. The harness-only cohort uses identical model IDs, reasoning limits and token/spend ceilings. Provider system wrappers may differ but are recorded; their instructions are part of the evaluated harness. A separately labelled routing cohort is a separate experiment and cannot supply a missing primary result.

Randomize system order per task/attempt with the frozen seed and execute each pair in the same provider/time block, targeted within one hour. Pin provider snapshots when supported; otherwise record requested/returned model IDs, fingerprints if offered, region, request time, provider release observations and calibration fixture results. A known model change or systemic provider outage invalidates the entire comparative cohort for a decision, retaining all results and costs in the record; any replacement cohort needs a new preregistration. Unknown drift is a residual limitation, reported rather than claimed eliminated.

Agent timeouts, exhausted budgets, bad arguments, invalid patches and agent-caused cancellations are valid failed attempts (accepted=false), with all spend included. External measurement failure, missing/unknown spend, broken initial fixtures or infrastructure outages cannot be silently excluded or turned into successes: mark the cohort invalid/WEAKENS. Audit this classification without system labels where possible. No selective reruns. Workspace reset checksums and logs prove isolation for every attempt.

## Analysis contract

The input to `evals/analyze_evaluation.py` is one JSON document:

```json
{
  "protocol": "EP1",
  "cohort_valid": true,
  "release_blocker": false,
  "tasks": [
    {"id": "task-id", "repo": "repo-id", "category": "bug",
     "omo": [{"accepted": true, "api_cost_usd": 1.0}],
     "runweft": [{"accepted": true, "api_cost_usd": 0.7}]}
  ]
}
```

This abbreviated shape illustrates fields only; a valid input requires 120 tasks, 24 repositories × 5 tasks, 30 of each category `bug|feature|refactor|investigation`, and exactly three attempt records per system per task. `accepted` must be Boolean; cost must be finite/nonnegative or null when unknown. The evaluation harness creates these inputs from immutable acceptance receipts and metering records. The analysis script validates structure and computes the decision; it cannot authenticate those upstream facts. Hash raw evidence, input JSON, script, Python version and output together.

For each system, acceptance is accepted attempts / 360 (equivalent to averaging the three attempts per task). Cost per accepted attempt is total API spend across all attempts / accepted attempts. Difference Δ is Runweft acceptance minus OmO acceptance; ratio R is Runweft cost-per-acceptance / OmO cost-per-acceptance. Human minutes, local compute, p50/p95 wall time, cost distribution, per-category quality and failure reasons are mandatory descriptive outputs from the harness, not hidden by this two-metric utility rule. Subscription-only unmetered providers require a separate study; do not treat unknown spend as zero.

Bootstrap 10,000 samples with Python `random.Random(20260928)`: sort repository IDs, draw 24 with replacement, retain each chosen repository's five paired tasks and all six attempt records. This preserves within-task and within-repository dependence. Compute Δ and R from each full resample. Use linearly interpolated quantiles at 0.0125 and 0.9875 for each metric (individual nominal 97.5% percentile intervals). Bonferroni targets joint 95% coverage for the two quantities under valid bootstrap assumptions; the two success branches use the same joint intervals. With only 24 clusters, coverage is approximate and precision may be poor; report that limitation. This is not an exact finite-sample test or a claimed power calculation.

If observed costs/acceptances make R undefined (including zero OmO cost-per-acceptance), or **any** bootstrap resample yields undefined R, conservatively return WEAKENS with the reason and no superiority claim. Null/missing metering or invalid cohort also yields WEAKENS. No resample is silently dropped. A confirmed unresolved release blocker takes precedence and returns DIES, even if the data are otherwise incomplete. Otherwise apply `spec.md` §11's strict inequalities. Equality at any threshold does not satisfy that inequality. One preregistered decision is made; no repeated holdout peeking.

## Reproducible operational workloads

These are release-test definitions, not executed results. Freeze fixture IDs, seed-to-schedule mapping, expected outcomes, platform/runtime versions and evidence digests before execution. Missing required fixtures blocks the claim.

Crash matrix: 10 boundaries × 1,000 seeded schedules = 10,000 coordinator crash cases: (1) before SQL commit; (2) after commit/before acknowledgement; (3) after acknowledgement/before dispatch; (4) after outbox claim/before invocation; (5) during invocation; (6) after external effect/before result receipt; (7) after result receipt/before durable outcome; (8) artifact flush/rename/reference window; (9) snapshot/backup/GC pin window; (10) lease revocation/cancellation/owner replacement. In every boundary allocate 250 schedules to each effect class: read-only, idempotency-keyed, externally reconcilable, opaque. A fixture must identify both expected durable state and whether automatic replay is allowed. Scenarios include dirty shutdown, disk-full write errors and deliberately corrupted state; distinguish process-crash tests from power-loss/filesystem-fault tests and do not claim one proves the other.

Policy inventory: for every advertised secure platform/backend, cover 14 classes, each with at least 10 deny cases and 10 minimally permitted controls: path traversal/symlinks/hardlinks; mount/home escape; inherited environment/secrets; direct network/DNS/socket bypass; credential misuse; compromised Node/MCP host; broker IPC impersonation; stale/revoked leases; child-grant amplification; parent/child context leakage; summary/cache/provider disclosure; malicious tool/schema content; local-web origin/CSRF; output/resource exhaustion. Count and publish every executed fixture. Any confirmed unauthorized effect blocks release; a missing case blocks certification. Trusted-local mode must pass honest-labelling and consent tests and is not counted as secure.

Latency workload: record exact CPU, storage, OS/kernel, filesystem, runtime builds and power mode on the 8-core/16-GB/SSD reference host. Maintain 32 active synthetic attempts with deterministic stubbed providers. Warm up 2,000 commands, then issue 10,000 commands using an open-loop 20-command/s schedule and seed 20260928: 40% status reads, 30% submit/dispatch acknowledgements, 20% cancel/pause acknowledgements, 10% approval decisions with durable commits. Requests are timestamped at intended send time so client queuing counts. Measure through complete local response (after required commit); include timeouts as infinite latency and report them. Calculate aggregate and per-class p95 by nearest-rank ceil(0.95 × n); require all five values below 150 ms and zero request errors/timeouts. Publish queue depth, CPU and peak resident memory. This target describes this workload only and excludes model execution, not local coordination work.

The finite crash/security suites support bounded release claims. External security review and production monitoring remain necessary. The script's synthetic self-checks establish calculation behavior only; they are not Runweft performance evidence.
