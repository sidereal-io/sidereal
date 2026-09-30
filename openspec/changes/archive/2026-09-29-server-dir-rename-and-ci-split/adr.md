# ADR Review Manifest

- Status: completed
- Review date: 2026-09-29

## Review Summary

ADR review completed for this change. The design makes six decisions (D1–D6). None is a real fork that is costly to reverse:

- D1, D2, and D5 are a path rename. A later rename or revert is a mechanical move.
- D3 and D4 change which workflow runs for which files. Each is a few lines of workflow config and cheap to change.
- D6 chooses where a spec's requirements live. That is spec organization, not architecture.

No existing ADR names the `backend/` path, so none needs an update or a supersession.

## In-Force ADRs Reviewed

- ADR-001 Plugin boundary (accepted)
- ADR-002 Core / domain pack split (accepted)
- ADR-003 Asset identity and content revisions (accepted)
- ADR-004 Database engine and schema (accepted)
- ADR-005 Frontend continuity (accepted)
- ADR-006 Rule engine deferral (accepted)
- ADR-007 Security and plugin trust (accepted)
- ADR-008 Facet schema and write authority (accepted)
- ADR-009 Backend language (accepted)
- ADR-010 Migration strategy (accepted)
- ADR-011 Storage tree layout (accepted)
- ADR-012 Embedded scripting engine (proposed, not in force)
- ADR-013 Development environment (accepted)

No ADR supersedes another. The highest number in use is 013.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
