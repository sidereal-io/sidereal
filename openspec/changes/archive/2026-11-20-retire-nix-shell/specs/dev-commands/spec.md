<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: The development tasks run without mise

Each development task SHALL run one shell command line, or one POSIX `sh` script, that works without mise on `PATH`, once the tools it needs are installed. No script that a task runs SHALL start `mise` or `just`. `CONTRIBUTING.md` SHALL list, for each development task, the command or script that the task runs, for contributors without mise.

#### Scenario: The listed commands match the tasks

- **WHEN** a reviewer compares the command that `CONTRIBUTING.md` lists for each development task with the output of `mise tasks info <task> --json | jq -r '.run[]'`
- **THEN** each pair is the same command

#### Scenario: A contributor without mise runs the stack

- **GIVEN** a shell with no `mise` and no `just` on `PATH`, the hand-installed tools that `CONTRIBUTING.md` lists, a prepared PostgreSQL fixture, and `DATABASE_URL` set to the fixture's runtime URL
- **WHEN** the contributor runs, from the repo root, the command that `CONTRIBUTING.md` lists for `dev`
- **AND** opens `http://localhost:5173` once the server has built
- **THEN** the page shows `healthy`

#### Scenario: A contributor without mise runs the end-to-end suite

- **GIVEN** a shell with no `mise` and no `just` on `PATH`, the hand-installed tools that `CONTRIBUTING.md` lists, a prepared PostgreSQL fixture, and no `DATABASE_URL` set
- **WHEN** the contributor runs, from the repo root, the command that `CONTRIBUTING.md` lists for `e2e`
- **THEN** the command exits with status 0
