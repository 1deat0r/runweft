# Acknowledgement freshness witness

Status: accepted for Ticket 02 implementation; release and production use are not
approved. Date: 2026-09-29. Independent adversarial, durability, and evaluation
reviews found no remaining design blockers on the reviewed revision.

## Context

The SQLite ledger and its coordinator marker live in the same profile directory and
can be rolled back together. That permits a coherent old backup to appear current and
can hide acknowledged changes made without a coordinator restart. `spec.md` requires
ordinary recovery to retain acknowledged state (DUR-01) and treats deliberate backup
restore as separately authorized, quarantined recovery in a fresh incarnation
(DUR-07). A marker in the same backup set cannot prove freshness.

## Decision

Keep a private per-profile freshness head outside the profile backup unit, under the
user's local state directory. Use `$XDG_STATE_HOME/runweft/witnesses` when set, else
`$HOME/.local/state/runweft/witnesses`; the chosen root must be absolute. Do not
silently switch roots if a configured path is invalid or a profile's witness is
missing. Reject a witness root that is the profile directory, lies beneath it, or is
an ancestor of it, validating overlap with the pinned directory identities rather
than string-prefix comparison alone. Runweft profile backups contain only the profile
directory and never package or restore the per-user witness. Key profile files by a
stable hash of the profile ID, never by caller-controlled path text. This detects
profile-directory rollback while the witness remains current. It does not detect a
whole-machine restore that rolls back both storage locations, a same-user actor that
can rewrite both, or loss of the entire user state directory. Those cases need a
stronger external witness and are outside this local design's claim.

The Linux file adapter pins every path component through descriptor-relative,
no-follow opens. It requires trusted ancestors, private directories and regular files
owned by the effective user, single-link lock/head files, and a supported persistent
local filesystem for the witness directory. It retains the relevant directory and
lock descriptors. It fails closed on symlinks, writable/untrusted ancestors,
unexpected file type/owner/link count, unsupported filesystems, and I/O errors. The
witness lock inode is stable and is never unlinked. Temporary files are unique,
private, same-directory files. The witness directory is outside the profile backup
unit; this is a profile-only rollback guarantee, not a second backup of the profile.

Only explicit initial provisioning may create the state directory tree or stable lock
entry. After each `mkdirat`, sync the new directory and its parent before relying on
the name. After creating the stable lock file, sync the file and containing directory.
Create and sync the initial witness head before finalizing bootstrap. Normal open never
creates missing directories, lock files, or heads. A crash during initial provisioning
may be resumed only from the valid local `Initializing` marker and its exact pending
bootstrap record; once the profile is active, missing witness state fails closed.

After acquiring the profile-local ownership lock and reading the profile identity,
the coordinator also acquires a stable per-profile OS lock in the witness directory
and holds it through shutdown. This serializes copies of the same profile ID even when
they reside in different profile directories. A second copy cannot advance the
generation, accept commands, or dispatch while another copy owns this lock. The
coordinator advances its generation through the witness protocol before admitting
work. Normal open never creates or resets witness state.

Each head binds profile ID, incarnation, a profile-wide monotonic commit epoch,
coordinator generation, and a previous-head digest. A versioned canonical commit
envelope encodes the complete logical write set using fixed field order and
length-delimited fields. It includes command scope and ID, canonical intent, saved
response and event cursor, ordered events, projection changes, budget changes, and
outbox intent. Later authoritative transactions similarly encode their full changes,
including owner-generation changes, grant/revocation and cancellation state,
leases, attempts, dispatch claims, effect receipts, reconciliation outcomes, and
artifact or backup manifests/references. No authoritative ledger transition may
bypass this commit path. Transient reads and non-authoritative diagnostics do not
advance it.

The envelope is persisted in an immutable commit record. Its digest is a
domain-separated hash of the previous digest and the exact versioned envelope bytes:
`SHA256("runweft-commit-v1" || 0x00 || u64be(len(previous)) || previous ||
u64be(len(envelope)) || envelope)`. The envelope begins with `RWCE`, followed by
big-endian u16 version, kind, and field count; fields are ordered by ascending u16 tag
and encode `tag:u16be || length:u64be || value`. Integers have fixed-width big-endian
encoding; nested maps use UTF-8 canonical sorted-key JSON with no floating-point
values. Each version/kind defines its allowed tags; any unknown version, kind, or tag
fails closed. The pending row stores the
envelope, old head, and proposed head. The finalized commit log stores the same
envelope and digest. On open, the ledger verifies the chain and checks that replaying
its envelopes reconstructs the stored authoritative state and projections. A stored
digest field alone is insufficient. This is an integrity/freshness check, not
protection against a malicious process running as the same user.

