# Sidereal Behavioral Specification Package

This package describes **what Sidereal currently does** — its externally observable behavior — so that a fresh implementation team can replicate compatible behavior without inheriting the current codebase's internal architecture, module layout, framework choices, or helper organization.

Source code was used strictly as **evidence for behavior**, never as a target design. Runtime observations (live HTTP calls, WebSocket connections, filesystem side effects, startup logs) take precedence over source inference wherever both exist.

## Package contents

| File | Required section(s) |
|---|---|
| [01-scope-and-evidence.md](01-scope-and-evidence.md) | 1. Scope · 2. Evidence Inventory |
| [03-behavioral-surface-map.md](03-behavioral-surface-map.md) | 3. Behavioral Surface Map |
| [04-domain-model.md](04-domain-model.md) | 4. Domain Model |
| [05-behavioral-specifications.md](05-behavioral-specifications.md) | 5. Behavioral Specifications |
| [06-api-contracts.md](06-api-contracts.md) | 6. API Contract Summary |
| [07-ui-workflows.md](07-ui-workflows.md) | 7. UI Workflow Specifications |
| [08-workers-and-integrations.md](08-workers-and-integrations.md) | 8. Worker and Integration Semantics |
| [09-persistence-and-migration.md](09-persistence-and-migration.md) | 9. Persistence and Migration Semantics |
| [10-error-contract.md](10-error-contract.md) | 10. Error Contract |
| [11-test-vectors.md](11-test-vectors.md) | 11. Test Vectors |
| [12-acceptance-criteria.md](12-acceptance-criteria.md) | 12. Acceptance Criteria |
| [13-compatibility-requirements.md](13-compatibility-requirements.md) | 13. Compatibility Requirements |
| [14-open-questions-and-conflicts.md](14-open-questions-and-conflicts.md) | 14. Open Questions and Unknowns · 15. Conflict Log |
| [16-provenance-index.md](16-provenance-index.md) | 16. Provenance Index |

## Conventions

- **Evidence IDs**: `SRC-*` source inspection, `SCHEMA-*` schemas/types, `MIG-*` migrations, `TEST-*` tests, `DOC-*` documentation, `RUN-*` command/runtime observations, `API-*` live HTTP observations, `WS-*` WebSocket observations, `UI-*` UI observations, `DOCKER-*` container artifacts, `FS-*` filesystem observations. The full inventory is in [01-scope-and-evidence.md](01-scope-and-evidence.md).
- **Confidence**: `High` = runtime-observed or directly test-asserted; `Medium` = source-inferred, single evidence path; `Low` = inferred from indirect evidence or docs only.
- **Fact classes**: claims are marked *Confirmed* (runtime/test), *Inferred* (source-only), *Conflict* (evidence disagrees — see the Conflict Log), or *Unknown*.
- Domain language: photo/asset ("image"), plate solve, sync, catalog object, target, equipment, equipment group, acquisition session, admin setting, notification, processed image, image source.
