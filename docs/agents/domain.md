# Domain documentation

Runweft uses one shared project context for its related Rust, TypeScript, and Python
packages. When domain terms or decisions need a durable home, use the root
`CONTEXT.md` glossary and `docs/adr/` for architecture decision records.

These files do not need to exist before they are useful. Create them lazily when a
real domain term or decision is resolved; do not add empty placeholders. Use the
project's established vocabulary, and identify an ADR conflict before proposing a
change that reopens the decision.

Do not use domain documentation as a substitute for the normative requirements in
`spec.md`, `docs/threat-model.md`, or `docs/evaluation-protocol.md`.
