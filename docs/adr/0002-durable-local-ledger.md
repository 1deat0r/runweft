# Durable local ledger

Status: Accepted for Ticket 02 implementation; release and production use are not
approved. Date: 2026-09-29.

## Context

Ticket 02 requires a local SQLite source of truth with WAL and FULL synchronization,
one bounded serialized writer, transactionally consistent command deduplication,
events, projections, budget reservations and dispatch intents, and acknowledgement
only after commit. The current v1 protocol contract also requires same-command replay
to be resolved before checking a new coordinator generation or lease. The repository
had no runtime storage module when Ticket 02 began.

## Decision

Implement a deep `runweft_core::ledger::Ledger` module. Its caller-facing operations
will be `open` and a single high-level `apply` operation that accepts bounded wire
bytes plus peer identity supplied by the authenticated transport. It returns an
accepted/replayed response only after the SQLite commit succeeds. Durable domain
records, SQL, transactions, event sequencing, projections, budget arithmetic and
outbox writes remain private to `runweft-core`; callers cannot assemble partial
write bundles or issue SQL.

The ledger uses `rusqlite` with SQLite bundled for reproducible builds and the online
backup interface. The intended writer owns one connection and receives work through a
bounded synchronous channel. The connection enables foreign keys, WAL,
`synchronous=FULL`, and bounded busy handling. Read work must not bypass ownership or
mutate durable state. The public module will not expose an in-memory adapter or a
generic storage trait until a second real backend exists. In this foundation slice,
the bounded queue and writer lifecycle are initialized, but the loop handles shutdown
only; no transaction request, apply, or backpressure path exists yet.

Command processing preserves the v1 documented order: bounded structural and
schema validation; authenticated peer/role and profile/project/incarnation scope;
dedup lookup by `(profile_id, project_id, profile_incarnation, command_id)`; current
generation and lease checks; policy/lifecycle/revision/budget checks; then commit.
The stored intent uses `runweft_protocol::command_intent_projection`, including
extensions and excluding command ID and coordinator generation. A matching command
returns its original stored outcome without writes; a changed intent conflicts.
Validation is split so stale mutable authority cannot prevent a valid duplicate from
replaying. The v1 wire shape remains frozen; revisions are coordinator-owned durable
guards, not new client-supplied fields. A future wire-level revision requirement
would need a separately reviewed protocol decision.

Before changing journal mode on an existing database, the opener checks the
application and schema version markers, required v1 table names and ordered column
names, STRICT/WITHOUT ROWID modes, primary and unique keys, required foreign keys and
check expressions, singleton state, profile identity, canonical generation, restore
quarantine, the durable profile marker, and absence of extra user schema objects. It
does not run a full physical integrity scan or prove every semantic invariant of
stored data.

Profile ownership is Linux-only for the initial Linux product target. Resolve each
profile path component from an opened root directory using `openat` with directory
and no-follow flags; reject untrusted writable ancestors and require the final
profile directory to be private and owned by the effective user. Open
`coordinator.lock` relative to that pinned directory handle without following
symlinks, validate that it is a private single-link regular file owned by the
effective user, and hold both descriptors for the coordinator lifetime. Do not
re-resolve the profile path after acquiring ownership. Other platforms fail closed
until they have an equivalent reviewed implementation. This lock coordinates trusted
same-user processes; it is not a sandbox against a same-user process that replaces
the lock entry or ignores the advisory lock. Preflight the main database and existing
WAL/SHM/journal entries with no-follow opens relative to the retained directory
descriptor, checking owner, link count, file type, filesystem type, and device.
`Connection::open` receives `/proc/self/fd/<dirfd>/ledger.sqlite`, after which SQLite's
VFS opens the database and sidecars by pathname. The private directory prevents other
UIDs from replacing those entries; a same-UID path race remains outside the trust
boundary. This is not fully descriptor-based SQLite I/O. Removing that limit needs a
reviewed SQLite VFS or equivalent fd-native open path. The initial Linux build accepts
only known persistent local filesystems (ext2/3/4, btrfs, XFS, F2FS, and bcachefs); it
rejects tmpfs, network filesystems, and unknown types before use.

