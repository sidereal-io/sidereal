## MODIFIED Requirements

### Requirement: Verification covers the Compose contract safely

The regular check gate and CI SHALL run daemon-free Compose configuration checks for the entry files, URL requirements, and overrides.

#### Scenario: Configuration checks need no daemon

- **GIVEN** the Compose CLI and `jq`, with no reachable Docker daemon
- **WHEN** a tester runs `mise run check-compose`
- **THEN** configuration checks pass without creating containers, networks, or volumes
