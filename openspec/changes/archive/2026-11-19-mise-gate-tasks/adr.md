<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-19

## Review Summary

ADR review completed for this change. It needs no new record.

The design's decisions (D-1 to D-5 in `design.md`) are cheap to reverse and readable from `mise.toml` and the specs: how the `check` task orders its parts, what each task body runs, and how CI and the docs call them. The costly choice, mise as the tool manager and task runner, is already recorded in ADR-016. The task-body rule is a shared decision of the epic, not an architectural fork.

## In-Force ADRs Reviewed

- [ADR-016: Development Environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md). Read in full. This change follows it: CI and agents run the gate through mise, and the plain pin files stay for contributors who use neither mise nor Nix.
- Checked for relevance by title and scope; none bears on the gate or contributor tooling:
  [ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md),
  [ADR-002](../../../docs/decisions/ADR-002-core-domain-pack-split.md),
  [ADR-003](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md),
  [ADR-004](../../../docs/decisions/ADR-004-database-engine-and-schema.md),
  [ADR-005](../../../docs/decisions/ADR-005-visual-design-system.md),
  [ADR-006](../../../docs/decisions/ADR-006-rule-engine-deferral.md),
  [ADR-007](../../../docs/decisions/ADR-007-security-and-plugin-trust.md),
  [ADR-008](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md),
  [ADR-009](../../../docs/decisions/ADR-009-backend-language.md),
  [ADR-010](../../../docs/decisions/ADR-010-migration-strategy.md),
  [ADR-011](../../../docs/decisions/ADR-011-storage-tree-layout.md),
  [ADR-014](../../../docs/decisions/ADR-014-managed-storage-identity.md),
  [ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md).
- Not in force: ADR-012 is Proposed, and ADR-013 is superseded by ADR-016.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
