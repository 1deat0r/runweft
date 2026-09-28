## Purpose and scope

- What changed and why:
- User-authorized scope and relevant spec/ticket, if any:
- Out-of-scope work left untouched:

## Verification and evidence

- Local commands and results (include versions when relevant):
- Known risks, limitations, and omitted checks:
- Evaluation or independent acceptance evidence, when relevant:

## Optional independent agent review

A structured receipt can record independent reviews for substantial changes. It is
optional; the template does not add one by default. If you include a receipt, add one
visible Markdown code block tagged `agent-review` and use the v2 format in
[the receipt guide](https://github.com/1deat0r/runweft/blob/main/docs/agents/agent-review-receipt.md). CI validates its shape
and commit binding when supplied. It cannot prove that an agent ran.

## Author checklist

- [ ] I read the relevant project instructions and spec.
- [ ] Generated bindings come from the schema and `npm run generate`.
- [ ] I ran `npm run check` or recorded why it could not be run.
- [ ] No credentials, private user data, or vulnerability details are in this public PR.
- [ ] Every recorded blocker has a verified closure in the final diff.
