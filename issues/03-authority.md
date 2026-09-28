# 03-authority

Status: Proposed
Spec revision: S2
Type: security. Phase: 1–3. Blocked by: 01; release also requires 02.
What/why: broker and secure worker prevent ambient tool authority.
Method: EP1's 14-class matrix includes compromised Node hosts, direct sockets, inherited environment and credential misuse; parent-to-child restricted context, derived-summary/provider disclosure; path traversal, symlink races, outbound network, malicious MCP content, revoked/stale leases and IPC peer spoofing. Within the disclosure classes include falsely permissive output labels, omitted dependencies, encoded exfiltration via arguments/artifacts/logs, and retained data in reused contexts. Authoritative labels must derive from every exposed input, including mounted data, under SEC-04a; untrusted provenance never lowers them.
Acceptance: explicit expected deny/allow fixtures for every supported platform/mode; 0 unauthorized effects and 100% required fixtures executed.
Rule: DIES on any confirmed authority escape; SURVIVES at 0 escapes with complete suite and external security review; otherwise WEAKENS.
Falsifier/null: instruction filters do not constrain executable authority. Risk: OS/FFI and adapter compromise. Owner: security lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
