# Runweft

A scaffold for a local-first agent runtime with durable runs, explicit capabilities,
and verification tied to exact artifacts. Working name; availability is not cleared.

**Status: scaffold only.** The CLI can display metadata. The coordinator refuses to
start, the example provider is metadata-only, and the inspector has no backend.
There is no model access, tool execution, persistent ledger or security sandbox yet.
The v1 protocol schema, bindings, and cross-language validators are implemented for
contract testing; they do not provide a coordinator, durable state, production IPC,
or effect execution.

## Start here

- [Implementation specification](spec.md)
- [Project state and next action](STATE.md)
- [Implementation tickets](issues/README.md)
- [Current board review record](docs/reviews/README.md)
- [Local-first contributor workflow](CONTRIBUTING.md)
- [Explorable architecture](docs/architecture/runweft-architecture.html)
- [Language choices and researched comparisons](docs/research/runweft-design.md)
- [Evaluation protocol](docs/evaluation-protocol.md)

## Development

Requirements: Rust 1.98.1 (pinned), Node 24 or 26, npm, Python 3.12+ and Git.
The lockfiles pin resolved dependencies. `npm run check` is the canonical local
`VERIFY` command. It checks generated-contract drift, Rust format/lint/tests,
TypeScript build/tests, Python helper tests, and the offline evaluation self-check.

The normal loop is local: inspect the task and Git state, implement, run
`npm run check`, review the complete diff, and make an atomic commit. GitHub Issues,
branches, and pull requests are optional when they add durable tracking, useful
isolation, independent review, or coordination. GitHub Actions runs the clean
Node 24/26 matrix on pushes and pull requests as a compatibility and reproducibility
safety net; it is not the primary development loop. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the full workflow and risk-based review
practice.

For a fresh checkout or changed dependencies, run:

```sh
npm ci --ignore-scripts
npm run check
```

To inspect scaffold CLI metadata, run:

```sh
cargo run --locked -p runweft-cli -- status --json
```

For the static development inspector, run `npm run typecheck` once, then `npm run dev`.
It binds to loopback. It cannot control a daemon. `npm run generate` updates scaffold
status and protocol bindings, metadata, transition allow-lists, and generated docs;
`npm run check:generated` verifies that all outputs match their sources. Run
`npm run build && node scripts/measure-ipc.mjs` for the manual local socket framing
and validation probe. Its results are recorded in
[the IPC measurement note](docs/protocol-ipc-measurement.md). Protocol validation
and test fakes do not enable CLI or daemon execution.
The working name has not been cleared for trademark or package use.

## Layout

```text
crates/                 Rust protocol contracts, authority placeholder, CLI and daemon stub
packages/               TypeScript protocol contracts, adapter SDK and provider metadata example
apps/inspector/         React/Vite static shell, no runtime connection
schemas/                Status and versioned protocol sources and policies
migrations/             Reserved ledger migration boundary; no executable schema yet
evals/                  Offline analysis script; synthetic checks are not benchmarks
tests/ scripts/         Cross-language contract checks and development tooling
issues/                 Eight prospective implementation tickets
docs/                   Architecture, threat model, requirements and review evidence
```

The project is newly authored; no OmO implementation, prompt bundle or credentials
were copied. Apache-2.0 applies to new project code; dependencies retain their own
licenses. See [provenance](docs/provenance.md). The public source repository is
[github.com/1deat0r/runweft](https://github.com/1deat0r/runweft).
