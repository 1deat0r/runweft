# Runweft implementation specification

Specification revision: **S2** · 2026-09-28 · Board status: **approved, 5/5 BUILD**.

This is the current implementation contract. The approved research proposal in
`docs/research/` supplies rationale and source comparisons; its older approval does
not approve this spec. MUST/MUST NOT describe required future behavior unless a
requirement explicitly says it is implemented by the scaffold.

## 1. Scope, authorization and success

The owner requested a project folder, scaffold, written spec and unanimous expert
review. This turn delivers those artifacts. It does not implement the agent runtime,
execute live model calls, import credentials, publish packages or deploy a service.

Runweft will be a local-first coding-agent runtime whose unit of work is a durable
run: a graph revision, task attempts, capabilities, budget reservations, workspace
snapshots and evidence. User interfaces are clients of one authority. Planning is
probabilistic; admission, authorization and acceptance rules are deterministic.

Initial product target: one user profile on Linux. macOS/Windows clients and secure
workers require separate certification; until then, secure execution uses a certified
Linux worker/VM or refuses. A trusted-local mode may be offered later with explicit
selection and accurate warnings, never as silent secure-mode fallback.

Non-goals for the initial release: active-active coordination, a plugin marketplace,
multi-region scheduling, autonomous permission expansion, training a foundation
model, seamless live OmO-run migration, guaranteed exactly-once external effects,
or a claim that Rust makes model reasoning better.

Success is verified useful work at controlled cost. The preregistered EP1 protocol
in `docs/evaluation-protocol.md` governs comparative claims. OmO source baseline:
`713775cedd533d00c2048ec6dc677f942f3b2ad0` (manifest 5.0.1). OmO already has durable
DAGs, memory, shared cores, isolation and a daemon; those are baseline capabilities,
not novel Runweft claims. All expected improvements remain hypotheses.

## 2. Scaffold inventory and honest behavior

| Present now | Exact behavior | Not implemented |
|---|---|---|
| Rust workspace | Protocol seed, core placeholder, CLI, daemon placeholder | Scheduler, persistence, capabilities and workers |
| `runweft` | Help/version and `status [--json]` | `run`, `resume`, tools, model calls |
| `runweftd` | Help/version only; startup exits 2 | Listening socket, service, IPC |
| TypeScript workspace | Generated status type, metadata-only SDK/provider | Plugin loading, provider inference, secret access |
| React inspector | Static metadata and honest disconnected status | Backend connection, grants, live runs |
| Status schema generator | Required constant-only fields in one seed schema | A general schema compiler or production run API |
| Python EP1 analysis | Offline arithmetic and synthetic self-checks | Dataset, actual benchmarks or performance results |
| CI workflow | Linux recipe for Node 24/26 | Evidence of remote CI success or platform certification |

SCAFFOLD-01: unsupported CLI execution commands and daemon startup MUST exit nonzero
(currently 2) without executing work or opening a server. The status contract MUST
report `phase=scaffold` and `execution_enabled=false` in Rust and TypeScript.

SCAFFOLD-02: build checks MUST detect generated-binding drift, compile/type-check
all packages, compare Rust JSON with the generated TypeScript status, verify disabled
execution, and run the EP1 arithmetic self-check. These checks do not prove runtime
security, durability, usability or superiority.

SCAFFOLD-03: every package MUST be non-publishable until a release ticket explicitly
changes that setting. No live-provider SDK or runtime credential source is included.
Vite's development server is loopback-only and has no daemon control endpoint.

## 3. Languages, modules and dependency direction

| Component | Language and location | Responsibility / rationale |
|---|---|---|
| Protocol | Rust `crates/runweft-protocol`; TS `packages/protocol`; `schemas/` | Versioned data contracts; generated bindings minimize drift. Scaffold schema is deliberately provisional. |
| Authority | Rust `crates/runweft-core` | Own durable transitions, graph compilation, budget, grants and acceptance; typed state and resource ownership. |
| Coordinator | Rust `crates/runweft-daemon` | Eventually host core, serialized SQL writer and broker; lifecycle/OS integration. |
| CLI/TUI | Rust `crates/runweft-cli` | Plain CLI first; clap/Ratatui later under the same client contract. |
| Adapters | TypeScript `packages/adapter-sdk` and provider packages | Rapidly changing model/MCP/ACP integrations; cannot own durable state or permissions. |
| Inspector | TypeScript/React `apps/inspector` | Read projections, submit typed commands, inspect diffs/evidence; no acceptance authority. |
| Storage | SQL `migrations/` with Rust ownership | SQLite local transactions, later alternative PostgreSQL deployment. No executable migration is scaffolded. |
| CodeMode | Restricted JavaScript in an isolated worker | Brokered batched computation; Node/VM language contexts are not security sandboxes. |
| Optional plugins | Rust/Wasm and WIT later | Bounded transforms with explicit imports; no mandatory Wasm dependency now. |
| Evaluations | Python `evals/` | Offline analysis only; not a production runtime dependency. |

