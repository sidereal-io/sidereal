## ADDED Requirements

### Requirement: The recovery command follows the database's state

`sidectl reset-admin-password` SHALL read from PostgreSQL whether an administrator exists, never from memory or a setting. When none exists the command SHALL be in the create state; otherwise in the reset state. Before it asks for anything, it SHALL print one line on stdout: `No administrator exists. Creating one.` in the create state, or `Administrator <username> exists. Setting a new password and ending every session.` in the reset state, naming the administrator.

#### Scenario: Unclaimed instance
- **WHEN** no administrator exists and Sam runs `sidectl reset-admin-password`
- **THEN** the first line on stdout is `No administrator exists. Creating one.`

#### Scenario: Claimed instance
- **WHEN** an administrator named `sam` exists and Sam runs `sidectl reset-admin-password`
- **THEN** the first line on stdout is `Administrator sam exists. Setting a new password and ending every session.`

### Requirement: The create state creates the one administrator

In the create state the command SHALL create a user with the given username, an argon2 hash of the given password and the role `admin`. It SHALL print `Created administrator <username>.` on stdout and exit 0. The claim SHALL be closed afterwards, and the administrator SHALL be able to sign in with those credentials. When an administrator appears between the state check and the write, the command SHALL write nothing and exit 1. Its one line on stderr SHALL say that an administrator now exists and to run the command again.

#### Scenario: Create from the box
- **WHEN** no administrator exists and Sam runs `sidectl reset-admin-password --username sam --password-stdin` with `correct-horse-battery` on stdin
- **THEN** the exit status is 0, stdout holds `Created administrator sam.`, `GET /api/claim` answers `{"open":false}`, and `POST /api/session` with username `sam` and that password answers 200

#### Scenario: The claim closes between the check and the write
- **WHEN** no administrator exists when the command starts, and a `POST /api/claim` creates one before the command writes
- **THEN** the exit status is 1, stderr says an administrator now exists, and the `users` table holds only the claimed administrator

### Requirement: The reset state sets a new password and ends every session

In the reset state the command SHALL replace the administrator's password hash with an argon2 hash of the given password, and delete every row in the `sessions` table. Both SHALL happen in one transaction, so no outcome has one without the other. It SHALL print `Reset the password of administrator <username> and ended <n> sessions.` on stdout, with `<n>` the number of rows deleted, and exit 0. A request that carries the cookie of a deleted session SHALL be refused as the requirement "Every API route needs an administrator session" says. This holds whether or not the server was running during the reset.

#### Scenario: Reset while signed in
- **WHEN** Sam is signed in twice and runs `sidectl reset-admin-password --password-stdin` with `new-horse-battery` on stdin
- **THEN** the exit status is 0, stdout holds `Reset the password of administrator sam and ended 2 sessions.`, the `sessions` table is empty, `GET /api/session` with either old cookie answers 401, `POST /api/session` with the old password answers 401, and `POST /api/session` with `new-horse-battery` answers 200

#### Scenario: Reset with no session
- **WHEN** an administrator exists, no session does, and Sam runs the reset with a valid password
- **THEN** the exit status is 0 and stdout holds `ended 0 sessions.`

### Requirement: Credentials come from the terminal or from the flags

Without `--password-stdin` the command SHALL prompt on the terminal. In the create state it SHALL ask `Username:` with the answer echoed, unless `--username` was given, and then `New password:` and `Repeat password:` with no echo. In the reset state it SHALL ask only the two password prompts. When the two answers differ it SHALL write nothing, print `passwords do not match` on stderr and exit 1. Prompts SHALL be written to stderr, so stdout holds only the state line, a generated password and the result. With `--password-stdin` the command SHALL never prompt. It SHALL read the first line of stdin as the password, without its line ending. In the create state the username SHALL then come from `--username`; without it, `--password-stdin` is a usage error. An empty or missing first line under `--password-stdin` SHALL be refused: one line on stderr saying a password is required, and exit 1. Without the flag, a stdin that closes before an answer to a prompt SHALL be refused: one line on stderr saying stdin closed before an answer, and exit 1. No refusal in this requirement changes the database.

#### Scenario: Script mode creates the administrator
- **WHEN** no administrator exists and a script runs `sidectl reset-admin-password --username sam --password-stdin` with `correct-horse-battery` and a line feed on stdin
- **THEN** the exit status is 0 and the stored hash matches `correct-horse-battery`, not `correct-horse-battery` followed by a line feed

#### Scenario: Script mode without a username on an unclaimed instance
- **WHEN** no administrator exists and a script runs `sidectl reset-admin-password --password-stdin`
- **THEN** the exit status is 2, stderr names `--username`, and the `users` table is empty

#### Scenario: Script mode with an empty line
- **WHEN** an administrator exists and a script runs `sidectl reset-admin-password --password-stdin` with a single line feed on stdin
- **THEN** the exit status is 1, stderr says a password is required, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: Script mode with a closed stdin
- **WHEN** an administrator exists and a script runs `sidectl reset-admin-password --password-stdin` with stdin closed and nothing on it
- **THEN** the exit status is 1, stderr says a password is required, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: A closed stdin without the flag
- **WHEN** an administrator exists and a script runs `sidectl reset-admin-password` with stdin closed, as `docker compose exec -T` with no input does
- **THEN** the exit status is 1, stdout holds only the state line, stderr says stdin closed, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: The repeated password differs
- **WHEN** Sam answers the prompts `New password:` with `correct-horse-battery` and `Repeat password:` with `correct-horse-batery`
- **THEN** stderr holds `passwords do not match`, the exit status is 1, and the `users` and `sessions` tables are unchanged