The implementation must retain a fixed encoding vector: a `RWCE` version-1,
bootstrap-kind-1 envelope with four fields (tag 1 profile ID `p`, tag 2 incarnation
`i`, tag 3 epoch 1, tag 4 generation 1), and a 32-byte zero previous digest, encodes as
`5257434500010001000400010000000000000001700002000000000000000169000300000000000000080000000000000001000400000000000000080000000000000001` and hashes to
`682b828701d3dbbb4d7441b51d4ec9feaf7b7d2ce9d4b30bb0a375f2239cc22d`. The golden
test also mutates or omits each field, and each command-fact category, to prove that
the resulting envelope cannot validate against the original witnessed head.

Serialize authoritative writes with at most one pending commit per profile:

1. One SQLite `synchronous=FULL` transaction records the pending envelope and its
   proposed next head. Pending command results are not acknowledged history; pending
   outbox entries and later effect state cannot dispatch.
2. Under the held per-profile witness lock, compare the on-disk head with the exact
   expected old head. Write the proposed bounded checksummed record to a unique
   same-directory temporary file, sync the file, atomically rename it over the head,
   sync the containing directory, and read the head back.
3. Finalize SQLite only after both file and directory sync have succeeded and the
   read-back exactly matches the proposal. Finalization marks the envelope committed,
   advances the acknowledged head, applies visible state and enables eligible outbox
   entries in one transaction.
4. Return the command response only after SQLite finalization commits.

Read-back verifies contents; it does not prove durability and cannot override a
failed sync. If a sync fails, the pending commit remains unacknowledged and its outbox
stays disabled. A later retry may proceed only after a successful file and directory
sync plus matching read-back. Ambiguous rename or read errors are classified by
re-reading, but are not treated as success without those successful barriers.

Startup may resolve only these ordinary states: (a) no pending record and the fully
verified database head matches the durable witness; (b) a verified pending record
extends the current witness head, in which case retry the replacement and finalize;
or (c) a verified pending record's proposed head is visible, in which case sync that
head and its parent directory again under the lock, read it back, then finalize. A
sync failure in (c) keeps the commit pending. All other states fail closed before
command admission or dispatch and never trigger implicit provisioning.

Fail-closed fixtures must cover at least missing and corrupt/oversized witness data;
bad checksum; wrong profile ID, incarnation or coordinator generation; unexpected
epoch; database behind or ahead without matching pending work; malformed pending
old/proposed heads; commit-envelope/hash mismatch; profile identity mismatch; and
epoch exhaustion. Epoch exhaustion is checked before any database or witness
mutation. The initial bootstrap is the only absent-witness case: explicit create may
finish a valid `Initializing` profile with its exact pending bootstrap record; normal
open cannot create it. Once provisioning is active, missing witness state requires
explicit recovery, not `create`.

At the future effect boundary, the broker must validate the current witnessed
generation, grant, and lease immediately before invoking an effect. It holds an
effect-admission guard that serializes this check with local revocation/cancellation
and holds the stable profile witness lock through upstream acceptance, so a new owner
cannot advance the generation between validation and invocation. The guard is
released after acceptance or invocation failure; an already accepted upstream call
may finish later and must be reconciled under DUR-05. A stale generation/lease fails
without invoking the effect. This broker and all effect execution remain disabled in
the current scaffold; pending commits must not make an outbox dispatchable until this
fence exists.

Explicit restore is distinct from normal open and never resets the retained witness.
It verifies the selected snapshot, records its cutoff, current witness epoch, known
gaps and possible later-acknowledgment loss, then writes a pending restore commit
whose predecessor is the current witness head, whose epoch is greater than that
head, and whose incarnation and coordinator generation are fresh. It finalizes via
the same file/SQLite protocol and keeps the profile quarantined. Old commands,
credentials, worker results, restored grants, attempts, ready nodes and outbox
intents remain non-actionable. The head detects a gap but does not contain the lost
commit bodies; independent evidence or upstream reconciliation is required for
potential post-cutoff effects. Effects that cannot be resolved remain unknown and
cannot be replayed automatically. Only explicit confirmation of the loss report and
freshly authorized state can leave quarantine. Missing/corrupt witness state cannot
be bypassed by normal restore; it requires a separately reviewed recovery procedure.

