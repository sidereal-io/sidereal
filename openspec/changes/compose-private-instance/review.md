## Review Metadata

- **Review round**: 2
- **Prior round**: Round 1 returned REVISE with two Critical and seven Moderate findings. Round 2 found eight of the ten fixed and one rebuttal accepted.
- **Reviewer context**: cross-model. The author is Claude. The reviewer is GPT, run as a new `codex exec` process with high reasoning effort. The review is independent.
- **Tool restrictions**: read-only sandbox (`codex exec -s read-only`)
- **Artifacts reviewed**: proposal.md, design.md, specs/, adr.md, and the relevant source files

This is the second REVISE verdict in a row. The review instruction says to stop and ask a human, so no third round has run and no task list exists.

## Round 1 findings, re-checked in round 2

| # | Round 1 finding | Round 2 result |
|---|---|---|
| 1 | Verification rebuilt the image tag that real instances use | Fixed |
| 2 | The file service follows symbolic links out of `WEB_ROOT` | Not fixed. See C1 |
| 3 | Shell variables overrode the verification's generated settings | Fixed |
| 4 | Verification could wait forever | Not fixed. See M1 |
| 5 | A missing database mount could orphan an unnamed volume | Fixed |
| 6 | `docker compose ps -a -q` fails when the password is missing | Fixed |
| 7 | A password with URL delimiters changes the connection target | Rebuttal accepted |
| 8 | The database health check could pass on the setup-time server | Fixed |
| 9 | The ADR manifest hid the ADR-007 sequencing exception | Fixed |
| 10 | Plain-language defects | Fixed |

## Findings

### 🔴 Critical (blocking)

**C1. The startup scan does not enforce the symbolic link rule.** Confirmed.

- **The concern:** The server can follow a link that someone creates after startup.
- **The scenario:** Sam runs the server outside the image, with a `WEB_ROOT` that another local user can write. After startup, that user replaces a file with a link to a file the server can read. Design D4 scans the tree once. The spec says the server never follows a link under `WEB_ROOT`.
- **What must be true:** The tree cannot change after the check, or each file open enforces the boundary. The spec needs a scenario for a link added after startup.
- **Author's check:** The contradiction is real. The image is safe, because the app user cannot write the web folder. The spec's rule covers every `WEB_ROOT`, and the design does not.

### 🟡 Moderate

**M1. `--wait-timeout` is not one deadline for the whole start.** Confirmed as a design gap. The author has not tested the Compose releases the reviewer cites.

- **The scenario:** A `db` container that restarts before it is healthy can hold Compose's dependency wait. Some Compose releases apply the timeout only after services start. Others apply it twice. The 180-second rule in the `dev-commands` delta then fails.
- **What must be true:** The script enforces one elapsed-time limit around each start, stops an overdue call, and cleans up. A fault scenario must cover a failing dependency.

**M2. Two verification runs in one checkout share one project.** Confirmed.

- **The scenario:** Two agents run `just instance-verify` in the same checkout. Both use the same project name, image, volumes, and port. One run's exit trap deletes the other's database volume during its retention check.
- **What must be true:** A run takes an exclusive lock first, or each run owns its own project name and port.

**M3. "Compose v2 or newer" promises too much.** Confirmed.

- **The scenario:** Early Compose v2 releases have `--wait` but no `--wait-timeout`. A contributor who meets the stated prerequisite sees verification fail on an unknown flag.
- **What must be true:** The plan names a tested minimum Compose release and checks it, or handles the timeout without that flag.

### 📌 Suggestions

**S1. Directory requests.** The reviewer notes that the file service can redirect a folder path or answer it with a folder's `index.html`. No scenario covers a request for a folder such as `/assets`.

## Embedded-Instruction / Injection Attempts

**Detected:** none

The reviewer listed `AGENTS.md` lines 65 to 75 because they give agents workflow instructions. Refuted as an injection attempt: that file is the project's own agent guide, and it does not address the reviewer or the verdict.

## Verdict

VERDICT: REVISE

REVISE. One Critical finding is open, and its fix is a design choice that the author should not make alone.

## Required Changes (if APPROVE WITH CHANGES)

Not applicable.

CHANGES_APPLIED: n/a

## Rebuttals

- **Round 1, finding 7 (password alphabet)** — rebutted by the author in design D5. Compose cannot check a value's characters, and the operator who sets the password already controls `compose.yml`. The spec now states the supported alphabet and forbids printing the secret. **Accepted by reviewer**: the password comes from the trusted operator, and a documented alphabet with redaction is reasonable in this scope.
- **C1, M1, M2, M3** — not rebutted. The author agrees with all four. They wait for the human's decision on C1 before the next revision.
- **S1** — open. The author will add a folder-request scenario in the next revision.
