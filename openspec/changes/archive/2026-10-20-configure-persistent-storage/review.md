## Review Metadata

- **Review round**: 1
- **Prior round**: none (an earlier delegation was rejected before it ran, so no review exists)
- **Reviewer context**: cross-model, Claude Opus 5.5 (`claude-opus-5-5`) running as a fresh-context review sub-agent for a GPT-authored plan
- **Tool restrictions**: read-only (file reads, grep, and shell inspection only; no edits, builds, tests, git changes, or delegation)
- **Artifacts reviewed**: `proposal.md`, `design.md`, `adr.md`, `specs/compose-instance/spec.md`, `specs/managed-storage/spec.md`, `specs/postgres-foundation/spec.md`, and `docs/decisions/ADR-014-managed-storage-identity.md`. Source compared: `compose.yml`, `Dockerfile`, `.env.example`, `docs/install.md`, `scripts/instance-verify.sh`, `server/crates/core/src/database/{mod.rs,startup.rs}`, `server/crates/core/migrations/1_foundation.sql`, `server/crates/core/schema/*`, `server/crates/server/src/{config.rs,main.rs}`, `openspec/specs/{compose-instance,postgres-foundation}/spec.md`, and `docs/decisions/ADR-011-storage-tree-layout.md`.

<!-- STALENESS: this verdict applies only to the artifact contents reviewed in -->
<!-- this round. Any later edit to proposal.md, design.md, or specs/ (other than -->
<!-- applying listed Required Changes) VOIDS the verdict and requires a new round. -->

## Decision Under Review

**Decision:** Startup pairs the database with a managed store. It records a UUID in the existing `sidereal_metadata` row and in a versioned marker file at the storage root. Startup creates the pairing only when the database has no UUID and the root is empty or holds only a valid marker. It refuses every other state. The existing startup advisory lock serializes this work.

**Alternatives ruled out:** a separate initialization command (ADR-014, Alternative 1); identifying storage by path or drive (Alternative 2); keeping the identity only in configuration (Alternative 3); a pending/ready state or a second lock (design D2 and D3); and a separate pairing table (design D3). The user made these choices, so this review does not reopen them.

**Where it stops working (accepted limits, recorded by the author):** both persistence locations lost at once; an unpaired database adopting any marker-only store; two databases sharing one store; mount changes after startup.

## Findings

### 🔴 Critical (blocking)

None.

### 🟡 Moderate

**M1. A crash during the access probe can block first installation permanently. A probe that runs before the marker check also writes to a substitute disk.**

- **Concern:** The design says when the marker is created relative to the database commit (design.md:59-61). It never says when the probe runs, what it is named, or what happens to a leftover probe (design.md:109-110; managed-storage spec:120-125). The interruption analysis covers the marker only.
- **Scenario 1:** Startup runs against an unpaired database and an empty root. It creates the probe and fsyncs it. The container is then killed by an OOM kill, a `docker compose down` timeout, or a host reboot. On the next start the root holds a probe and nothing else. Rule 5 (design.md:56) refuses the start. So does "Unpaired storage contains other files" (managed-storage spec:50-54). The probe is a "pre-existing file", so startup will not remove it (design.md:110). The install stays refused until Sam deletes the file by hand.
- **Scenario 2:** The probe has a fixed name and is created exclusively. A leftover probe then makes exclusive creation fail on every later start, even on a paired store with a matching marker.
- **Scenario 3:** The probe runs before the pairing check. A paired database then meets an empty substitute directory, the case this change exists to catch. Every restart under `restart: unless-stopped` creates, syncs, and deletes a file on the wrong disk before refusing. The spec only says "creates no marker" (managed-storage spec:60), so this passes the spec. It still breaks ADR-014's goal of not writing to the wrong disk.
- **Scenario 4:** The probe runs outside the startup lock. A concurrent starter then sees another starter's probe and refuses under rule 5.
- **For this not to matter:** The design and spec must place the probe after the pairing rules succeed and after the UUID is committed or confirmed, inside the startup lock. Each probe needs a unique name. In that order, a leftover probe can only sit in a paired, matching store, where rule 1 ignores other contents (design.md:52). Refusals then write nothing.

