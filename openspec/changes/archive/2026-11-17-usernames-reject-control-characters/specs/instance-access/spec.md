## RENAMED Requirements

- FROM: `### Requirement: Claimed credentials meet the length rules`
- TO: `### Requirement: Claimed credentials meet the credential rules`

## MODIFIED Requirements

### Requirement: Claimed credentials meet the credential rules

`POST /api/claim` SHALL answer 400 with the body `{"error":"invalid credentials"}` and write nothing when the request breaks a rule. The rules are:

- The body is a JSON object with string `username` and `password`.
- The username is 1 to 64 characters.
- The username has no leading or trailing whitespace.
- The username holds no control character.
- The password is 8 to 256 bytes.

A control character is any character in the Unicode general category Cc, such as a line feed, an escape or a delete, wherever it appears in the username.

#### Scenario: Short password
- **WHEN** no administrator exists and a caller sends `POST /api/claim` with username `sam` and password `short`
- **THEN** the status is 400, the body is `{"error":"invalid credentials"}`, and the `users` table is empty

#### Scenario: Padded username
- **WHEN** no administrator exists and a caller sends `POST /api/claim` with username ` sam` and a valid password
- **THEN** the status is 400 and the `users` table is empty

#### Scenario: A control character in the username
- **WHEN** no administrator exists and a caller sends `POST /api/claim` with the username `sa\u001b[2Jm`, which holds an escape character, and a valid password
- **THEN** the status is 400, the body is `{"error":"invalid credentials"}`, and the `users` table is empty

#### Scenario: Not a credentials object
- **WHEN** no administrator exists and a caller sends `POST /api/claim` with the body `["sam"]`
- **THEN** the status is 400 and the `users` table is empty

### Requirement: The recovery command applies the claim's credential rules

The command SHALL apply the rules of the requirement "Claimed credentials meet the credential rules" to a username in the create state and to a password in either state. A violation SHALL be refused before any change: one line on stderr that names the rule broken and exit 1. A generated password meets the rules by construction.

#### Scenario: Short password on stdin
- **WHEN** an administrator exists and a script runs the reset with `short` on stdin
- **THEN** the exit status is 1, stderr names the password length, the hash is unchanged and the `sessions` table is unchanged

#### Scenario: Padded username
- **WHEN** no administrator exists and a script runs the command with `--username " sam"` and a valid password
- **THEN** the exit status is 1, stderr names the username, and the `users` table is empty

#### Scenario: A control character in the username
- **WHEN** no administrator exists and a script runs the command with a `--username` value of `sa`, a line feed and `m`, and a valid password on stdin
- **THEN** the exit status is 1, stderr is exactly `username must not contain control characters` and a line feed, and the `users` table is empty