DEP-01: dependencies flow from CLI/coordinator toward core, then protocol; protocol
MUST NOT import a harness adapter. TS provider packages depend on the SDK/protocol,
not Rust internals. IPC schemas, not a Rust ABI or shared memory, cross languages.

DEP-02: no provider SDK, SQL library, sandbox backend or UI framework belongs in the
protocol crate. Add runtime dependencies only with the ticket that uses and tests
them. All new Rust code forbids unsafe code in the scaffold; a future OS boundary
exception requires explicit review, inventory and focused tests.

Toolchains: Rust 1.98.1 is pinned for scaffold reproducibility. Node 24 is the initial
intended LTS production line; Node 26 is allowed for development. npm lockfile and
Cargo.lock pin dependency resolutions. Local verification must name the versions
actually exercised. Future patches require compatibility checks; none is assumed
safe merely because it is newer. Python 3.12+ runs the dependency-free EP1 script.

## 4. Process and trust architecture

ARCH-01: one coordinator per user profile is the single durable authority for all
runs in that profile. Core modules are in-process; the design is a modular monolith.
An extension failure affects its scope; a coordinator failure stops new dispatch
until recovery. High availability is not an initial promise.

ARCH-02: three process classes are distinct: trusted Rust coordinator, contained
extension hosts, and contained execution workers. A worker returns proposals and
artifacts, never authoritative permissions or a trusted acceptance decision.

ARCH-03: local IPC uses an owner-only Unix socket in an owner-only directory
(0700 directory, 0600 socket); Windows later uses an ACL-protected named pipe. Peer
identity, protocol negotiation, request size and deadlines MUST be checked. A token
alone is insufficient if an unrelated process can steal the endpoint/credential.
Untrusted workers/extensions must not see the management socket.

ARCH-04: a future browser gateway MUST be loopback-only by default, use an ephemeral
authenticated session, validate exact allowed origins, and defend mutation endpoints
against CSRF. Opening a local webpage must not grant runtime authority. Static scaffold
Vite traffic has no such runtime API and makes no claim of implementing this gateway.

The existing Archify artifact is a logical map. Its boxes do not imply separate
deployments or prove OS trust boundaries. `docs/threat-model.md` defines the latter.

## 5. Domain objects and state transitions

DATA-01: production contracts MUST define Run, GraphRevision, Task, Attempt,
CapabilityGrant, BudgetReservation, EffectIntent, WorkspaceSnapshot, Artifact,
VerificationReceipt and MemoryEntry. Every reference includes profile/project scope.
Events include schema version, run ID, per-run sequence, event/causation IDs, actor,
graph revision, policy revision and observed time. Sequence orders events; wall time
does not order concurrent authority decisions.

Commands include a profile incarnation; worker/broker leases bind both that incarnation
and the current coordinator generation. Ordinary restart preserves the incarnation;
backup rollback creates a new one under DUR-07. Ticket 01 freezes these fields.

DATA-02: identifiers and all amounts cross JSON as explicitly specified strings
where the integer domain exceeds JavaScript's safe range. Money is integer micro-USD
with a pinned price-table revision; no floating-point accumulation in the runtime.
EP1's offline floating-point estimator is a separate analysis interface.

STATE-01: normal task flow is queued → ready → running → verifying → succeeded.
Waiting/exceptional states include awaiting_approval, paused, failed, cancelled and
unknown_effect. An unknown effect blocks dependent work until reconciled. An attempt
has its own immutable input, grant, model/config versions and terminal outcome.
Retry creates a new attempt; historical attempts and receipts remain attributable.

