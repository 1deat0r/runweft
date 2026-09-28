# Adversarial security — S2

Reviewer `/root/spec_security`, GPT-6 Astra, high effort. Verdict: **BUILD**.

SEC-1 **PASS**. S2 SEC-04a says “the coordinator owns classification and provenance”
and “An untrusted producer MUST NOT lower labels or declare an authoritative
dependency set.” It includes all exposed inputs, mounts and retained state; reducing
classification requires a clean isolated context boundary.

“All outputs inherit that context classification, including artifacts, code, tool
arguments, logs, summaries and encoded/transformed content.” “Every outbound effect
checks both these restrictions and the complete payload/destination.” Automatic
declassification is prohibited; the user's authorization binds artifact/action and
recipients. TM2 and tickets 03/05 carry the required adversarial fixtures.

No revision-introduced blocker found, including in DUR-07/08. Conservative labels may
restrict reuse and outbound calls after broad workspace exposure: an explicit tradeoff.
Production containment, propagation, revocation and recovery remain unimplemented.
The operator independently verified the material closure quotes against frozen S2.
