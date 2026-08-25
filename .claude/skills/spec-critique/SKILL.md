---
name: spec-critique
description: Run the independent adversarial spec critique before any implementation plan is written. Portable across harnesses — establishes who authored the spec and dispatches the review to a different model family. Use whenever a spec is newly written or materially revised, whenever the user says "critique this spec", "spec review", or asks whether a design is ready to plan.
---

# Spec Critique

Independent review is the highest-ROI quality step in this process. What makes it work is
**independence by model family**: a critic that inherits the author's reasoning just confirms it.

Two things not to assume. **You are not necessarily the author** — someone else may have written the
spec and pushed it before you saw it. And **a CLI is not a family** — independence is a property of
the model that answers, not the tool that launches it.

Three families here, one executor each: `gpt` via `codex`, `claude` via `claude`, `gemini` via `agy`.
No cross-running — if a family's CLI is unavailable, so is that family, and you fall to the next one.

The critic critiques; it does not rewrite. You and the user triage the findings and revise the spec.

## Step 1 — Pull the spec

A GitHub issue body. Use the number the user gave, or the issue under discussion; if ambiguous, ask.

```bash
gh issue view <n> --json body --jq '.body' > .workspace/issue-<n>.md
gh issue view <n> --json labels --jq '.labels[].name'
```

## Step 2 — Establish the author family

First of these that answers:

1. The provenance block in the issue body (Step 6 writes it) — authoritative.
2. An `authored-by/*` label.
3. This session, if you wrote or materially revised the spec here.
4. Ask. Don't guess — a wrong answer turns this whole step into a rubber stamp, quietly.

A human author unaided frees every family. But if you then shaped the spec — the usual case out of
`writing-specs` — exclude your own family too; you'd be reviewing your own reasoning.

State the author family and how you know.

## Step 3 — Pick the reviewer family

Order is **`gpt` → `claude` → `gemini`**. Take the first that isn't the author's family and whose
executor is available (`command -v`, and not out of credit).

| Author | Reviewer | Then |
|---|---|---|
| `claude` | `gpt` | `gemini` |
| `gpt` | `claude` | `gemini` |
| `gemini` | `gpt` | `claude` |
| human, unaided | `gpt` | `claude`, then `gemini` |

`gemini` sits last because its executor needs house rules pointed at explicitly and folds reasoning
tier into model choice. Still fine as a primary when you specifically want its perspective.

**If every executable family is the author's, stop** and say so. No review beats a self-review
wearing an independence label.

## Step 4 — Scale scrutiny

Score what the spec would *change*, not its length. Torn between two rows, take the higher.

| Tier | Fits |
|---|---|
| `low` | Narrow fix or polish; one package; no new contracts, schema, events, or authz surface |
| `medium` | Standard single-subsystem slice on established patterns; contained blast radius |
| `high` | Multi-package; new contracts/events/migrations; authz or trust boundaries; architecture-changing (new-ADR territory); novel external dependency |

Always use the reviewer's **strongest available reasoning model** — this step is worth the tokens.
Scale the reasoning dial (`low`/`mid`/`highest`) and the run shape: foreground for `low` and
`medium`, background with a long timeout for `high`, plus a nudge in the prompt to dig deeper.

Only Codex exposes reasoning independently of model choice. Where the tier is folded into the model
name, pick the matching variant — and **discover the current list rather than trusting a name written
here** (`agy models`), since model ids rot fast.

State the tier and a one-line reason.

## Step 5 — Execute the family

`agy models` also lists Claude and GPT models — ignore them and pick a Gemini one. Running another
family there would change which CLI launched the review, not who actually did it.

Prompt goes in `.workspace/critique-prompt.md`; run from the repo root. All recipes are read-only.

```
You are an independent design reviewer with read-only access to this repository.
[tools that don't auto-load house rules: First read AGENTS.md for the project's house rules.]
Adversarially critique the specification in the body of GitHub issue #<n> (pasted at
.workspace/issue-<n>.md), checking its claims against the actual code in this repository
(verify, don't assume).

Hunt specifically for: incorrect assumptions about the existing system; missing edge cases;
security/authorization gaps; state or migration hazards; simpler alternatives that deliver the
same guarantees; anything that would fail in production or during implementation. Durable
context: `docs/decisions/` (accepted and proposed ADRs) and `docs/architecture/README.md`
(the architecture map and Design facts).

Do NOT rewrite the spec or produce code or a plan. Output ranked findings, most severe first,
each with: severity (blocker/major/minor), the spec section it targets, the evidence you
verified in the repo, and what question the spec must answer. If the design is sound, say so
plainly rather than inventing objections.
```

That house-rules line is conditional: codex reads `AGENTS.md` and claude reads `CLAUDE.md` (symlinked
to it here) on their own; `agy` reads neither.

```bash
# codex — -s read-only sandboxes it, reasoning is its own dial
codex exec -s read-only -m <strongest-model> -c model_reasoning_effort="<tier>" \
  -o .workspace/spec-critique-raw.md - < .workspace/critique-prompt.md

# claude — plan mode is read-only; critique is the final message on stdout
claude -p --permission-mode plan --model <highest-reasoning-tier> \
  < .workspace/critique-prompt.md > .workspace/spec-critique-raw.md

# agy — plan mode is read-only; prompt is the VALUE of -p and must come last
agy --mode plan --model "<a Gemini model, from agy models>" --print-timeout <timeout> \
  -p "$(cat .workspace/critique-prompt.md)" > .workspace/spec-critique-raw.md
```

Costs real tokens — never loop it, never re-run without a changed spec or a changed question. If the
executor errors or is out of credit, fall to the next family in Step 3 and note the substitution.

## Step 6 — Record and stamp

1. **Verify every finding before accepting it.** A "likely" asserted over something checkable is a
   question, not a finding. Mark each CONFIRMED, REFUTED (name the contradicting evidence), or
   OUT-OF-SCOPE (say why).
2. Summarize to the user by severity with an accept/reject rec each — the critic has no product
   context, so some findings are legitimately out of scope.
3. Revise the issue body, then update the provenance block at its top. Step 2 reads this next time,
   so record what *actually* ran, fallback substitutions included:

   ```markdown
   <!-- provenance -->
   **Authored:** <date> · <family> (<tool/model>)
   **Critiqued:** <date> · <family> (<tool/model>, <tier>) — N findings, M accepted
   ```

   All blocker/major findings resolved — fixed or explicitly rejected with reasons — means ready for
   `writing-plans` and the approval gate (`status/design` → `status/ready`).
4. Mirror onto labels for cross-issue queries: `authored-by/<family>`, `reviewed-by/<family>`,
   following the repo's `category/value` convention. If a label doesn't exist, say so and let the
   user create it rather than minting labels on a shared repo. The body block stays authoritative;
   labels are just an index.
5. `gh issue comment <n> --body-file .workspace/spec-critique-raw.md`
