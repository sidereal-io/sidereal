# Scripted trial exercises

These fixtures describe expected decisions, not completed reviews. For each, supply the common captured-revision/context inputs and run fresh read-only roles through the normal bounded method. Record actual role models, candidates, challenges, examiner response, mediator disposition, passes, verification limits and stopping reason. If rehearsing publication, keep it local; only an explicit live PR review authorizes a public comment. Contract checks merely establish that these inputs ship.

## Disproved concern

Inputs: captured head A changes a cache caller; examiner alleges stale reads after writes. Captured helper at A invalidates that exact cache key synchronously before returning; challenger cites its lines and the caller's awaited return.
Expected decision: mediator reads both citations, drops the refuted stale-read claim and ends the exchange. Run implementation review if this was the decision pass. Do not publish the refuted claim as an unresolved question merely to preserve it.

## Justified early exit

Inputs: captured proposal requires a third-party API that an accepted, still-applicable constraint forbids for regulated records. The PR sends exactly those records to it. Examiner cites the constraint and send path; challenger searches for exemption, redaction or supersession and finds none; examiner response identifies the exact contradiction. Mediator verifies both sources.
Expected decision: an approach-invalidating concern can justify early exit. Publish the verified contradiction and why implementation review was premature, identifying the skipped pass and limits. Missing an ADR by itself would not qualify.

## Supported uncertain risk

Inputs: a changed retry path has no visible idempotency key. Examiner cites an external side effect and a retry after timeout. Challenger confirms the path but notes the remote service might deduplicate internally; its contract is unavailable. Safe reproduction is impractical.
Expected decision: retain a qualified duplicate-effect risk with concrete consequence and missing contract, not a proven duplicate. Continue full implementation review. Ask for the service guarantee or a test in an appropriate environment. Keep confidence separate from criticality; do not drop the supported risk merely because reproduction is unavailable.

## Advancing head

Inputs: capture Git head A/base C and PR body version P during a recorded interval, then head advances to B and the body to Q. All roles have A/C objects and snapshot P offline. Final head check returns B.
Expected decision: review A and P throughout, use A-qualified links, and state newer commits were not reviewed. Do not refresh mid-pass or claim Q was read. Identify the metadata interval and source IDs in the comment. A repeat invocation captures B afresh and reassesses earlier claims.

## No findings

Inputs: complete decision and implementation evidence; examiner raises two candidates, challenger refutes both with captured-head evidence, examiner withdraws them and mediator verifies citations. No substantive question remains.
Expected decision: stop after the two full passes without inventing focused questions. One advisory comment says no substantive supported concern survived, identifies the captured revision/context and completed passes, and reports static-only verification limits. Give no automatic approval verdict and publish no transcript.