**M2. A mixed-mode `down --volumes` breaks the pairing, and no supported recovery path exists.**

- **Concern:** The guide must say that `down --volumes` deletes Docker-managed volumes but never bind-mounted data (compose spec:91). The two locations can now be chosen separately (compose spec:133-143). So the documented deletion command can delete one location and keep the other.
- **Scenario:** Sam uses `compose.database-bind.yml` and keeps the default storage volume. Sam runs `docker compose down --volumes` and then starts again. The database still holds its UUID. Docker creates a new, empty storage volume. Rule 2 refuses on every start (design.md:53). The only guidance is mount recovery (managed-storage spec:139; compose spec:100), and nothing can be remounted. The design rules out any force-rebind option (design.md:62). Sam's only ways out are hand-written SQL or deleting the database. In this release, the lost store held only a marker.
- **For this not to matter:** The guide must warn that `down --volumes` with only one bind override deletes one location and breaks the pairing. It must also name the supported action when a paired store is permanently lost. Two such actions fit the design: restore both locations from copies taken together, or start again after removing both locations.

**M3. The PostgreSQL bind has no missing-mount protection, and the guide does not tell Sam how to get Compose's protection.**

- **Concern:** D1 relies on `create_host_path: false` to reject missing bind sources. It relies on the marker to catch an empty directory at a missing storage mount (design.md:33-34). The database has no marker. If `SIDEREAL_DATABASE_PATH` names the mount point itself and the disk fails to mount, the empty mount-point directory still exists. Compose mounts it, and the PostgreSQL image initializes a new cluster on the root filesystem.
- **Scenario:** The database disk fails to mount after a reboot. A new, unpaired database starts. In this release the store holds only a marker, so rule 4 adopts it silently (design.md:55). The instance looks healthy while the real database sits on the unmounted disk. A related case: a newly formatted ext4 mount root contains `lost+found`. A first start against that root refuses under rule 5. That refusal is safe but will confuse Sam.
- **For this not to matter:** The guide must tell Sam to create a dedicated subdirectory below each disk's mount point and to point both path settings at those subdirectories. An unmounted disk then removes the bind source, and Compose refuses before either service starts. This also avoids `lost+found`.

**M4. The verification recipe cannot reliably prepare or remove its own bind directories, and its refusal case can pass for the wrong reason.**

- **Concern:** D6 requires test-owned directories under `.workspace/`, keeps the existing cleanup checks, and adds a refusal case (design.md:123-129). The current cleanup runs `compose down --volumes` (scripts/instance-verify.sh:49). That command never removes bind-mounted data.
- **Scenario 1:** On rootful Linux Docker, the PostgreSQL entrypoint creates `18/` as root and gives `18/docker` to UID 999 with mode 0700. The host user who runs the recipe cannot delete those files. Cleanup therefore leaves data behind, or a later run fails on it.
- **Scenario 2:** A storage directory created by the host user (for example UID 1000, mode 0755) is not writable by UID 10001. In the refusal case, startup may then fail on the access check instead of the missing marker. The compose requirement only demands a refusal (compose spec:169). It would pass even though the missing-marker rule never ran.
- **For this not to matter:** The design must say how the recipe makes its storage directories writable by UID 10001 without host root. It must also say how the recipe removes container-owned data, for example with a disposable container limited to its own `.workspace` directories. The refusal case must check that the output names the missing marker.

**M5. Reading the marker can hang startup without any error message.**

