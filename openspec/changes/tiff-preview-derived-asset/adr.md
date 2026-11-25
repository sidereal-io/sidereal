<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-25

## Review Summary

ADR review completed for this change. Three decisions have durable consequences
for the plugin contract, persistence or execution boundary.

- Decision 4 (core records Operator output as a derived Asset, with an
  `origin` column and an `asset_lineage` table), together with decision 1
  (the Operator declares previews and streams the bytes through its Asset
  context), settles the question ADR-003 left open for the Operator
  contract. It sets a table shape that later outputs such as stacks will
  reuse, and rules out renditions with no Asset. Its record builds on
  ADR-003.
- Decisions 2 and 3 (an Operator declares the media types it accepts, its
  context gives it the version's media type, and core runs it only on an
  original's version it accepts) change the public plugin contract and
  decide which versions every Operator runs on, previews included. It is
  the first step towards the `accepts` Selector, so its record builds on
  ADR-006.

- Decision 8 places the preview decoder in a short-lived helper so its
  memory limit and failure do not apply to the server. It keeps the
  built-in Operator interface from ADR-001.

Decision 5 (queries cover originals only) follows from the first record.
The format layout, directory reader, codec choices, upload ordering,
refusal message and web behavior can change within the boundaries above.

The first plan of this change numbered its derived-asset record ADR-019,
after #547's ADR-018 for a choice value type. #547 deleted that record
before it reached `main`, so the derived-asset record takes the free number,
ADR-018, and the new contract record is ADR-019. No document outside this
change linked to the old number.

## In-Force ADRs Reviewed

- [ADR-001: Plugin Contract and Execution Profiles](../../../docs/decisions/ADR-001-plugin-boundary.md) — the preview Operator is a built-in Rust Operator that reaches its input and output only through the Asset context.
- [ADR-002: Core / Domain-Pack Seam](../../../docs/decisions/ADR-002-core-domain-pack-split.md) — the format modules, TIFF reading and the preview Operator live in `packs/image`, which still depends on `plugin-abi` only; the `ImageFormat` trait stays inside the pack.
- [ADR-003: Asset Identity and Content Revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md) — the preview is a separate Asset, and lineage links versions.
- [ADR-004: Database Engine and Schema Strategy](../../../docs/decisions/ADR-004-database-engine-and-schema.md) — migration 6 is forward-only.
- [ADR-005: v2 Visual Design System](../../../docs/decisions/ADR-005-visual-design-system.md) — the web change adds no token.
- [ADR-006: Declarative Processing, Selectors, and Policy Deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md) — upload still runs a fixed list of Operators; an Operator's accepted media types are a first, partial form of its `accepts` Selector.
- [ADR-007: Security and Plugin Trust Model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md) — the Operator gets no store path; core checks and publishes its output; the web shows a pack's refusal reason only as text.
- [ADR-008: Metadata Envelope, Facets, and Write Authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md) — making previews and accepted media types are declared at registration, as facet grants are.
- [ADR-009: Backend Language and Runtime](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010: Migration Strategy — Clean Break with a One-Way Importer](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011: Storage Tree Layout and Cross-Filesystem Moves](../../../docs/decisions/ADR-011-storage-tree-layout.md) — a preview's bytes are staged, hashed and published before any record, like an upload's.
- [ADR-014: Managed Storage Identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)
- [ADR-015: Opaque Database-Backed Sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md) — the preview route sits behind the same session guard.
- [ADR-016: Development Environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md)
- [ADR-017: Facet Values as Rows with a JSONB Value](../../../docs/decisions/ADR-017-facet-value-rows.md) — this change adds no facet and no value type.

ADR-012 (proposed) and ADR-013 (superseded by ADR-016) were read but are not
in force.

## New Durable ADRs Created

- [ADR-018: Operator Output Becomes a Derived Asset Linked by Lineage](../../../docs/decisions/ADR-018-operator-output-as-derived-assets.md) — design decisions 1 and 4.
- [ADR-019: Operators Declare the Media Types They Accept](../../../docs/decisions/ADR-019-operators-accept-media-types.md) — design decisions 2 and 3.
- [ADR-020: Bound Preview Decoding in a Child Process](../../../docs/decisions/ADR-020-bound-preview-decoding-in-a-child-process.md) — design decision 8.
