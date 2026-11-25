## Story

#529 forged-facet-refused — a plugin that writes a facet it may not write, or bypasses its context, is refused.
https://github.com/sidereal-io/sidereal/issues/529

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which of the story's refusals does #524 already test? | An undeclared facet, a facet not granted, a grant widened after registration, and counts below 0 or above the largest are each recorded failed with a reason that names the facet, and store nothing. | Evidence: `server/crates/core/src/operations/refuses.rs`, `openspec/specs/asset-facets/spec.md` |
| D2 | What does #529 add on top of #524? | Only the gaps: a test that a run's values never land on another asset or its versions, a core test that an Operator proposing a new asset is recorded failed and adds no asset, and a requirement that an Operator cannot target another asset. No repeat of #524's tests. | Assumption |
| D3 | How is "a facet for an asset that is not their input" covered, given that `propose_facet` takes only a facet name and a value? | A requirement that the Asset context gives no way to name a target, proven by a runtime test that a run's values land only on its own input while other assets and versions exist. No `compile_fail` doctest. | Assumption |
| D4 | Does #529 cover an Operator that changes its stored input on disk during a run? | No. It is new core behavior outside this story's scope, so it goes to a new issue under epic #356. | Assumption |

## Changes from the story

- **Cut:** repeating the undeclared-facet and bad-value refusal tests. #524 already tests them in core (D1).
- **Cut:** one named back-door suite that runs every refusal through the upload path. It would repeat #524's coverage (D2).
- **Cut:** a `compile_fail` doctest for naming another asset. It passes on any compile error, so it proves little (D3).
- **Split out:** core rechecking an Operator's input after a run, and recording a changed input as a failed run. New issue under epic #356, to be filed once its packet is confirmed (D4).

Confirmed by user: yes
