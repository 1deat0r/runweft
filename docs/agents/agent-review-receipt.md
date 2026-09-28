# Optional agent review receipt

Use a receipt when independent agent reviews add useful audit evidence to a
substantial or high-impact pull request. Routine changes do not need one. Add a
single visible Markdown code block tagged `agent-review` to the PR body; do not use a
placeholder receipt. The CI job accepts no receipt and validates any receipt that is
present.

The v2 receipt binds the reports to the exact PR head. Include the implementation
and evaluation seats when using the receipt. Add adversarial security,
systems/durability, and product/scope seats only for the risks that apply to the
changed paths and behavior. Each reviewer must be a distinct agent separate from the
implementer. A receipt records a claim; it is not trusted proof that an agent ran.

Example for a change whose risk needs only the two baseline seats:

```json
{
  "protocol": "runweft-agent-review/v2",
  "reviewed_head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "implementer_agent": "implementer-17",
  "required_seats": [
    "implementation/developer-experience",
    "evaluation/confounds"
  ],
  "reviews": [
    {
      "seat": "implementation/developer-experience",
      "reviewer_agent": "reviewer-implementation-4",
      "reviewed_head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "verdict": "APPROVE",
      "summary": "Reviewed the final diff and the scoped local verification."
    },
    {
      "seat": "evaluation/confounds",
      "reviewer_agent": "reviewer-evaluation-9",
      "reviewed_head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "verdict": "APPROVE",
      "summary": "Checked fixed acceptance cases and likely confounds."
    }
  ],
  "evaluation": {
    "reviewer_agent": "reviewer-evaluation-9",
    "result": "PASS",
    "scope": "The affected feature's stated acceptance behavior.",
    "evidence": "Named deterministic checks and observed outcomes."
  },
  "findings": []
}
```

Every review must identify its seat, reviewer, exact head SHA, verdict, and summary.
`required_seats` must match the reviews and include seats required by the changed
paths. Evaluation is `PASS` with evidence or `NOT_APPLICABLE` with a reason and scope.
Findings have unique numbered IDs and a severity. Blockers also include a location,
impact, closure condition, and a quote with evidence from the final revision.