### Requirement: A given username must name the administrator in the reset state

In the reset state, a `--username` that is not the administrator's username SHALL be refused before any change. The command SHALL print one line on stderr naming the administrator and exit 1. A `--username` equal to the administrator's username SHALL be accepted.

#### Scenario: The wrong username
- **WHEN** the administrator is `sam` and a script runs `sidectl reset-admin-password --username alex --password-stdin` with a valid password
- **THEN** the exit status is 1, stderr names `sam`, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: The right username
- **WHEN** the administrator is `sam` and a script runs `sidectl reset-admin-password --username sam --password-stdin` with a valid password
- **THEN** the exit status is 0

### Requirement: An empty answer at the password prompt is generated

When the answer to `New password:` is an empty line, the command SHALL generate a random password of 24 characters drawn from ASCII letters and digits. It SHALL skip `Repeat password:`. It SHALL print `Generated password: <password>` on stdout, before the line that says what it did, and use that password. This is the only output that ever holds a password. Only the prompt on a terminal generates a password. When stdin is not a terminal, an empty answer to `New password:` SHALL be refused like an empty line under `--password-stdin`, and `--password-stdin` never generates one.

#### Scenario: Empty answer at the prompt
- **WHEN** an administrator exists and Sam answers `New password:` with an empty line
- **THEN** the command does not ask `Repeat password:`, the exit status is 0, stdout holds one line `Generated password: <p>` where `<p>` is 24 ASCII letters and digits, that password appears once in stdout and never in stderr, and `POST /api/session` with `<p>` answers 200

#### Scenario: Empty answer through a pipe
- **WHEN** an administrator exists and a script runs `sidectl reset-admin-password` without `--password-stdin`, with a single line feed piped to stdin
- **THEN** the exit status is 1, stderr says a password is required, no `Generated password:` line is printed, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: Empty answer in the create state
- **WHEN** no administrator exists and Sam answers `Username:` with `sam` and `New password:` with an empty line
- **THEN** the exit status is 0, stdout holds a `Generated password:` line and `Created administrator sam.`, and `POST /api/session` with `sam` and the generated password answers 200

### Requirement: The recovery command applies the claim's credential rules

The command SHALL apply the rules of the requirement "Claimed credentials meet the length rules" to a username in the create state and to a password in either state. A violation SHALL be refused before any change: one line on stderr that names the rule broken and exit 1. A generated password meets the rules by construction.

#### Scenario: Short password on stdin
- **WHEN** an administrator exists and a script runs the reset with `short` on stdin
- **THEN** the exit status is 1, stderr names the password length, the hash is unchanged and the `sessions` table is unchanged

#### Scenario: Padded username
- **WHEN** no administrator exists and a script runs the command with `--username " sam"` and a valid password
- **THEN** the exit status is 1, stderr names the username, and the `users` table is empty

### Requirement: The recovery command uses the server's database settings and never changes the schema

The command SHALL read `DATABASE_URL`, the timeout settings and the TLS settings as the server does, with the same errors for a missing or invalid value. It SHALL check that the database holds the current Sidereal schema before it reads the `users` table. It SHALL apply no migration and create no table. When the database is empty or behind, the command SHALL print one line on stderr that names the cause and says to start the server first, and exit 1. For any other database fault, such as a database that is not Sidereal's or a rejected login, it SHALL print the same line the server prints for that fault, and exit 1.

#### Scenario: An empty database
- **WHEN** `DATABASE_URL` names an empty database that no server has started against, and a script runs the command with valid credentials
- **THEN** the exit status is 1, stderr says to start the server first, and the database still holds no `_sqlx_migrations` table

#### Scenario: A database that is not Sidereal's
- **WHEN** `DATABASE_URL` names a database that holds another application's tables, and a script runs the command with valid credentials
- **THEN** the exit status is 1, stderr starts with `database is not an empty or owned Sidereal database`, and the database is unchanged

#### Scenario: Missing configuration
- **WHEN** a script runs the command with `DATABASE_URL` unset
- **THEN** the exit status is 1 and stderr starts with `missing database configuration`

### Requirement: The recovery command keeps secrets out of its output

No stdout or stderr line of the command SHALL hold a password it was given, a password hash, a session token or the database URL. A generated password appears exactly once, in its `Generated password:` line.

#### Scenario: A distinctive password on stdin
- **WHEN** a script runs the reset with a distinctive password on stdin against a database whose URL holds a distinctive database password
- **THEN** neither distinctive value appears in stdout or stderr, and no `$argon2` string does

### Requirement: The command line is checked before anything runs

`sidectl --help` and `sidectl reset-admin-password --help` SHALL print the usage on stdout and exit 0 without touching the database. No command, an unknown command, an unknown flag or a flag without its value SHALL print the usage on stderr and exit 2 without touching the database.

#### Scenario: Help
- **WHEN** a script runs `sidectl --help` with `DATABASE_URL` unset
- **THEN** the exit status is 0 and stdout names `reset-admin-password`, `--username` and `--password-stdin`

#### Scenario: Unknown command
- **WHEN** a script runs `sidectl rotate-keys`
- **THEN** the exit status is 2 and stderr holds the usage