- **Concern:** The spec accepts only "a regular non-symlink marker" and lists the invalid kinds (managed-storage spec:106, 117). The list leaves out FIFOs, sockets, and device files. An open-then-check implementation blocks forever when it opens a FIFO for reading.
- **Scenario:** Anyone who can write to the host storage directory creates a FIFO named `.sidereal-storage.json`. Startup blocks while it holds the startup advisory lock. The design sets no filesystem deadline (design.md:143). The process never exits and never logs an error. Docker restarts containers only when they exit, not when they are unhealthy, so the instance stays hung with no diagnostic. The storage directory is a trust boundary, and this input crosses it with no rule that covers it.
- **For this not to matter:** The design must require opening the marker without following symlinks and without blocking, then checking that the opened file is a regular file. The invalid-marker scenario must list FIFOs, sockets, and device files.

### 📌 Suggestions

1. **Scenario name contradicts its outcome.** In the compose spec, "A start leaves storage empty" (compose spec:77-80) now expects the marker in storage. Rename it to "A first start leaves only the marker".
2. **Upgrade scenario contradicts itself.** The postgres-foundation spec (lines 41-42) says the metadata "remain identical" and that the row "gains a null storage UUID". Say that `product` and `format_version` remain identical.
3. **Some scenario outcomes are not mechanically testable.**
   - "Mount-recovery guidance" (managed-storage spec:145) has no fixed text to check. Name a fixed phrase or error category.
   - "Without accessing a managed storage directory" (managed-storage spec:18) is hard to check. A testable form is: with `STORAGE_ROOT` unset, the server reaches readiness while the image's default storage path is absent or unreadable.
4. **Future risk: a paired database started without `STORAGE_ROOT` skips every storage check.** This is harmless now, because nothing writes to storage (managed-storage spec:12). Record that any later feature that writes storage must refuse when `STORAGE_ROOT` is unset.
5. **Directory sync may fail on some Docker Desktop or network-share bind mounts.** This is plausible but unconfirmed. On such a mount, every first start would refuse. The guide could say that a sync error marks unsupported storage, or the implementation could test Desktop mounts.
6. **ADR-014 wording.** Line 49 says "The proposed design uses…" in an accepted ADR. Change it to "This decision uses…". The manifest's review date (`adr.md:4`, 2026-10-16) is earlier than the 2026-10-19 acceptance it records.
7. **Second-instance guidance.** `docs/install.md:97` tells Sam how to run a second instance. With bind overrides, two instances can share one storage path, and the second unpaired database adopts the first store silently. The design accepts this limit (design.md:141). The guide's second-instance paragraph should warn about it.

### Checks Passed

- **Plain language.** No sentence in the proposal, design, specs, `adr.md`, or ADR-014 exceeds 30 words; the longest are about 26. Actors are named (Sam, a tester, the server, Compose). Terms stay consistent (marker, pairing, storage root, paired/unpaired). D2 repeats the spec's rules on purpose, as the source of truth for the design. That repetition is acceptable.
- **Scenario testability.** Most THEN clauses can be checked with a command, a database query, a byte comparison, or `docker inspect`. The exceptions are in Suggestion 3.
- **Scope vs. proposal.** No scope creep. The specs and design stay within the proposal. Ingestion, relocation tooling, mount monitoring, HTTP mutation, and multi-instance ownership are excluded everywhere. The deferred deployment tests match the proposal.
- **Design/spec contradictions.** None that block. The five D2 rules match the managed-storage requirements. Validating paths before connecting (design.md:49) is stricter than the spec's "before database writes", which is consistent. The access probe's placement is unspecified rather than contradictory (M1).
- **Claims checked against the repo.**
  - The startup lock is released right after migrations (`startup.rs:160-162`), as D3 says.
  - The identity check reads only `product, format_version` (`startup.rs:227-234`), so a nullable column does not affect it.
  - Schema baselines are chosen by migration count (`startup.rs:91-99, 248`), so baseline `2.json` and the "count is 2" updates are required and specified.
  - An older binary rejects the longer history (`startup.rs:236`), as the Migration Plan says.
  - The `Dockerfile` creates `/var/lib/sidereal/storage` owned by UID 10001, so a new named volume starts writable.
  - `main.rs` exits nonzero on errors, so a refusal never binds HTTP.
