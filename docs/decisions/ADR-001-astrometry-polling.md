# 001: Astrometry submission polling

**Status:** Accepted
**Date:** 2026-10-06
**Context:** [Issue #309](https://github.com/sidereal-io/sidereal/issues/309), [consolidated approved specification](https://github.com/sidereal-io/sidereal/issues/309#issuecomment-6018047300), targeting `v0.x` on `issue-309-astrometry-polling`.

## Problem

Worker/manual and synchronous polling misinterpret null Astrometry.net job placeholders and truthy unset `processing_finished` strings as failure. Submission acceptance and solver completion have separate lifecycles. A usable remote ID allows monitoring; terminal success requires usable calibration.

The synchronous 720-attempt budget limits caller waiting, not upstream processing. Exhaustion must remain nonterminal with useful UI feedback. Individually checking a false historical failure must inspect its existing submission and let worker polling resume if it is still waiting. Current storage stamps completion time on updates, so the UI must use terminal status when showing completion dates.

The actual single-image consumer in `apps/client/src/components/image-modal.tsx` currently calls `/api/images/:id/plate-solve`; the registered route is `/api/plate-solving/images/:id/plate-solve`. Necessary consumer integration uses the canonical route and handles HTTP 202; no general route aliases are proposed.

## Options

### Option A: Remove the null-failure branches only

Smallest diff and directly fixes placeholders, but leaves duplicated selection/validation and cannot fulfill the approved exhaustion, recovery, and presentation requirements.

### Option B: Shared server-local policy with explicit outcomes and scoped caller/UI integration

One-check `PollObservation` remains separate from the result-bearing direct poll outcome:

```ts
type PollObservation =
  | { status: 'processing'; jobId: string | null }
  | { status: 'success'; jobId: string }
  | { status: 'failed'; jobId: string | null; error: string };
type PlateSolvingPollOutcome =
  | { status: 'success'; result: PlateSolvingResult; remoteJobId: string }
  | { status: 'failed'; error: string; remoteJobId: string | null }
  | { status: 'processing'; remoteJobId: string | null };
type PlateSolvingWorkflowOutcome =
  | { status: 'success'; result: PlateSolvingResult }
  | { status: 'processing'; jobId: number; submissionId: string;
      message: 'Still processing. Check status later.' };
```

Policy accepts positive safe-integer IDs, skips invalid placeholders and does not use processing timestamps as terminal evidence. Select first usable calibrated pair's job (first element), otherwise retain selected job or choose first usable submission job. Valid calibration advertisement takes precedence over stale selected-job/submission failure, but usable results must be retrieved before success persistence. Explicit submission error, selected-job failure and valid selected-job HTTP 404 are failures. Request/parse/calibration errors are check errors, not terminal evidence.

Direct polling preserves 720 attempts, configured intervals, sleep-after-attempt and retryable errors. Success carries validated result/remote ID; explicit failure carries its error; exhaustion carries processing and selected ID. Workflow success preserves existing completion, failure preserves error handling, and processing returns existing local job/submission IDs. Single-image route emits HTTP 202 with exact processing message and keeps existing HTTP 200 success shape. Actual consumer uses canonical route, shows the nonterminal message and refreshes data without claiming solved/failed.

Manual update calls `checkJobStatus(jobId, { resumeProcessing: true })`. On a nonterminal check of a previously failed record, persist processing and clear stale result so normal worker selection resumes. Success recovers false failures with usable calibration; explicit failures stay failed; check errors leave all prior stored state intact. Default worker checks do not opt into historical repair. Checks never upload or create another local/remote job.

Page stage labels infer Waiting for Astrometry.net versus Solving from valid remote ID, without adding persisted states. Processing details include submission link/time; annotated-result links require valid remote ID and completion dates require terminal status. Per-record Check status uses existing POST update endpoint, disabled Checking… while pending, and distinct request-error feedback. Jobs query refreshes every 30 seconds while pending/processing exists and on focus; observed completion/manual results refresh images/stats. This works independently of the current ref-based socket hook.

**Pros:** Shared interpretation and focused real-caller regressions; expresses exhaustion correctly; scoped recovery repairs individual false failures without duplicate uploads; accurate UI without schema/dependency changes.

**Cons:** Larger service/route/page/consumer/test diff; one selected remote job only; no worker deadline means indefinitely waiting submissions can retain concurrency slots; query polling adds read requests while work is active.

### Option C: New queue/state machine and broad recovery framework

Could introduce waiting/running states, deadlines, cancellation and multi-job tracking. It exceeds this maintenance issue and requires separate persistence, timeout, retry and migration decisions.

## Recommendation

Choose Option B under the consolidated approved specification. Keep existing Zod/dependencies, status vocabulary/schema, upload path and worker scheduling. The direct budget is not a wall-clock deadline because checks take request time. No request timeout settings, worker deadlines/cancellation, automatic re-upload, historical bulk repair, multi-job aggregation, generic socket/UI refactor or global timestamp-policy change.

Tests follow Red-Green-Refactor for policy, service/route outcomes and recovery, then actual consumer/page POM behavior. Include refresh/check no-upload assertions, terminal-date and valid-link presentation, per-record pending/error states, timer/focus recovery and compatibility of successful route responses. All heavy commands use `deli -- <command>`; typecheck after each task. Baseline 91 tests/typecheck passed; dependencies unchanged.

## Decision

The developer accepted this work on 2026-10-06. Choose Option B with the conditions in this record and the consolidated approved specification. Implementation may proceed through the approved plan using subagent-driven execution, targeting `v0.x`.
