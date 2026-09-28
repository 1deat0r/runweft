# Adversarial security — S1

Reviewer: `/root/spec_security`, GPT-6 Astra, high effort. Independent frozen read.
Verdict: **CONDITIONAL**.

Material blocker SEC-1: output-label assignment must be authoritative across arbitrary
computation. SEC-04 (S1 lines 212–218) labels derived summaries and rejects missing
labels, but leaves arbitrary output labels/provenance unspecified. A compromised
extension can encode restricted input into an artifact/tool argument and claim a
permissive label or omit input dependencies. Checking false classifications does not
enforce the rule that models cannot declassify data. Threat-model lines 20–25 and
tickets 03/05 must cover this boundary.

Closure requested: coordinator-owned labels; untrusted producers cannot lower
restrictions or author authoritative provenance. Conservatively label all outputs,
including arguments, artifacts, logs and encoded content, from every input exposed
to the execution context. Check every outbound effect. Narrower labels require a
specified trusted mechanism or authorized user declassification. Add mislabeled
artifact, omitted-dependency and encoded-exfiltration fixtures to tickets 03/05.

Containment, broker scoping, explicit credential TCB exceptions, current revocation,
fencing and unknown-effect reconciliation are coherent requirements. No executed
security-test evidence was reviewed; approval would not establish runtime security.

This file records the review finding and closure condition; the conversation retains
the full original response.
