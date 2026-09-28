# Scaffold verification — 28 September 2026

Executed locally on Linux at the new project root, before specification round 1.
Toolchain: rustc/cargo 1.98.1, Node 26.8.1, npm 11.19.0, Python 3.14.4.

`npm install --ignore-scripts --no-audit --no-fund` completed and created the npm lockfile.
`cargo generate-lockfile` created Cargo.lock. `npm run check` then exited **0**:

| Check | Observed result |
|---|---|
| Schema generation drift | Current; Rust and TS bindings match the source generator |
| Rust formatting | Passed |
| Clippy, all workspace targets, warnings denied | Passed |
| Cargo test | All crate/doc-test targets passed; **zero Rust unit tests** exist in this scaffold |
| TypeScript project build | Passed |
| Vite production bundle | Passed; 16 modules transformed |
| Cross-language status integration test | Passed; exact Rust JSON / TypeScript constant equality |
| Disabled-execution integration test | Passed; CLI run and daemon startup exit 2; metadata provider cannot execute |
| Python EP1 self-test | Passed; synthetic arithmetic/gate checks only |

There were **two Node integration tests**, both passed. No live model/provider calls,
credentials, persistent runtime database, production sandbox or external publication
was exercised. No daemon or preview server was left running. Browser/UI interactions,
Node 24, macOS, Windows and remote CI were **not tested**. The static inspector is
build-verified only. Historical Archify receipts do not test this React application.

Initial authoring uncovered a newline-escaping error in the generator; it was corrected
before the passing run and before review snapshots. No failed run is represented as a pass.