Because the restored snapshot's local commit log ends before the retained witness,
the restore commit carries an explicit `RestoreCheckpoint` anchor. It binds the
verified snapshot-manifest digest and snapshot's last local `(incarnation, epoch,
digest)` to the exact retained witness `(incarnation, epoch, digest)` and the loss
report digest. The restore envelope's predecessor is that retained witness head; its
epoch is the retained epoch plus one. Verification first validates the snapshot chain
through its cutoff, then validates this one explicit gap record against the verified
snapshot manifest and retained head, confirms the new incarnation and quarantine
state, and verifies the new commit chain from the retained witness head. No other
chain gap is accepted. The gap record does not recreate unavailable commit bodies or
claim zero loss; after restore, normal reopen verifies the snapshot prefix, checkpoint,
and new chain without requiring the lost interval's records. A restore-interruption
reopen accepts only the matching pending checkpoint and resolves it through the normal
old-or-proposed-witness recovery rules.

The public seam remains inside `Ledger`: explicit provisioning, open-time
reconciliation, command application, and restore. Fault injection stays private to
the ledger and exercises the real SQLite and file adapters. Tests must assert that no
response or dispatch is possible before finalization, a matching retry returns the
identical saved result before current generation/lease checks, changed intent
conflicts, and every unrecognized state blocks. A response lost after finalization
must replay identically after a new coordinator generation. Restoring only the
profile directory while retaining the witness must not reopen the profile normally.

## Alternatives considered

- **Keep only the profile-local marker:** rejected because coherent rollback of the
  database and marker remains invisible and cannot meet DUR-01.
- **Append-only witness journal:** offers an audit trail but makes bounded growth,
  torn-tail recovery, and safe compaction part of the correctness protocol. The
  versioned SQLite commit log already retains the ledger history, so a single external
  head is the smaller freshness witness.
- **TPM or remote witness:** provides stronger separation from local snapshots but
  adds hardware availability/reset or network dependence and latency. Revisit if the
  product requires whole-machine rollback resistance.

## Required deterministic acceptance evidence

Keep EP1's 10 boundaries × 1,000 seeded crash schedules and four effect classes.
Add witness-specific crash points as subcases; they do not replace any of the 10,000
EP1 cases. Cover SQLite pending commit before/after; temporary-file write and sync;
rename; directory-sync failure with the proposed head still visible; read-back failure
or mismatch; durable witness advance before SQLite finalization; finalization commit;
and finalized response loss. Include generation-only ownership advancement and
restore pending/finalization windows. For each case record fixture/seed, injected
stage, expected and observed database/witness heads, whether a response was returned,
whether dispatch/effect-boundary invocation occurred, retry result, and recovery
classification. Test process crashes separately from power-loss/filesystem-fault
claims.

Also require fixtures for: identical scoped command replay after finalization and
lost response (including a new coordinator generation), changed intent conflict,
hash-chain and materialized-state verification, all fail-closed matrix rows above,
two copied profiles contending for the same per-profile lock, and a stale worker
failing its effect-boundary fence. The restore fixture snapshots before an opaque
effect and grant revocation, then verifies normal open fails, explicit restore creates
a fresh incarnation/generation at an epoch after the retained witness, cutoff/loss is
reported, quarantine persists through recovery, old client commands and worker
results are rejected, and no restored grant or intent becomes actionable.

For every failure fixture assert no acknowledgment or dispatch before the required
durability barriers. If the witness exists visibly but a file or directory sync failed,
recovery stays pending until those barriers later succeed. Keep the existing evaluation
evidence fields and add the commit envelope/head digest and effect-boundary count.
Report witness fsync latency within EP1's existing acknowledgement latency workload;
do not trade the barrier away to meet the target.

The deterministic filesystem denial fixtures also exercise the public open path with
symlinked components, permissive/untrusted ancestors, wrong file types/owners/link
counts, relative roots, roots overlapping/containing the profile, invalid configured
state roots, and unsupported filesystems. Run the wrong-owner case in a disposable
Linux fixture environment that can create such an inode; if that
environment is unavailable, report the denial case as uncompleted rather than passed.
First-bootstrap fixtures crash after each newly created directory/lock/head entry and
verify that parent and containing-directory syncing either permits exact initialization
recovery or normal open fails closed. The golden envelope vector and field/fact
mutations are fixed expected values, not values derived from the implementation under
test.

The restore fixture explicitly snapshots, then completes one opaque effect and revokes
a grant before restoring the older snapshot. It asserts exactly one invocation before
restore and no second invocation after it; independently reconciled completion is
retained, and unresolved completion remains `unknown_effect`. Restore
interruption/reopen fixtures verify the explicit checkpoint bridges the validated
snapshot cutoff to the retained witness without treating the missing commit interval
as verified data. A deterministic cancellation/revocation versus invocation
interleaving covers both orders: revocation winning prevents the invocation; upstream
acceptance winning is recorded for reconciliation and an opaque effect is never
replayed.

## Consequences and limits

Command acknowledgment and all dispatch remain disabled until the commit protocol,
state reconstruction, recovery, and effect fence are implemented and verified.
Disk-full, lock contention, unsupported filesystem, sync failure, ambiguous
replacement, identity/digest mismatch, or epoch exhaustion block progress; there is
no local-only fallback. The guarantee is limited to profile-only rollback while the
per-user witness survives. Process-crash tests and `synchronous=FULL` do not establish
power-loss behavior for every storage device or filesystem. The work remains a
Ticket 02 implementation decision, not production readiness, certification, or
release approval.
