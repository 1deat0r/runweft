# 04-adapters

Status: Proposed
Spec revision: S2
Type: integration. Phase: 2–3. Blocked by: 01–03.
What/why: two providers, MCP and one editor adapter using the same durable contract.
Method: fixture suite for tool schema, streaming, usage, cancellation, retries, unknown capabilities and secret scoping.
Acceptance: 100% advertised features pass; unsupported capabilities reported before dispatch; fallback and context-cache hits preserve disclosure restrictions and recheck the destination.
Rule: DIES if fallback widens data/permission scope; SURVIVES if all advertised cases pass on two providers and one editor; otherwise WEAKENS.
Falsifier/null: shared request syntax is not semantic compatibility. Risk: provider drift. Owner: integrations lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
