# Code review: use-existing-postgres

Claude Opus 5.5 independently reviewed commit `b873759` against `origin/main` (`ee11567`).
GPT authored the implementation. This was one review round, with no gate reruns by the reviewer.
The author checked all four findings against the code and confirmed them.

## Findings and fixes

1. **Medium — interrupted cleanup cannot recover. CONFIRMED.**
   Cleanup depended on old generated files, which an interrupted run could leave incomplete.
   Fix `d9d92aa` reconstructs settings before cleanup and gives missing-URL checks a separate environment file.
   Both reproduced states recover automatically: an environment file without a URL and an ownership record without generated files.

2. **Medium — Compose guarantees lack committed regression checks. CONFIRMED.**
   App-setting equivalence and external database-bind rejection were checked only by an ignored working script.
   Fixes `deeff30` and `1ab2b2f` add both checks to the deployment script, including its `--config-only` mode.
   The checks support both Compose YAML list formats.
   Deliberately changing the external restart policy, removing required capabilities, or making the database bind override valid causes the corresponding check to fail.

3. **Low — certificate guidance implies bundled configurability. CONFIRMED.**
   The bundled URL fixes plaintext transport and cannot select certificate paths from `.env`.
   Fix `cc2953b` scopes certificate guidance to external mode and clarifies the design.

4. **Low — password table implies both modes require the bundled password. CONFIRMED.**
   The shared settings table contradicted the external installation instructions.
   Fix `f901c9b` marks `POSTGRES_PASSWORD` as required only by bundled mode.

Each fix commit was inspected and changes only its finding's scope.
There are no refuted or deferred findings.

## Validation

- Interrupted-state reproductions and configuration mutation checks pass after the fixes.
- Full `deli -- just external-db-verify` passes after the fixes, including cleanup.
- The committed configuration checks pass on installed Compose 5.6.0 and official Compose 2.20.0.
- The existing full project gate and bundled deployment demonstration passed before review.
  The fixes change no server, web, or bundled runtime behavior.

The PR remains draft until OpenSpec verification and archive are complete.
