# Verification Report: use-existing-postgres

Verified on 2026-10-21 against implementation `38ddc1f` on draft PR #432.
Schema: `spec-driven-review`. Scope: this change's artifacts and implementation, before archive.

## Summary

| Dimension | Status |
| --- | --- |
| Completeness | 19/19 tasks complete; all 9 ADDED or MODIFIED requirements have implementations |
| Correctness | 9/9 requirements mapped; 24/25 scenarios have test or direct-check evidence; 1 scenario coverage warning |
| Coherence | All 5 design decisions followed; file layout, shell conventions, and resource ownership follow project patterns |

All six applicable checks ran: Task Completion, Spec Coverage, Requirement Implementation Mapping, Scenario Coverage, Design Adherence, and Code Pattern Consistency.
No checks were skipped. There are no REMOVED or RENAMED requirements.
The schema explicitly tracks `tasks.md`; the CLI reports all 19 tasks complete with no remaining tasks.
The apply command's `all_done` state was treated as task evidence, not as a verification verdict.

## Requirement implementation mapping

| Requirement | Implementation and evidence |
| --- | --- |
| The operator supplies the database password (MODIFIED) | `compose.yml:12` and `compose.yml:21` require a nonempty bundled password without a default. `.env.example:4` states the alphabet and `:8` is empty. External mode requires only its URL. `scripts/instance-verify.sh:200` checks missing and empty passwords. Static checks confirm the example contains one empty password assignment and Git ignores `.env`. The delimiter-specific coverage gap is W1 below. |
| External mode starts only the app | `compose.external-db.yml:3` defines only `app` and `storage`, extends `compose.app.yml`, and requires `DATABASE_URL`. `scripts/external-db-verify.sh:186` checks missing URLs; `:199` compares both app contracts; `:230` checks service and volume lists. `Dockerfile:35` retains the app user and readiness probe. |
| External mode preserves database and storage safeguards | `server/crates/server/src/main.rs:22` retains initialization before HTTP binding. `server/crates/core/src/database/startup.rs:212` enforces PostgreSQL 18; `:364` validates ownership and history. `scripts/external-db-verify.sh:342` snapshots retained records and bytes; `:355` checks recreation and external container identity; `:517` checks unrelated data and newer history. |
| External mode permits the existing storage override | `compose.storage-bind.yml:5` preserves the storage mount and missing-source protection. `scripts/external-db-verify.sh:222` rejects the external database-bind combination; `:370` verifies both storage modes; `:429` checks missing paired markers without changing state. |
| External transport never silently becomes insecure | `compose.external-db.yml:9` forwards the URL unchanged. `server/crates/server/src/config.rs:39` retains verified remote defaults and allowed TLS modes. `scripts/external-db-verify.sh:443` proves default TLS with a failed plaintext control; `:463` rejects bad certificates while plaintext is available. `:336` checks redaction. |
| Operators can mount database certificates read-only | `compose.database-tls.yml:5` requires the directory and mounts it read-only without creating it. `scripts/external-db-verify.sh:351` inspects the mount; `:494` checks absent directories; `:487` and `:506` compare authorization counts around certificate-file failures. |
| The installation guide explains external operation | `docs/install.md:79` covers entry selection, networking, encoding, and credentials. `:106` covers PostgreSQL, ownership, and migrations; `:145` covers TLS and trust roots; `:178` covers storage and lifecycle; `:197` covers availability and rotation. `scripts/external-db-verify.sh:411` demonstrates distinct fresh storage while preserving the bundled installation. |
| External database availability recovers without resetting state | `compose.app.yml:16` retains startup retries. `scripts/external-db-verify.sh:545` checks late startup, HTTP refusal, restart count, recovery, and retained state. `:570` checks a 60-second runtime outage, health responses, ordered logs, unchanged restart count, and recovery within five seconds of database availability. |
| Deployment verification covers external operation safely | `justfile:78` runs the external demonstration. `scripts/external-db-verify.sh:7` confines scratch data to `.workspace`; `:31` clears inherited operator settings; `:39` passes explicit file, environment, and project selections. `:60`, `:80`, and `:130` constrain helpers and cleanup to recorded resources. Interrupted-state reproductions and operator-setting checks passed during apply and review. |

## Scenario coverage

The bundled scenarios below belong to the MODIFIED requirement. All other scenarios belong to ADDED requirements.

