# 02-durable-execution

Status: In progress (owner-authorized 2026-09-29)
Spec revision: S2
Type: implementation/verification. Phase: 1. Blocked by: none (01 locally accepted).
What/why: ledger, outbox, attempts, leases and effect reconciliation.
Method: EP1's 10 crash boundaries × 1,000 seeds, including corrupt state, disk full, orphan artifacts, backup/GC leases, restart and cancellation. Also freeze explicit DUR-07/08 fixtures: take a backup, complete an opaque effect and revoke a grant, restore the older backup, retry the old client command and submit a stale-worker result; test two coordinators competing for ownership. These additional restore/ownership fixtures do not replace any EP1 case.
Acceptance: 10,000 reproducible injected crashes plus every added restore/ownership fixture pass; ordinary crash recovery retains acknowledged state; opaque ambiguous effects enter unknown_effect; expired grants cannot dispatch. Restore stays quarantined in a fresh incarnation, rejects old commands/credentials, never reactivates restored grants/intents and requires the cutoff/loss report, reconciliation and explicit authorization specified in DUR-07. Only the exclusive current coordinator can dispatch.
Rule: DIES if any ordinary-recovery acknowledged transition is lost, an opaque effect is blindly replayed, or rollback activates stale authority or conceals acknowledgement loss; SURVIVES if all stated acceptance criteria pass; otherwise WEAKENS. Explicitly confirmed backup rollback is evaluated under DUR-07, not misreported as zero-loss recovery.
Falsifier/null: a database migration alone does not make external effects safe. Risk: power-loss assumptions and filesystem behavior. Owner: runtime lead.

Implementation is tracked by the active Codex goal. Design decision: see
[`docs/adr/0002-durable-local-ledger.md`](../docs/adr/0002-durable-local-ledger.md).
Scaffold checks do not satisfy this ticket's implementation acceptance. The 10,000
crash cases and added restore/ownership fixtures remain mandatory acceptance work.

## Foundation progress (2026-09-29)

The first local milestone adds Linux profile ownership, a pinned local-filesystem
boundary, the SQLite v1 schema, explicit profile creation versus existing-only open,
profile identity/generation markers, exact schema preflight, and quarantine refusal on
normal open. The marker catches partial database rollback when it survives, and normal
open refuses to recreate a profile after both state files are lost. The core currently
has 39 passing Rust tests, including fail-closed initialization, overflow preflight,
schema-object rejection, and ordinary torn-marker recovery; `npm run check` passes.

Known blocker before acknowledging or dispatching any command: the database and marker
share a backup/rollback domain. A coherent snapshot rollback of both files—and
acknowledged state loss within one coordinator generation—cannot be detected by this
marker. A freshness witness outside the profile backup set and a recoverable
acknowledgement protocol are still required. The command writer remains shutdown-only;
command/event transactions, artifacts, backups, restore, and effect fencing are not
implemented. This foundation is not production-ready and does not satisfy DUR-01 or
DUR-07.

Normal open cannot create a missing database from an `Initializing` marker; explicit
create is required to finish provisioning. Torn nonempty initial-marker bytes fail
closed for both open and create. Their shape cannot prove whether provisioning
stopped before database creation or an initialized profile's marker was truncated
after database loss. Recovery must use an explicit fresh-incarnation restore/reset
path; initialization never guesses from ambiguous local state. A missing/empty marker
and missing database can be treated as uninitialized only by explicit provisioning;
normal coordinator startup must use `Ledger::open` and cannot route lost state to
`Ledger::create`.
