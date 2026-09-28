# Runweft Context

Runweft separates proposed work, accepted work, and external effects. Protocol messages describe requests and recorded facts; only the coordinator owns durable transitions and effect authority.

## Protocol and execution

**Profile incarnation**: An opaque identifier for one namespace of valid profile commands and leases. Ordinary restarts preserve it; restoring an older backup creates a fresh incarnation.

**Coordinator generation**: A durably allocated, never-reused fencing identifier for one owner of a profile incarnation. It advances on owner replacement and cannot wrap; a stale generation cannot admit commands or effects.

**Graph revision**: An immutable version of a run's task graph, inputs, dependencies, and completion requirements.

**Task**: A unit of requested work in a graph revision. It may have multiple attempts, but its accepted output is tied to an exact artifact and verification receipt.

**Attempt**: One execution of a task with immutable inputs, grant, and model/configuration provenance. Retrying creates a new attempt.

**Effect intent**: A durable description of a proposed external action before invocation. An intent is not proof that the action happened.

**Unknown effect**: An external action whose result cannot be established from trusted evidence. It blocks dependent work and cannot be blindly replayed.

**Critical protocol variant**: A command, response, or durable event whose semantics affect authority or state. A reader that does not recognize its variant rejects the message.

**Protocol extension**: Namespaced optional data carried in the designated extensions map. It is preserved as data and cannot grant authority or change core interpretation.

## Protocol boundaries

Version 1 messages use JSON Schema Draft 2020-12. User/worker/reconciler claims are proposals; only an authenticated coordinator emits durable lifecycle events. Worker results bind to profile, project, run, task, attempt, incarnation, generation, lease ID, and epoch. A claimed actor field never authenticates its sender.

Effect approval binds the exact action, target, artifact digest, grant revision, and policy revision. The durable `invoking` event commits before an external call. Recovery from a committed `invoking` state becomes `unknown_effect`; that intent is never replayed. Reconciliation requires independently verified evidence, and a later action gets a new intent and fresh authorization.

JSON counters and amounts are decimal strings. Extension data excludes JSON numbers so both languages preserve its value exactly. See `docs/protocol-contract.md` for limits, error precedence, command identity, cursor rules, and fixture expectations.
