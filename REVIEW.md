# Review rules

These rules add to your usual review. `AGENTS.md` already gives the repo's
invariants, so they are not repeated here. Report a finding only when you can
point to the file and line that shows it.

## Change fit

If the branch adds or changes a folder under `openspec/changes/` (not under
`archive/`), that folder holds the plan for this code. Read its `proposal.md`,
`design.md` and `tasks.md`. Report:

- code that contradicts a decision in `design.md`
- code that does work outside the change's scope, as the proposal states it

## Tests

Report new or changed behavior that no test covers. A test covers it only if the
test would fail without the change. Do not ask for tests of code the branch did
not touch.

## Simplicity and reuse

Report:

- logic that is more complex than the problem needs, with the simpler form
- new code that repeats something the repo already has, with where it lives
- names that would mislead a reader who does not know this change

Do not suggest speculative abstractions.
