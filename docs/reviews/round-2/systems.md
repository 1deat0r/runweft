# Systems / durability — S2

Reviewer `/root/spec_systems`, GPT-6 Astra, high effort. Verdict: **BUILD**.

SYS-1 **PASS**, quoting S2 DUR-07: “Restore starts quarantined in a fresh,
unpredictable profile incarnation and coordinator generation”; “Restored grants,
ready nodes, dispatch intents and attempts MUST NOT automatically become actionable.”
It also says “Reject commands for the previous incarnation, including client retries,
without executing them.” The reviewer verified reconciliation, cutoff/loss reporting,
fresh authorization, and ticket 02's exact old-backup/retry/stale-worker sequence.

Ownership note **PASS**: DUR-08 requires “acquire exclusive OS-enforced profile
ownership before dispatch and retain it through shutdown.” Ticket 02 covers competing
owners and stale generations. Lifecycle note **PASS**: ticket 01 says “Freeze the
transition table, error union, profile incarnation and coordinator generation fields
before execution work,” with both frozen contract and 50/50 fixtures required.

No new material blocker. This is implementation-contract approval; runtime durability,
containment, restore and future fixture outcomes remain unverified. Read-only review.
The operator independently verified the material closure quotes against frozen S2.
