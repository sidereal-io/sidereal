## ADDED Requirements

### Requirement: Dependabot proposes Nix pin updates every week

Dependabot SHALL check the inputs in `flake.lock` every week. When an input has a newer upstream commit, Dependabot SHALL open a pull request that updates it. The configuration SHALL ask Dependabot to group all Nix inputs into one pull request.

#### Scenario: The configuration declares the Nix updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "nix")' .github/dependabot.yml`
- **THEN** the output is exactly one entry
- **AND** that entry has `directory` set to `/` and `schedule.interval` set to `weekly`
- **AND** that entry has a group whose `patterns` list is `["*"]`

#### Scenario: A flake input has a newer commit

- **WHEN** Dependabot's weekly run finds a newer upstream commit for an input in `flake.lock`
- **THEN** an open pull request from Dependabot changes `flake.lock`
- **AND** that pull request changes no other file

### Requirement: Dependabot proposes Cargo updates every week

Dependabot SHALL check the crates in `backend/Cargo.lock` every week. When a crate has a newer release, Dependabot SHALL open one grouped pull request for all the updated crates.

#### Scenario: The configuration declares the Cargo updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "cargo")' .github/dependabot.yml`
- **THEN** the output is exactly one entry
- **AND** that entry has `directory` set to `/backend` and `schedule.interval` set to `weekly`
- **AND** that entry has a group whose `patterns` list is `["*"]`

#### Scenario: Several crates have newer releases

- **WHEN** Dependabot's weekly run finds newer releases for two or more crates
- **THEN** one open pull request from Dependabot updates all of them
- **AND** that pull request changes files under `backend/` only

### Requirement: CI builds the shell on every pull request that changes it

CI SHALL run `nix flake check` on every pull request that changes a file the shell is built from. These files are `flake.nix`, `flake.lock`, any file under `nix/`, and `backend/rust-toolchain.toml`. A change to the workflow file itself SHALL also run the check. The check SHALL fail when the shell cannot be built.

#### Scenario: A pull request breaks the shell

- **WHEN** a pull request changes `flake.nix` so that the development shell cannot be built
- **THEN** the flake check on that pull request fails

#### Scenario: A pull request leaves the shell working

- **WHEN** a pull request changes `flake.lock` and the development shell still builds
- **THEN** the flake check on that pull request passes

#### Scenario: A pull request does not touch the shell

- **WHEN** a pull request changes none of the listed files
- **THEN** GitHub does not run the flake check on that pull request

### Requirement: The flake check runs without secrets

The flake check SHALL use no repository secret. Its workflow SHALL grant the job read-only access to repository contents and no other permission. A Dependabot pull request SHALL therefore run the check with no extra setup.

#### Scenario: A reviewer inspects the workflow permissions

- **WHEN** a reviewer reads the flake check workflow file
- **THEN** its `permissions` block grants only `contents: read`
- **AND** the file contains no reference to `secrets.`

#### Scenario: Dependabot opens a Nix pin update

- **WHEN** Dependabot opens a pull request that changes `flake.lock`
- **THEN** the flake check runs on that pull request and reports pass or fail

### Requirement: The flake check reports the shell's tool versions

After the shell builds, the flake check SHALL print the versions of `openspec`, Node, and `just` from inside the shell. A reviewer SHALL be able to read them in the job log.

#### Scenario: A reviewer checks what a pin update changed

- **WHEN** the flake check passes on a pull request
- **THEN** the job log contains the output of `openspec --version`, `node --version`, and `just --version`, each run inside the development shell
