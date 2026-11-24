## MODIFIED Requirements

### Requirement: mise tasks run the development commands

`mise.toml` SHALL define fifteen development tasks, for the database fixture, the server and the web shell, the end-to-end suite, the disposable deployment checks, and the clean-up of the Rust build output. A contributor runs each one as `mise run <task>` from the repo root.

Each task SHALL have a description, which `mise tasks ls` prints.

#### Scenario: The development tasks exist

- **WHEN** a reviewer runs `mise tasks ls --no-header` at the repo root
- **THEN** the output includes `db-up`, `db-down`, `db-clean`, `db-url`, `db-test-url`, `db-test-clean`, `db-tls`, `db-schema`, `server`, `web`, `dev`, `e2e`, `instance-verify`, `external-db-verify` and `target-clean`

#### Scenario: Every task says what it does

- **WHEN** a reviewer runs `mise tasks ls --json | jq -r '.[] | select((.description // "") == "") | .name'` at the repo root
- **THEN** the output is empty

## ADDED Requirements

### Requirement: A task deletes the Rust build output once it is over a size limit

`mise run target-clean` SHALL measure the repo's `server/target/` folder. The folder's size is the disk space that `du -sk` reports for it, rounded up to whole MiB. When the folder is larger than the size limit, the task SHALL delete it. When the folder is at or under the limit, the task SHALL leave it in place. Either way it SHALL print the folder's size and the limit, in MiB, and exit with status 0.

#### Scenario: The folder is over the limit

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with status 0
- **AND** `server/target/` no longer exists
- **AND** the output names the folder's size, the limit of 1 MiB, and that the folder was deleted

#### Scenario: The folder is under the limit

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=10 mise run target-clean` at the repo root
- **THEN** the command exits with status 0
- **AND** the 2 MiB file is still in `server/target/`
- **AND** the output names the folder's size, the limit of 10 MiB, and that the folder was kept

#### Scenario: The folder is at the limit

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **AND** `SIDEREAL_TARGET_LIMIT_MIB` set to the folder's size: the `du -sk` value for `server/target`, rounded up to whole MiB
- **WHEN** a contributor runs `mise run target-clean` at the repo root
- **THEN** the command exits with status 0
- **AND** the 2 MiB file is still in `server/target/`
- **AND** the output says that the folder was kept

#### Scenario: There is no build output

- **GIVEN** no `server/target/` folder
- **WHEN** a contributor runs `mise run target-clean` at the repo root
- **THEN** the command exits with status 0
- **AND** the output says there is nothing to delete

### Requirement: The clean-up task stops safely when it cannot measure or delete the folder

When `mise run target-clean` cannot measure the `server/target/` folder, the task SHALL delete nothing and exit with a non-zero status. When deleting the folder fails, the task SHALL exit with a non-zero status, and its error output SHALL say that the folder may be partly deleted.

#### Scenario: The size cannot be measured

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **AND** a `du` command first on `PATH` that exits with a non-zero status
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the 2 MiB file is still in `server/target/`

#### Scenario: The folder cannot be deleted

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **AND** an `rm` command first on `PATH` that exits with a non-zero status
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the error output says that the folder may be partly deleted

### Requirement: Contributors can change the clean-up size limit

The size limit of `mise run target-clean` SHALL be 4096 MiB. A contributor SHALL be able to set another limit, as a whole number of MiB, in the `SIDEREAL_TARGET_LIMIT_MIB` environment variable. When that variable holds anything else, the task SHALL delete nothing and exit with a non-zero status.

#### Scenario: The default limit

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data, and no `SIDEREAL_TARGET_LIMIT_MIB` in the environment
- **WHEN** a contributor runs `mise run target-clean` at the repo root
- **THEN** the output names a limit of 4096 MiB
- **AND** the 2 MiB file is still in `server/target/`

#### Scenario: The limit is not a whole number

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=2G mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the error output names `SIDEREAL_TARGET_LIMIT_MIB`
- **AND** the 2 MiB file is still in `server/target/`

### Requirement: The clean-up task deletes only this checkout's build folder

`mise run target-clean` SHALL delete nothing but the repo's own `server/target/` folder. It SHALL ignore `CARGO_TARGET_DIR` and any Cargo setting that moves the build output elsewhere. When `server/target` is a symbolic link, the task SHALL delete nothing and exit with a non-zero status.

#### Scenario: A personal build folder elsewhere is left alone

- **GIVEN** no `server/target/` folder, and `CARGO_TARGET_DIR` set to another folder that holds a 2 MiB file of random data
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with status 0
- **AND** the 2 MiB file is still in the folder that `CARGO_TARGET_DIR` names

#### Scenario: The build folder is a link

- **GIVEN** `server/target` is a symbolic link to another folder that holds a 2 MiB file of random data
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the link and the 2 MiB file both still exist

### Requirement: The clean-up task refuses mount points

When `server`, `server/target`, or any folder inside `server/target`, is a mount point, `mise run target-clean` SHALL delete nothing and exit with a non-zero status. The task SHALL find mount points in the system mount table, which `mount` prints. When it cannot read that table, or when the folder's physical path holds a space, tab, newline or backslash, which `mount` prints escaped, the task SHALL delete nothing and exit with a non-zero status.

#### Scenario: The build folder is a mount point

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **AND** a `mount` command first on `PATH` whose output lists the physical path of `server/target` as a mount point
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the 2 MiB file is still in `server/target/`

#### Scenario: A folder inside the build folder is a mount point

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **AND** a `mount` command first on `PATH` whose output lists the physical path of `server/target/debug/cache` as a mount point
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the 2 MiB file is still in `server/target/`

#### Scenario: The server folder is a mount point

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **AND** a `mount` command first on `PATH` whose output lists the physical path of `server` as a mount point
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the 2 MiB file is still in `server/target/`

#### Scenario: The mount table cannot be read

- **GIVEN** a `server/target/` folder that holds a 2 MiB file of random data
- **AND** a `mount` command first on `PATH` that exits with a non-zero status
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the 2 MiB file is still in `server/target/`

#### Scenario: The build folder's path holds a space

- **GIVEN** a repo checkout inside a folder whose name holds a space, with a `server/target/` folder that holds a 2 MiB file of random data
- **WHEN** a contributor runs `SIDEREAL_TARGET_LIMIT_MIB=1 mise run target-clean` at the repo root
- **THEN** the command exits with a non-zero status
- **AND** the 2 MiB file is still in `server/target/`

### Requirement: The gate does not clean the build output

`mise run check` SHALL NOT run the `target-clean` task, so a gate run never deletes build output.

#### Scenario: The gate does not run the clean-up

- **WHEN** a reviewer runs `mise tasks info check --json` at the repo root
- **THEN** the output does not contain `target-clean`