- **Security requirements.** Marker contents never become paths, commands, or SQL. Reads are limited to 4096 bytes. Pairing queries are parameterized. Paths in logs are escaped. Database credentials, URLs, marker bodies, and raw database errors stay redacted. Host paths never enter the app. No HTTP mutation is added, and the existing 405 rule still holds. Permission repair and privilege elevation are ruled out. The remaining gap at this trust boundary is M5.
- **Architecture constraints.** Filesystem logic goes in `core::storage` and environment wiring in `server`. The astro pack and `plugin-abi` are unchanged. No astronomy concepts enter `core`, and only PostgreSQL is used.
- **ADR hygiene.** ADR-014 is accepted, links only to ADR-007, and cites no tables, columns, issues, or milestones. Its marker at the storage root fits ADR-011's `store/` and `tmp/` layout.
- **Delta completeness.** The MODIFIED compose and postgres-foundation requirements carry the full original text with the intended edits. Removing "The server SHALL NOT read or write that folder in this change" is correct.

## Embedded-Instruction / Injection Attempts

**Detected:** none. The reviewed files contain no text that tries to direct the reviewer. Every "reviewer" mention is ordinary scenario text (for example, "a reviewer runs `grep`").

## Verdict

VERDICT: APPROVE_WITH_CHANGES

APPROVE WITH CHANGES. The plan is sound and the user's decisions are applied consistently. Five small, fully specified edits are needed before tasks are generated.

## Required Changes (if APPROVE WITH CHANGES)

1. **(M1) Probe order and naming.**
   - In `design.md` D5 (near line 109), add: "Startup runs the access probe only after the pairing rules succeed and the database UUID is committed or confirmed, while it still holds the startup lock. Each probe uses a unique name. A refused start creates no file."
   - In `specs/managed-storage/spec.md`, requirement "Storage access checks preserve operator data", add: "The server SHALL run the access probe only after pairing succeeds, and each probe SHALL use a unique name."
   - Add this scenario: "Leftover probe — GIVEN a paired store with a matching marker and a leftover probe file, WHEN Sam starts the instance, THEN the app reaches readiness and the leftover file remains."
   - In the scenario "Paired storage loses its marker", change "creates no marker" to "creates no file".
2. **(M2) Partial deletion and permanent loss.**
   - In `specs/compose-instance/spec.md`, add these two guide bullets:
     - "that `down --volumes` with only one bind override deletes one location, keeps the other, and leaves startup refusing";
     - "that when a paired store is permanently lost, the operator restores both locations from copies taken together or starts again after removing both locations."
   - Mention both in design.md D1 or the Risks section.
3. **(M3) Dedicated subdirectories.**
   - In `specs/compose-instance/spec.md`, add this guide bullet: "to point each path setting at a dedicated subdirectory below a disk's mount point, so an unmounted disk makes Compose refuse to start."
   - Add the same sentence to design.md D1.
4. **(M4) Verification directory handling.**
   - In design.md D6, add: "The recipe makes its storage bind directory writable by UID 10001 without host root. It removes container-owned bind data with a disposable container limited to its own `.workspace` directories. The refusal case checks that the output names the missing marker."
   - In the compose-instance "Deployment verification" requirement, add: "The refusal case SHALL confirm that startup failed because the marker is missing."
5. **(M5) Special-file markers.**
   - In design.md D4, add: "Open the marker without following symlinks and without blocking, then confirm the opened file is regular."
   - In `specs/managed-storage/spec.md`, scenario "Invalid marker", change the list to: "a symlink, directory, FIFO, socket, device file, oversized file, malformed JSON, unsupported version, or invalid required fields".

