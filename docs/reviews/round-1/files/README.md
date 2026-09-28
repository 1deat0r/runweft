# Runweft

A scaffold for a local-first agent runtime with durable runs, explicit capabilities,
and verification tied to exact artifacts. Working name; availability is not cleared.

**Status: scaffold only.** The CLI can display metadata. The coordinator refuses to
start, the example provider is metadata-only, and the inspector has no backend.
There is no model access, tool execution, persistent ledger or security sandbox yet.

## Start here

- [Implementation specification](spec.md)
- [Project state and next action](STATE.md)
- [Implementation tickets](issues/README.md)
- [Current board review record](docs/reviews/README.md)
- [Explorable architecture](docs/architecture/runweft-architecture.html)
- [Language choices and researched comparisons](docs/research/runweft-design.md)
- [Evaluation protocol](docs/evaluation-protocol.md)

## Development

Requirements: Rust 1.98.1 (pinned), Node 24 or 26, npm, Python 3.12+ and Git.
Node 24 is the initial intended LTS production line; scaffold validation records
the actual locally tested version. Lockfiles pin resolved dependencies. CI is a
recipe until it has run on a configured remote.

```sh
npm ci --ignore-scripts
npm run check
cargo run --locked -p runweft-cli -- status --json
```

For the static development inspector, run `npm run typecheck` once, then `npm run dev`.
It binds to loopback. It cannot control a daemon. `npm run generate` updates the
provisional constant-only status bindings; `npm run check:generated` detects drift.

## Layout

```text
crates/                 Rust protocol seed, authority placeholder, CLI and daemon stub
packages/               TypeScript protocol, adapter SDK and provider metadata example
apps/inspector/         React/Vite static shell, no runtime connection
schemas/                Provisional status source schema
migrations/             Reserved ledger migration boundary; no executable schema yet
evals/                  Offline analysis script; synthetic checks are not benchmarks
tests/ scripts/         Cross-language contract checks and development tooling
issues/                 Eight prospective implementation tickets
docs/                   Architecture, threat model, requirements and review evidence
```

The project is newly authored; no OmO implementation, prompt bundle or credentials
were copied. Apache-2.0 applies to new project code; dependencies retain their own
licenses. See [provenance](docs/provenance.md). No repository remote or publication is configured.
