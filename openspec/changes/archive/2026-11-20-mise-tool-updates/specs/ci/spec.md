<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The tool update workflow grants only the token access it needs

`mise-tool-updates.yml` SHALL grant its jobs `contents: read` and no other permission. Its only write access SHALL come from a GitHub App token that asks for `contents: write` and `pull-requests: write`, and for no other permission. The only repository secret it SHALL refer to is `DEPENDABOT_MERGE_APP_PRIVATE_KEY`.

#### Scenario: The workflow's own token can only read

- **WHEN** a reviewer runs `yq '[.permissions, .jobs[].permissions] | map(select(. != null))' .github/workflows/mise-tool-updates.yml`
- **THEN** the output lists only `contents: read`

#### Scenario: The App token asks for two permissions

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "actions/create-github-app-token*") | .with | with_entries(select(.key | test("^permission-")))' .github/workflows/mise-tool-updates.yml`
- **THEN** the output is exactly two entries: `permission-contents: write` and `permission-pull-requests: write`

#### Scenario: The workflow uses one secret

- **WHEN** a reviewer runs `grep -o 'secrets\.[A-Z_]*' .github/workflows/mise-tool-updates.yml | sort -u`
- **THEN** the output is exactly one line: `secrets.DEPENDABOT_MERGE_APP_PRIVATE_KEY`
