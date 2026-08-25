---
name: spec-critique
description: Run the independent adversarial spec critique before any implementation plan is written. Use whenever a spec is newly written or materially revised, whenever the user says "critique this spec", "spec review", or asks whether a design is ready to plan.
---

# Spec Critique via Codex CLI

Independent review is the highest-ROI quality step in this process. The property that makes it work is **independence**: the critic must not inherit the authoring session's reasoning, or it converges on confirming it. Codex CLI gives that for free — different model family, fresh context, read-only access to the real repo so it can check the spec's claims against the code instead of taking them on faith.

The critic critiques; it does not rewrite. You (with the user) triage its findings and revise the spec yourself.

## Step 1 — Identify the spec

The spec is a GitHub issue body — use the issue number the user gave, or the issue under discussion. If ambiguous, ask rather than guess. Pull it to the scratchpad for the critic to read alongside the live repo:

```bash
gh issue view <n> --json body --jq '.body' > .workspace/issue-<n>.md
```

## Step 2 — Scale effort to complexity

Read the spec and score its *proposed* complexity — what it would change, not how long the file is. Pick the highest row that matches; when torn between two, take the higher (an under-reviewed design costs far more than extra reasoning tokens). `high` is the ceiling — never use `xhigh`:

| Effort | When it fits |
|---|---|
| `low` | Narrow fix or polish; one package; no new contracts, schema, events, or authz surface |
| `medium` | Standard single-subsystem slice on established patterns; contained blast radius |
| `high` | Multi-package; new contracts/events/migrations; authz or trust boundaries; architecture-changing (new-ADR territory); novel external dependency |

State the chosen effort and the one-line reason before running.

## Step 3 — Run the critique

Write the prompt to the scratchpad in `.workspace`, then run from the repo root:

```bash
codex exec -s read-only -m gpt-5.6-sol \
  -c model_reasoning_effort="<effort>" \
  -o .workspace/spec-critique-raw.md \
  - < .workspace/critique-prompt.md
```

Prompt template (adjust the bracketed parts only):

```
You are an independent design reviewer. Adversarially critique the specification in the body of GitHub issue #<n> (paste from .workspace/issue-<n>.md), checking its claims against the actual code in this repository (you have read-only access — verify, don't assume).

Hunt specifically for: incorrect assumptions about the existing system; missing edge cases; security/authorization gaps; state or migration hazards; simpler alternatives that deliver the same guarantees; anything that would fail in production or during implementation. Durable context: `docs/decisions/` (accepted and proposed ADRs) and `docs/architecture/README.md` (the architecture map and Design facts).

Do NOT rewrite the spec or produce code. Output ranked findings, most severe first, each with: severity (blocker/major/minor), the spec section it targets, the evidence you verified in the repo, and what question the spec must answer. If the design is sound, say so plainly rather than inventing objections.
```

Run it in the background (`run_in_background`) for `high` — it can take several minutes — and foreground with a generous timeout (600000) otherwise. This costs real tokens; never loop it or re-run without a changed spec or a changed question.

## Step 4 — Record and triage

1. **Verify every finding before accepting it.** Check each claim against the file, doc, or behavior it cites; a "likely" asserted over something checkable is a question, not a finding. Record the outcome per finding: CONFIRMED, REFUTED (name the contradicting evidence), or OUT-OF-SCOPE (say why).
2. Summarize the verified findings to the user ranked by severity, with your own accept/reject recommendation per finding — the critic has no product context, so some findings are legitimately out of scope; say why when rejecting.
3. Revise the issue body for accepted findings, and note at the top: `Critiqued <date> (gpt-5.6-sol, <effort>): N findings, M accepted — see critique comment.` A spec with all blocker/major findings resolved (fixed or explicitly rejected with reasons) is ready for `writing-plans` and the human approval gate (`status/design` → `status/ready`).
4. Post the critique to the issue as a comment (`gh issue comment <n> --body-file .workspace/spec-critique-raw.md`).
