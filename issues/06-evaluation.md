# 06-evaluation

Status: Proposed
Spec revision: S2
Type: empirical gate. Phase: 0 baseline; phase 2 decision. Blocked by: baseline contract; successor comparison requires 02–05.
What/why: decide whether the successor improves useful completed work.
Method: EP1's preregistered 120-task, 24-repository, 3-attempt paired protocol and `../evals/analyze_evaluation.py`; immutable task/criteria/provider/workload manifests and script digest before evaluation. Pilot repositories are disjoint.
Acceptance and numeric decision rule: exactly the ordered DIES/SURVIVES/WEAKENS utility rule in spec.md §11; this is the sole governing rule for comparative superiority.
Falsifier/null: Runweft is no better than OmO at matched resources. Risk: task selection, provider changes, correlated attempts and weak statistical precision. Owner: evaluation lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