The locked `coordinator.lock` inode also stores two alternating checksummed marker
slots for profile identity, bootstrap state, and last issued coordinator generation.
`Ledger::create` is an explicit provisioning operation; normal `Ledger::open` never
falls back to creation. The initializing marker is synced before creating a new
database. Only explicit `Ledger::create` may create a database from a valid initializing
marker; normal open returns `ProfileNotInitialized` without creating it. After the
initial SQLite transaction commits, normal open can finish active-marker promotion
from the database's strictly newer generation. Nonempty corrupt or torn marker bytes
fail closed for both open and create. A short write during first provisioning can
leave no database, while truncating an initialized profile can leave the same marker
shape after database loss; the local files cannot prove which history occurred. A
missing or empty marker and missing database are treated as uninitialized only by
explicit create; callers must not use create to recover an existing profile. A restore
path must use a fresh incarnation rather than silently reusing the old one.
The next marker sequence is checked before SQLite generation changes. The active
marker is synced after the SQLite generation transaction and before open returns.
Normal open with a marked profile but missing database, mismatched identity, or a
database generation below the marker fails closed. A damaged active slot is accepted
only when the database proves a strictly newer generation than the surviving marker;
this prevents a rolled-back database from reusing a generation. Normal open rejects
profiles marked quarantined.

This local marker is not an independent freshness witness. A coherent old snapshot of
both `ledger.sqlite` and `coordinator.lock` can still pass normal open, as can a
snapshot that omits acknowledged changes within one coordinator generation. Losing
both files causes normal open to fail as uninitialized, but explicit provisioning can
create a profile again if its caller chooses to do so. Therefore this foundation does
not satisfy DUR-07 restore safety or prove DUR-01 acknowledged-state durability.
Before dispatch or command acceptance is enabled, restore must be an explicit path
that assigns a fresh unpredictable incarnation, quarantines the restored state, and
uses a freshness witness outside the profile backup/rollback domain. The witness must
advance at each ack-eligible commit; a coordinator-open-only generation witness is
insufficient. This API and recoverable witness protocol are not implemented yet.

The schema reserves tables for events, projections, scoped command results, budget
reservations, and an outbox. Atomic command application, sequence allocation,
deduplication, artifacts, backup restore quarantine, and generation-checked dispatch
are not implemented yet. The outbox currently cannot invoke an external effect, and
the dedicated bounded writer currently handles shutdown only. These are Ticket 02
work items, not capabilities of this foundation.

## Alternatives considered

- A caller-owned typed commit bundle makes each caller responsible for assembling
  coherent event/projection/budget/outbox updates. It is flexible but weakens the
  module's leverage and spreads authority invariants.
- Separate public methods for event append, projection update, budget reserve, and
  outbox enqueue permit partial commits and are rejected.
- A public repository trait or fake in-memory adapter is not justified while SQLite
  is the sole production adapter; tests will use temporary on-disk SQLite profiles.
- A direct mutex around a connection serializes writes but has no explicit bounded
  admission queue or dedicated writer lifecycle; use the bounded writer thread.
- Adding expected revision fields to protocol v1 is deferred. The current protocol
  identity projection and schema are frozen; coordinator-owned revisions will guard
  changes inside the durable transaction.

## Consequences and limits

The ledger is a high-consequence module. Tests must exercise its public interface
against on-disk SQLite and inject failure at transaction/acknowledgement boundaries.
The serialized queue limits concurrency by design and must expose backpressure rather
than accumulate unbounded work. WAL is for same-host local filesystems; network
filesystem profiles must be refused or otherwise proven unsupported before use.
SQLite FULL synchronization and process-crash fixtures do not alone establish power-
loss behavior for every filesystem. No scheduler, live provider, broker effect path,
secure sandbox, production daemon, release, or certification follows from this ADR.