| Scenario | Coverage |
| --- | --- |
| The password is missing | `scripts/instance-verify.sh:200` exercises refusal and checks that no resources were created. |
| The password is empty | The same loop supplies an empty assignment and verifies refusal. |
| The repo holds no password | Direct checks confirm exactly one empty assignment and no password default in `compose.yml`. |
| The password holds URL delimiters | **W1:** no test or recorded run covers the specified delimiter-bearing password. |
| The example file states the alphabet | Directly checked `.env.example:4`. |
| Git ignores the settings file | `git check-ignore .env` passed. |
| Only external credentials are present | `scripts/external-db-verify.sh:230` checks only `app` and `storage`, without a bundled password. |
| External credentials are missing or empty | `scripts/external-db-verify.sh:186` checks both values and the absence of containers. |
| App restrictions match | `scripts/external-db-verify.sh:199` compares the app contract and asserts restrictions. Runtime identity checks are at `:375`; both entries build the same Dockerfile with its readiness probe. |
| Initialize and recreate an external instance | `scripts/external-db-verify.sh:355` and `:370` compare records, marker and sentinel bytes, and external container identity for both storage modes. |
| The selected database contains unrelated data | `scripts/external-db-verify.sh:517` checks refusal, unbound HTTP, and equal database dumps. |
| The selected database belongs to a newer release | `scripts/external-db-verify.sh:527` inserts an unknown migration and checks refusal with unchanged contents. |
| An external installation selects the bundled database bind override | `scripts/external-db-verify.sh:222` requires configuration failure with the expected missing-image diagnostic. |
| Choose managed storage independently | `scripts/external-db-verify.sh:370` verifies the actual bind mount. Configuration matrix checks also resolved the supported combinations. |
| A paired external database finds empty substitute storage | `scripts/external-db-verify.sh:429` checks the diagnostic, no HTTP or marker creation, and unchanged database records. |
| Remote transport defaults to verification | `scripts/external-db-verify.sh:443` reaches readiness without `sslmode` while the fixture rejects plaintext; the explicit plaintext control fails. |
| An untrusted CA or wrong hostname fails | `scripts/external-db-verify.sh:463` permits plaintext, proves it works, then checks both TLS refusals, unchanged dumps, and redaction. |
| A private CA is available to the app | `scripts/external-db-verify.sh:351` and `:370` verify read-only mounting and readiness using the fixture CA. |
| The host certificate directory is absent | `scripts/external-db-verify.sh:494` checks refusal, no source creation, and no app container. |
| A named certificate file is unreadable | `scripts/external-db-verify.sh:506` checks both missing and unreadable files, unchanged authorization count and database contents, and redacted diagnostics. |
| Follow the guide using disposable infrastructure | The external recipe uses the documented entry and TLS/storage settings, initializes only the app in its project, and verifies recreation. |
| A fresh external installation uses separate storage | `scripts/external-db-verify.sh:411` compares volume names and UUIDs and preserves the bundled marker and pairing. |
| The external database starts late | `scripts/external-db-verify.sh:545` observes startup refusal and a retry, then readiness and retained state. |
| The external database stops during operation | `scripts/external-db-verify.sh:570` checks the full 60-second outage and recovery without app restart. |
| Run with operator settings in the environment | Full external verification passed with injected operator settings. A separate root `.env` check confirmed those values were ignored and the file stayed byte-identical. Explicit Compose arguments and environment clearing support that result. |

## Coherence

All five design decisions are followed:

1. Separate entries share the credential-free app definition and retain bundled resources and the default command.
2. The server's PostgreSQL and transport policy is reused without changing server code.
3. The certificate override is optional, read-only, and documented for external connection URLs.
4. Guidance and the disposable fixture use a dedicated non-superuser owner login and preserve storage pairing.
5. Separate fixture and app projects prove retained state, TLS rejection, availability recovery, and cleanup ownership.

The new script follows the existing `scripts/instance-verify.sh` pattern: POSIX shell, private diagnostics, explicit Compose selections, locking, and recorded resource ownership.
No architecture, dependency, database format, or web design change was introduced.
Independent code review covered `b873759`; all four findings are recorded as confirmed and fixed in `code-review.md`.
The five listed fix diffs were inspected during review closeout. Later changes through `38ddc1f` only record that review.

## Validation evidence and limits

- This verification reran strict OpenSpec validation, shell syntax, static settings checks, Git ignore verification, and the official Compose 2.20.0 checksum comparison. All passed.
- Existing session evidence confirms the configuration matrix on Compose 5.6.0 and official 2.20.0, including mutation checks for app divergence, lost capabilities, and an incorrectly accepted override.
- The session's `deli -- just check` passed server gates and all 84 web tests. No server or web code changed after that run.
- Both deployment demonstrations passed. External verification passed again after the cleanup and coverage fixes. The final YAML-format correction was checked with configuration and mutation tests on both Compose versions.
- Existing successful gates were not rerun without a new implementation change or unresolved runtime concern.

## Issues by priority

### CRITICAL

None.

### WARNING

**W1 — Scenario not covered: The password holds URL delimiters.**

`openspec/changes/use-existing-postgres/specs/compose-instance/spec.md:36` specifies a fresh-volume run with `Zq7@evil.invalid/x?sslmode=disable#` and a 60-second observation.
`scripts/instance-verify.sh:458` instead tests a different alphanumeric password against an established database.
That proves wrong-login redaction, but does not exercise URL parsing when reserved characters alter the bundled URL.
The existing Rust configuration tests also do not reproduce this packaged scenario.

Recommendation: add the specified fresh-volume case to `scripts/instance-verify.sh`, then check that the app remains unhealthy after 60 seconds and logs contain neither the password prefix nor the connection URL.
This gap already exists in the baseline scenario; verification found no evidence of a new behavioral regression.

### SUGGESTION

None.

## Final assessment

No critical issues. One warning to consider. Ready for archive with the noted scenario-coverage improvement.
This verification did not implement changes or archive the change.