<!-- CANONICAL FIELD — machine-readable completion signal for APPROVE_WITH_CHANGES. -->
<!-- The AUTHOR sets this AFTER applying every required change and the reviewer -->
<!-- has re-checked them. Values: yes (all applied & re-checked) | no (outstanding) -->
<!-- | n/a (verdict is APPROVE or REVISE, no required changes). -->
<!-- Downstream work (tasks, apply) MUST NOT proceed on -->
<!-- VERDICT: APPROVE_WITH_CHANGES unless CHANGES_APPLIED: yes. -->

CHANGES_APPLIED: yes

## Rebuttals

### Author responses accepted by reviewer

- **M1 — CONFIRMED, fixed.** D5 and the access-check requirement place unique probes after committed or confirmed pairing, under startup serialization.
  Pairing refusals create no file; a leftover probe cannot block a matching paired store.
  The no-file promise concerns pairing refusals, not later probe failures that can occur after initial marker creation.
- **M2 — CONFIRMED for the reported direction, fixed with a qualification.** The guide contract and design explain mixed-mode deletion and permanent-loss recovery.
  Retained database plus deleted storage refuses; deleted database plus retained marker-only storage recovers under the accepted startup rule.
  Therefore the requested blanket statement that every one-bind deletion refuses would contradict D2.
  A retained populated store refuses adoption by a new database.
- **M3 — CONFIRMED, fixed.** D1 and the guide contract require dedicated subdirectories created only on the mounted disk.
  They explain why the missing source then makes Compose refuse the affected service.
- **M4 — CONFIRMED, fixed.** D6 specifies test-only helper containers limited to recorded run directories for ownership preparation and bind-data cleanup.
  The refusal check must identify the missing marker rather than accept any startup failure.
- **M5 — CONFIRMED, fixed.** D4 requires a nonblocking, no-follow marker open and regular-file verification.
  The invalid-marker scenario now includes FIFOs, sockets, and device files.

### Suggestions declined for this revision

1. Retain the existing scenario name because strict delta validation requires existing scenario identifiers during replacement.
   Its updated THEN clause explicitly expects only the marker.
2. Retain the upgrade wording: the first clause refers to saved existing metadata values, while the next describes the added null column.
   No implementation should interpret this as identical old and new row shapes.
3. Keep the outcome contract rather than fixing human-readable error wording or adding an operating-system access-tracing test.
   The missing-marker deployment check added for M4 identifies the failure category.
4. Future storage writers remain outside this change; the current scope explicitly excludes ingestion and object writes.
5. D1 already requires file and directory sync, and the risk section already limits storage to working sync semantics.
   Additional host-platform qualification remains deferred.
6. Keep the ADR's original date and manifest review date as historical dates.
   Renewed acceptance is recorded separately as 2026-10-19; the alternative's wording describes the selected design without changing its decision.
7. D6's multi-instance ownership exclusion and the risk section already require one instance per store.
   A broader second-instance guide revision remains deferred.


## Reviewer Re-check

- Date: 2026-10-19.
- Reviewer: Claude Opus 5.5, independent cross-model and read-only.
- Scope: Round 1 Required Changes M1-M5 and author responses.
- Reviewed fix commits: `5dae389` and `b5823d5`.
- Result: All five Required Changes are applied and re-checked.
- The reviewer confirmed that the edits stay within Required Changes and do not trigger the staleness rule.

### Accepted by reviewer

- **M1:** Unique probes run after committed or confirmed pairing, while holding the startup lock.
  The reviewer accepted limiting the no-file promise to pairing refusals.
- **M2:** Mixed-mode deletion and permanent-loss guidance cover both directions.
  The reviewer accepted the correction that a retained marker-only store can pair with a new database.
- **M3:** Dedicated subdirectories created only on the mounted disk let Compose reject missing sources.
- **M4:** Verification uses narrowly mounted test-only helpers and identifies the missing-marker diagnostic.
- **M5:** Nonblocking, no-follow opening rejects special-file markers before reading.

The reviewer accepted all five fixes and found no required defect hidden by the declined suggestions.
The verdict remains APPROVE_WITH_CHANGES, with every required change complete.
Implementation tasks are now unblocked.
