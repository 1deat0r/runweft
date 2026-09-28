# 07-distribution

Status: Proposed
Spec revision: S1
Type: release/migration. Phase: 3. Blocked by: 02–06.
What/why: signed install, schema migration, import, retention and safe upgrade.
Method: version-pair restore tests, config import dry-run, credential reauthorization, snapshot/manifest verification, stale extension rejection.
Acceptance: 100% of supported upgrade/restore fixtures pass; importer leaves original OmO files byte-identical; every shipped component has recorded license/provenance.
Rule: DIES on irreversible undocumented data loss or unapproved authority import; SURVIVES with complete acceptance evidence; otherwise WEAKENS.
Falsifier/null: packaging can erase the advantages of the core. Risk: platform supply chain and migrations. Owner: release lead.

This ticket is future work. Scaffold checks do not satisfy its implementation acceptance.
