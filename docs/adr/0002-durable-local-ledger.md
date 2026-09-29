# Durable local ledger

Status: Accepted for Ticket 02 implementation; release and production use are not
approved. Date: 2026-09-29.

## Context

Ticket 02 requires a local SQLite source of truth with WAL and FULL synchronization,
one bounded serialized writer, transactionally consistent command deduplication,
events, projections, budget reservations and dispatch intents, and acknowledgement
only after commit. The current v1 protocol contract also requires same-command replay
to be resolved before checking a new coordinator generation or lease. The scaffold
has no runtime storage module.

## Decision

Implement a deep `runweft_core::ledger::Ledger` module. Its caller-facing operations
will be `open` and a single high-level `apply` operation that accepts bounded wire
bytes plus peer identity supplied by the authenticated transport. It returns an
accepted/replayed response only after the SQLite commit succeeds. Durable domain
records, SQL, transactions, event sequencing, projections, budget arithmetic and
outbox writes remain private to `runweft-core`; callers cannot assemble partial
write bundles or issue SQL.

The ledger uses `rusqlite` with SQLite bundled for reproducible builds and the online
backup interface. One dedicated writer thread owns one connection and receives work
through a bounded synchronous channel. The connection enables foreign keys, WAL,
`synchronous=FULL`, and bounded busy handling. Read work must not bypass ownership or
mutate durable state. The public module will not expose an in-memory adapter or a
generic storage trait until a second real backend exists.

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
the lock entry or ignores the advisory lock. The future ledger must open its database
and sidecars relative to the retained profile directory descriptor.

Accepted state changes append coordinator-authored events, update projections,
reserve budget, enqueue internal dispatch intent, and save the exact response and
intent projection in one short transaction. Commit failure returns no accepted
response. Outbox persistence alone never invokes an external effect. Unknown or
ambiguous effects remain unreplayed according to S2. Artifact publication, backup
restore quarantine, profile ownership, and generation fencing are part of Ticket 02,
but are implemented and reviewed as distinct internal modules around this ledger.

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