STATE-02: a model proposes a graph, never mutates the live scheduler. The deterministic
compiler MUST reject cycles, missing inputs, unauthorized tools, excessive delegation
or impossible budgets before admission. Amendments create a new graph revision.
Reuse requires matching input/artifact digests and valid receipts; changed inputs
invalidate derived acceptance in the new revision, without rewriting history.

STATE-03: dependencies become ready only after required outputs are accepted by the
coordinator. Worker exit 0, a model's success claim, or a reviewer vote is insufficient.
Graph completion requires every required output accepted; failure/skip propagation
is explicit in the graph's completion policy. Optional skipped nodes cannot hide a
missing required artifact. Ticket 01 must freeze the transition table and error union
before execution features are introduced.

## 6. Durable transactions and effect recovery

DUR-01: SQLite in WAL mode with FULL synchronization is the local source of truth.
Use one bounded serialized writer, short transactions, foreign keys and unique
command keys. Do not use a network filesystem. Transactionally validate the expected
revision, append an event, update projections, reserve budget and enqueue dispatch
intent. Only acknowledge accepted durable commands after commit.

DUR-02: clients supply a scoped command ID. Repeating the same ID and canonical
payload returns the same recorded outcome; reusing it with a different payload is
a conflict. Retain deduplication data for at least the run's replay/retry lifetime;
commands for retired runs are rejected, not newly executed. Stream consumers resume
by durable sequence; live tokens/telemetry may be coalesced separately.

DUR-03: artifact publication writes a temporary object, flushes, renames atomically,
syncs the directory and only then commits its digest reference in SQL. Orphans can
be collected after a grace period; missing referenced objects block acceptance.
Backups pin the exact database snapshot and reachable object set under a GC lease,
verify hashes, then publish a manifest. Restore verifies before accepting commands.

DUR-04: dispatch is at-least-once intent handling, never an exactly-once promise to
arbitrary external systems. Effect classes are read-only, remote-idempotent,
externally reconcilable and opaque/non-idempotent. Recovery retries safe reads,
reuses valid idempotency keys, queries reconcilable IDs, and marks ambiguous opaque
effects unknown_effect. A destructive action MUST NOT be replayed merely because
its result was not recorded. Idempotency-key expiry is a reconciliation condition.

DUR-05: every attempt has a lease epoch/fencing token. The actual effect boundary
checks current authority immediately before invocation. Cancel prevents future
admissions, revokes broker leases, and attempts termination. It cannot guarantee
undoing an upstream call already accepted; such calls need reconciliation. Resume
reconstructs state from recorded facts and does not regenerate past model output.

DUR-06: durable event readers and migrations are versioned. Unknown critical event
variants halt recovery safely. Backups, restore tests and rollback compatibility are
required before schema updates. Replaying recorded responses is reconstruction;
calling a live model again is a new attempt with new provenance.

DUR-07: restoring a backup is distinct from ordinary crash recovery. Restore starts
quarantined in a fresh, unpredictable profile incarnation and coordinator generation;
it invalidates all previous client sessions and worker/broker credentials. The broker
MUST reject old generations at each effect boundary. Restored grants, ready nodes,
dispatch intents and attempts MUST NOT automatically become actionable. Previously
accepted upstream calls may still complete and require reconciliation under DUR-05.

Restored records are historical evidence with a declared snapshot cutoff. Reconcile
each pending or potentially post-snapshot effect with independently retained evidence
or the upstream service. Unresolved opaque effects remain unknown_effect and cannot
be automatically replayed. Explicit user authorization may create a separately
recorded new action after disclosure of the duplicate-effect risk; it must not relabel
the old effect as known. Only freshly authorized, currently validated grants and
reconciled inputs may admit a new attempt in the restored profile.

Deduplication guarantees apply within an incarnation and do not promise that a backup
contains later acknowledgements. Reject commands for the previous incarnation,
including client retries, without executing them. Clients MUST NOT automatically
rewrite stale commands into the new namespace. A restore report names the cutoff,
known gaps and possible loss of later acknowledgements; explicit operator confirmation
is required before leaving quarantine. Deliberate backup rollback cannot be presented
as zero-loss recovery. Normal crash recovery still owes DUR-01–02's acknowledged-state
guarantee. Ticket 02 must test old-backup restore after an opaque effect and revocation,
including duplicate client requests and stale-worker submissions.

