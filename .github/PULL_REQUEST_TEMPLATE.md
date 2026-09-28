References #<N>

## Issue and authorization

- User-authorized scope:
- Relevant ticket and spec revision:
- Out-of-scope work deliberately left untouched:

## Change and evidence

- What changed and why:
- Contracts, invariants, and compatibility impact:
- Verification commands and results (include versions):
- Independent acceptance evidence or reason it does not apply:
- Known risks, limitations, and omitted checks:

## Independent review seats

Keep the JSON receipt in the PR description. CI requires a valid receipt tied to the
exact head commit, with distinct implementation and evaluation agents. Add all
risk-based seats. Set `full_board` to true for normative spec/architecture,
review-policy, workflow, and branch-protection changes. In that case all five
independent seats must review the same frozen commit before publishing the receipt.

```agent-review
{
  "protocol": "runweft-agent-review/v1",
  "reviewed_head_sha": "REPLACE_WITH_40_CHARACTER_HEAD_SHA",
  "implementer_agent": "REPLACE_WITH_IMPLEMENTER_ID",
  "full_board": false,
  "required_seats": [
    "implementation/developer-experience",
    "evaluation/confounds"
  ],
  "omitted_seats": [
    { "seat": "systems/durability", "reason": "REPLACE_WITH_OMITTED_SEAT_REASON" },
    { "seat": "adversarial/security", "reason": "REPLACE_WITH_OMITTED_SEAT_REASON" },
    { "seat": "product/scope", "reason": "REPLACE_WITH_OMITTED_SEAT_REASON" }
  ],
  "reviews": [],
  "evaluation": {
    "reviewer_agent": "REPLACE_WITH_EVALUATION_REVIEWER_ID",
    "result": "PASS",
    "scope": "REPLACE_WITH_EVALUATION_SCOPE_AND_POTENTIAL_IMPACT",
    "evidence": "REPLACE_WITH_ACCEPTANCE_EVIDENCE"
  },
  "findings": []
}
```

Each review item must include `seat`, `reviewer_agent`, `reviewed_head_sha`,
`verdict` (`APPROVE`), and a concise `summary`. Evaluation may be
`NOT_APPLICABLE` only with a `reason` instead of `evidence`. Every finding must
include a unique numbered ID (such as `SYS-1`) and severity; blockers also include
location, impact, a closure condition, and closure evidence with a quote from the
final diff. List every seat
not included in `required_seats` under `omitted_seats` with a reason; use an empty
array for a full five-seat board.

## Author checklist

- [ ] The issue and PR remain within user-authorized scope.
- [ ] I read `README.md`, `STATE.md`, `spec.md`, and the relevant ticket.
- [ ] Generated bindings come from the schema and `npm run generate`.
- [ ] I ran the required checks and recorded omissions honestly.
- [ ] No credentials, private user data, or vulnerability details are in this public PR.
- [ ] Every blocking review finding has a verified closure in the final diff.
- [ ] The receipt names independent review agents and is tied to the current PR head.
