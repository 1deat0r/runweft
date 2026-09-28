# Threat model TM1

Normative companion to spec S1. Future enforcement requirements; not implemented
by the scaffold. Assets: credentials, private source/context, user changes, durable
state, approved artifacts, compute/billing budgets and external service authority.

Adversaries include malicious repository text/code, prompt-injecting websites,
compromised npm/MCP extensions, misbehaving models, stale workers, hostile local web
pages and accidental concurrent updates. Trusted computing base: coordinator,
selected OS/VM primitives, storage engine, broker/proxy and explicitly audited
credential-bearing adapters. A compromised local administrator/kernel and approved
provider's internal handling of submitted data are outside the promised boundary.

| Boundary | Enforcement required | Negative/control evidence |
|---|---|---|
| User → coordinator | Authorized command subject; exact artifact/action approval | Spoofed principal and valid principal fixtures |
| Client → local IPC | Peer identity, private socket/pipe ACL, protocol/size limits | Unrelated peer denied; valid client accepted |
| Browser → gateway | Authenticated session, exact origin allowlist, CSRF defense | Hostile page denied; legitimate action succeeds |
| Model/repository → planner | Treat all output as proposals/data | Instructions cannot grant authority or mark acceptance |
| Coordinator → extension | OS containment, package-only read mounts, secret-free env, scoped IPC | Compromised host cannot read home, ledger or management socket |
| Extension/worker → outside | Brokered egress with current grant/lease/destination | Direct socket/DNS bypass denied; allowed call works |
| Secret broker → adapter | Prefer auth injection; explicit audited credential TCB exception | Untrusted extensions never receive reusable tokens |
| Parent → child/context | Recipient read authorization plus inherited disclosure labels | Restricted parent context/summary cannot leak to child/provider |
| Worker → workspace | Approved handles/mounts, race-resistant path checks, quota | Symlink/hardlink/path/mount escape fixtures |
| Worker → evidence | Coordinator checks digest, environment and independent acceptance | Worker success text cannot counterfeit acceptance |
| Old worker → current run | Epoch fencing at actual effect boundary | Stale lease denied after cancel/restart |
| Database → artifact/backup | Durable object publication, snapshot/GC pins, verified restore | Missing object/partial backup blocks acceptance |

Revocation narrows pinned attempt authority immediately at the next read/effect
boundary; a pinned policy revision cannot bypass it. Existing upstream effects may
still finish and must be reconciled. Opaque effects cannot be blindly retried.

Untrusted extension processes receive no arbitrary direct internet access. A future
browser/tool worker may need broader explicit grants, but cannot silently inherit a
provider adapter's secrets or a parent run's context. Grants and payload labels are
separate: permission to call a model is not permission to send every available file.

Secure Linux certification must exercise all EP1 policy classes. macOS/Windows secure
mode initially uses a certified Linux VM/worker if native enforcement is unavailable.
Trusted-local mode cannot be counted as passing secure-mode fixtures. Wasm limits only
its own guest and imports; it does not contain separately spawned native processes.

Residual risks: OS/runtime defects, transformation-label errors, malicious selection
of already permitted data, compromise of explicitly trusted adapters, provider-side
behavior, resource accounting delays and unknown effects. External security review,
fault injection and production monitoring are future requirements. No security
assurance follows from the scaffold's refusal tests or the board's design votes.