DUR-08: acquire exclusive OS-enforced profile ownership before dispatch and retain
it through shutdown. Each ownership replacement creates a fresh coordinator generation;
every broker/worker dispatch checks it. SQLite writer serialization alone is not the
ownership mechanism. Ticket 02 must prove a second coordinator cannot dispatch and
stale credentials remain fenced after replacement and restore.

## 7. Authority, containment and data disclosure

SEC-01: effective grants are the intersection of administrator constraints, user
authorization and narrower task scope. Children cannot widen them. Project files,
skills, prompts and tool annotations are untrusted data, never permission sources.
Grant fields include subject, roots, tool/action IDs, network destinations, secret
handles, delegation limits, cost limits and expiry.

SEC-02: secure mode MUST contain every untrusted extension and worker at the OS/VM
boundary. Extension hosts get read-only verified package/runtime mounts, quota-bound
private temp storage, an allowlisted secret-free environment and only scoped broker
IPC. No ambient home, ledger, credential store, management socket or unrestricted
network access. Workers receive only explicitly scoped workspace mounts. A worktree
is isolation for edits, not a security sandbox. Linux certification includes namespace,
mount, syscall, egress and resource enforcement; unavailable enforcement means refusal.

SEC-03: prefer broker-side credential injection. Credential-bearing provider adapters
are a separately audited part of the trusted computing base, isolated from unrelated
data with narrowly constrained egress and least-privileged credentials. The broker
checks leases before requests and terminates/revokes on cancellation where possible.
Do not claim to revoke a long-lived upstream credential or in-flight effect when the
provider cannot enforce that promise. Untrusted adapters never receive reusable secrets.

SEC-04: every artifact/context/memory/cache read checks the receiving subject's
authorization. Immediately before provider submission, check the complete payload's
scope and destination. Derived summaries inherit the union of source restrictions
(intersection of allowed recipients). Delegation, retry, fallback, caching and export
preserve these restrictions. Models cannot declassify data. Only an authorized user
decision can broaden disclosure. Cache hits reauthorize; parent transcripts are not
copied wholesale into children. Missing labels fail closed.

SEC-04a: the coordinator owns classification and provenance. An untrusted producer
MUST NOT lower labels or declare an authoritative dependency set. Each execution
context accumulates a monotone union of restrictions from every input exposed to it:
messages, tool results, mounted files, environment and retained process/cache state.
Where individual filesystem reads cannot be mediated, include all data reachable
through the granted mounts, even if the producer claims not to have read it. Context
reuse inherits this accumulated classification; a fresh lower-classified context
requires an isolated clean process/state boundary, not a model's reset assertion.

All outputs inherit that context classification, including artifacts, code, tool
arguments, logs, summaries and encoded/transformed content. Every outbound effect
checks both these restrictions and the complete payload/destination. Untrusted labels
may only narrow recipients further. The initial implementation provides no automatic
declassification; only an authorized user's recorded decision bound to the exact
artifact/action and recipients can broaden disclosure. Any later trusted label-reduction
mechanism requires a separate reviewed contract. Tickets 03/05 require falsely labeled
artifacts, omitted dependencies, encoded exfiltration and reused-context fixtures.

SEC-05: existing attempts pin model/config/prompt revisions, but current deny rules,
grant expiry and revocation take precedence at every effect/read boundary. Hot reload
cannot broaden an active attempt's permissions. Broader grants require explicit new
authorization and a recorded revision; revocation must not wait for a new attempt.

SEC-06: material external actions use approval bound to action, target, artifact
digest and grant/policy revision. Any material change invalidates that approval.
Routine allowed operations proceed within preapproved scope. Sanitize untrusted
terminal controls and rendered HTML. Telemetry omits sensitive payloads by default.

SEC-07: JSON-RPC/MCP provides interchange, not authorization. Set size, nesting,
output, deadline and resource limits before enabling a transport. OS administrators,
approved destination compromise, kernel/FFI defects and credential-bearing adapters
remain residual risks. No prompt filter or finite test suite proves complete safety.

## 8. Scheduling, model routing and budgets

