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

### Requirement: The tool update workflow keeps the App key away from the updated tools

`mise-tool-updates.yml` SHALL run the tool update in a job named `update` and open the pull request in a separate job named `publish`, which runs only after `update` succeeds. Only `publish` SHALL refer to the App's client ID or private key. `publish` SHALL NOT install tools with mise or run a `mise` command, and SHALL read nothing from `update` except one patch file.

#### Scenario: Only the publish job holds the App credentials

- **WHEN** a reviewer runs `yq '.jobs | to_entries | map(select(.value | tostring | test("DEPENDABOT_MERGE_APP"))) | map(.key)' .github/workflows/mise-tool-updates.yml`
- **THEN** the output lists exactly one job: `publish`

#### Scenario: The publish job waits for the update job

- **WHEN** a reviewer runs `yq '.jobs.publish.needs' .github/workflows/mise-tool-updates.yml`
- **THEN** the output is `update`

#### Scenario: The publish job runs no updated tool

- **WHEN** a reviewer reads the steps of the `publish` job in `.github/workflows/mise-tool-updates.yml`
- **THEN** no step uses `jdx/mise-action`
- **AND** no step's `run` text contains `mise`
- **AND** no expression in the job refers to `needs.update`

### Requirement: The tool update workflow publishes only the files an update may change

Before it creates the App token, the `publish` job SHALL check every path that the update changed. The allowed paths SHALL be `mise.toml`, `mise.lock`, `AGENTS.md`, and the files under `.agents/skills/` and `openspec/schemas/`. A changed path SHALL be a regular file, and the update SHALL NOT rename or copy a file. When any path fails the check, the job SHALL fail, SHALL print each refused path, SHALL leave the working tree unchanged, and SHALL open, update or push to no pull request.

#### Scenario: An update changes a workflow file

- **WHEN** the update job's patch changes `mise.lock` and `.github/workflows/ci.yml`
- **THEN** the `publish` job fails before its App token step
- **AND** its log names `.github/workflows/ci.yml`
- **AND** `mise.lock` in the publish job's working tree is unchanged

#### Scenario: An update adds a link under the skills folder

- **WHEN** the update job's patch creates `.agents/skills/link` as a symbolic link
- **THEN** the `publish` job fails before its App token step

#### Scenario: An update changes only allowed files

- **WHEN** the update job's patch changes `mise.lock`, `AGENTS.md` and a file under `.agents/skills/`
- **THEN** the `publish` job's working tree holds those changes before its App token step

#### Scenario: An update changes nothing

- **WHEN** the update job's patch is empty
- **THEN** the `publish` job's working tree is clean before its App token step
- **AND** the run opens no pull request
