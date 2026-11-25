<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-25

## Review Summary

ADR review completed for this change. Of design.md's decisions, decision 2
(core records Operator output as a derived Asset, with an `origin` column and
an `asset_lineage` table), together with decision 1 (the Operator declares
previews and streams the bytes through its Asset context), is a costly fork
that the code alone would not explain. It settles the question ADR-003 left
open for the Operator contract, sets a table shape that later outputs such
as stacks will reuse, and rules out renditions with no Asset. It gets a new
record, which builds on ADR-003. Decision 3 (queries cover originals only)
follows from that record. Decisions 4 to 7 (the TIFF directory reader, the
decoders and encoder, one preview at a time, and the web fallback) can be
changed inside one story and need no record.

## In-Force ADRs Reviewed

- [ADR-001: Plugin Contract and Execution Profiles](../../../docs/decisions/ADR-001-plugin-boundary.md) — the preview Operator is a built-in Rust Operator that reaches its input and output only through the Asset context.
- [ADR-002: Core / Domain-Pack Seam](../../../docs/decisions/ADR-002-core-domain-pack-split.md) — the preview Operator and TIFF reading live in `packs/image`, which still depends on `plugin-abi` only.
- [ADR-003: Asset Identity and Content Revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md) — the preview is a separate Asset, and lineage links versions.
- [ADR-004: Database Engine and Schema Strategy](../../../docs/decisions/ADR-004-database-engine-and-schema.md) — migration 7 is forward-only.
- [ADR-005: v2 Visual Design System](../../../docs/decisions/ADR-005-visual-design-system.md) — the web change adds no token.
- [ADR-006: Declarative Processing, Selectors, and Policy Deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md) — upload still runs a fixed list of Operators; no policy decides it.
- [ADR-007: Security and Plugin Trust Model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md) — the Operator gets no store path; core checks and publishes its output.
- [ADR-008: Metadata Envelope, Facets, and Write Authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md) — making previews is declared at registration, as facet grants are.
- [ADR-009: Backend Language and Runtime](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010: Migration Strategy — Clean Break with a One-Way Importer](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011: Storage Tree Layout and Cross-Filesystem Moves](../../../docs/decisions/ADR-011-storage-tree-layout.md) — a preview's bytes are staged, hashed and published before any record, like an upload's.
- [ADR-014: Managed Storage Identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)
- [ADR-015: Opaque Database-Backed Sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md) — the preview route sits behind the same session guard.
- [ADR-016: Development Environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md)
- [ADR-017: Facet Values as Rows with a JSONB Value](../../../docs/decisions/ADR-017-facet-value-rows.md)
- [ADR-018: Choice Facets Keep Their Allowed Values with the Schema Version](../../../docs/decisions/ADR-018-choice-facets-keep-allowed-values.md) — adding `TIFF` to `image.format` needs schema version 2.

ADR-012 (proposed) and ADR-013 (superseded by ADR-016) were read but are not
in force.

## New Durable ADRs Created

- [ADR-019: Operator Output Becomes a Derived Asset Linked by Lineage](../../../docs/decisions/ADR-019-operator-output-as-derived-assets.md) — design decisions 1 and 2.