RUN-01: one-agent execution is first-class. A bounded ready frontier uses critical-path
priority with aging/fairness and explicit global/provider/workspace quotas. Independent
reads may batch; writers sharing mutable state require isolation or serialization.
Delegation has depth, descendant, time, attempt and spending limits.

RUN-02: reserve a conservative per-invocation cost bound before dispatch; settle actual
usage afterwards. Unknown billing retains the reservation until reconciled. Prevent
parallel overspend with transactional reservation. A hard billed-spend guarantee is
only advertised for providers with enforceable request bounds; otherwise show an
estimated ceiling and bound concurrency/exposure explicitly. Do not fabricate free
usage when metering is absent. The first provider integration must document this.

RUN-03: routing applies disclosure, required capability, context and budget constraints
before ranking on evaluated quality/cost/latency. Provider capabilities are observed,
versioned and fixture-tested. An OpenAI-compatible request shape is not semantic
equivalence. Fallback cannot widen disclosure or permissions. Role profiles are
provider-neutral. Retry/backoff and circuit breakers are finite and recorded.

## 9. Workspaces, context and acceptance

WORK-01: capture the starting revision, approved dirty overlay and relevant untracked
files. Never overwrite the user's working checkout. Independent writers use separate
snapshots/checkouts. Explicitly handle submodules, symlinks, ignored files and large
assets. Edits check content hashes or document versions; integration uses base-aware
merge and reports conflicts without discarding user changes.

WORK-02: verification executes relevant tests, compilation/static checks, independent
acceptance checks and real-surface tests where the change requires them. Distinguish
agent-editable repository tests from independent acceptance fixtures. Receipts bind
artifact digest, input/environment/tool revisions, commands, outcomes and omissions.
If integration changes the tested artifact, rerun the affected acceptance checks.
Neither modified tests nor consensus alone can promote an artifact to accepted.

CTX-01: separate transient context, project knowledge and opt-in preferences. Entries
record source scope/digest, observation time, attribution and invalidation rules.
Re-read changed sources; preserve uncertainties in summaries. Memory cannot grant
authority. Start with lexical/path/symbol retrieval; embeddings require demonstrated
benefit. Imports are attributed history, not new instructions. Deletion propagates
to controlled indexes/caches; backups have declared retention. Sensitive payloads
must be redactable independently of minimal append-only event metadata.

UX-01: UI distinguishes produced, checked, accepted and published; shows cost estimates
versus actuals, unresolved effects, pending grants and evidence omissions. Pause/cancel
semantics must be honest. The inspector renders projections and cannot manufacture
accepted state. Accessibility and interaction testing are separate future gates.

## 10. Integration, migration and 2027 boundaries

COMPAT-01: internal APIs, durable events, provider profiles and plugin manifests have
independent versions and compatibility fixtures. Production schema generation and
critical-variant handling must be chosen in ticket 01. MCP/LSP/ACP are boundary adapters,
not the internal authorization or scheduler model. Optional fields can be preserved;
unknown execution-critical variants fail closed.

COMPAT-02: initial OmO import supports only fixture-backed formats from the pinned
baseline. Dry-run preferences, user-owned skills, MCP endpoints and attributed memory
into a new profile. Historical transcripts are artifacts. Reauthorize credentials;
do not import OAuth tokens, active sessions/DAGs/effects, executable hooks or broad
grants. Keep the original install/state byte-identical; canary in separate checkouts.

FUTURE-01: remote workers require a new gate: mutually authenticated transport,
short-lived workload authority, scope enforcement at the worker, fencing and partition
reconciliation. One coordinator remains authoritative per run. PostgreSQL/object
storage is an alternative deployment backend, not a SQLite dual-write authority.
Optional Wasm, local models and new modalities preserve the same grant/effect/evidence
contracts. No 2027 model or protocol feature is assumed to exist.

RELEASE-01: new code is Apache-2.0, with dependency provenance and component license
review before distribution. OmO's restricted root license is not a permissive source
grant. Future binaries/runtime bundles need signatures, SBOMs, verified installation,
draining/checkpointing during upgrades and tested restore paths. No auto-patching other
harnesses. No remote repository or publication is part of this scaffold request.

## 11. Delivery phases, tickets and decision gates

