## MODIFIED Requirements

### Requirement: A security event is written once its act takes effect

A `security event:` line SHALL be written only once its act has taken effect, and SHALL be written even when what follows the act fails. No security event SHALL write to PostgreSQL beyond the act itself.

#### Scenario: The generated password cannot be shown
- **WHEN** Sam resets the password from the box, leaves `New password:` empty, and stdout cannot be written, so the command exits 1
- **THEN** the command's stderr holds exactly one line that starts with `security event:`, because the reset took effect

#### Scenario: Nothing is stored
- **WHEN** Sam claims, signs in, fails a sign-in, signs out, and resets the password from the box
- **THEN** the database's list of application tables is the same after these acts as before them
- **AND** every application table other than `users` and `sessions` holds the same rows after these acts as before them