| Phase | Ticket | Dependency / acceptance intent |
|---|---|---|
| Scaffold | current task | SCAFFOLD-01–03; current board unanimous; recorded local checks |
| 0 | [01 contracts](issues/01-contracts.md) | Current spec approval, then owner-authorized implementation; 50 protocol fixtures |
| 1 | [02 durability](issues/02-durable-execution.md) | 01; EP1 crash matrix and artifacts/backup recovery |
| 1–3 | [03 authority](issues/03-authority.md) | 01; release also needs 02 and certified containment |
| 2–3 | [04 adapters](issues/04-adapters.md) | 01–03; two providers, MCP and one editor |
| 2 | [05 context/edits](issues/05-context-and-edits.md) | 02–04; attribution, conflict handling and receipt validity |
| 0 baseline / 2 decision | [06 evaluation](issues/06-evaluation.md) | Frozen baseline; successor comparison after 02–05 |
| 3 | [07 distribution](issues/07-distribution.md) | 02–06; upgrade, rollback, import/provenance |
| 4 optional | [08 remote](issues/08-remote-workers.md) | Local release + renewed review; partition/fencing suite |

EVAL-01: EP1 is normative for dataset selection, paired repository bootstrap, complete
cost accounting, operational workloads and decision-rule totality. The code lives at
`evals/analyze_evaluation.py`. Concrete manifests, acceptance criteria,
provider controls and script digests MUST be frozen before real collection.

Utility decision order: unresolved confirmed authority escape or acknowledged-state
loss → DIES. Invalid cohort, missing metering or undefined ratio → WEAKENS. Otherwise
SURVIVES only if the lower acceptance-difference interval exceeds −0.02 and either
the upper cost-per-acceptance ratio is below 0.85, or the lower acceptance difference
exceeds +0.05 while the upper cost ratio is below 1.10. All other cases → WEAKENS.
No superiority claim on missing/inconclusive evidence. This is a product decision
rule with approximate bootstrap uncertainty, not proof of general superiority.

Operational gates: EP1's 10,000 crash schedules; zero unauthorized effects in the
complete policy inventory; specified p95 local command latency below 150 ms at 32
active synthetic attempts; independent security review. Failures/missing evidence
block the associated release claims. Scaffold checks cannot substitute for them.

## 12. Normative decisions and review records

Review effort follows consequence and uncertainty. Routine implementation changes
use the local verification and diff-review workflow in `CONTRIBUTING.md`. Normative
specification and architecture changes use a frozen revision and independent
reviewers selected for the decisions they affect. Include adversarial security for
authority, trust-boundary, or security-policy changes; systems/durability for
protocol, state, persistence, or recovery; implementation for feasibility and
integration; evaluation for claims and acceptance evidence; and product/scope for
user-facing requirements and architecture. Use all five perspectives when the
change crosses those risks, not simply because a particular path changed.

Keep reviewers' first-pass reports independent when practical. Findings state the
exact artifact, impact, and testable closure condition. Recheck material closures
against the final revision and record unresolved dissent with its evidence. Review
is not authorization to expand scope, implement a proposed ticket, publish, or
release. The owner authorizes the task and remains responsible for high-impact
external actions.

A BUILD decision on this specification or scaffold means it is suitable to
implement and test; it does not mean that the unbuilt runtime has passed its future
gates. No runtime ticket is marked implemented by a vote. Round manifests bind the
reviewed specification, threat model, tickets, schemas, and scaffold source;
lockfile and build evidence is recorded separately.

Board record: S1 received two BUILD and two CONDITIONAL votes; the fifth seat was
reserved for a fresh final read. S2 received **5 BUILD / 0 CONDITIONAL / 0 REJECT**.
Systems, security, build, evaluation and the product cold reader all approved the
same frozen revision. See `docs/reviews/README.md` and the immutable round manifests.

Adjudication: `docs/reviews/adjudication.md` records each ruling. SYS-1 and SEC-1 were
confirmed, fixed and quote-verified PASS. S2 adds quarantined restoration and
incarnation/fencing contracts, coordinator-owned output classification, corresponding
ticket fixtures and current-document evaluation references. No scaffold code or EP1
decision rule changed. Remaining notes are nonblocking and recorded with limitations.

Only record/status metadata changed after final votes; normative sections 1–11 are
byte-identical to reviewed S2. `docs/reviews/handoff-integrity.json` binds the reviewed
and delivered hashes. Further normative changes require another revision and review.
