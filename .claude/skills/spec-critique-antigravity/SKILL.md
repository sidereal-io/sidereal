---
name: spec-critique-antigravity
description: Run the independent adversarial spec critique using the Antigravity CLI (`agy`) on its top Gemini model as the reviewer. This is the LAST-RESORT reviewer — reach for it when the preferred non-author toolset is unavailable or errors: e.g. Claude authored the spec and Codex is down, or Codex authored and Claude is down. Also valid as a primary when the Gemini family did not author the spec and you want its distinct point of view. Drives `agy` headless in read-only (plan) mode. Triggers: "critique this spec with Antigravity/agy/Gemini", "the other reviewer failed", "spec review fallback".
---

# Spec Critique via Antigravity CLI (`agy`)

Independent review is the highest-ROI quality step in this process. The property that makes it work is **independence by toolset**: the reviewer must be a different model family than the one that wrote the spec, or it converges on confirming the author's reasoning.

This is the **fallback reviewer**, running the Gemini model family via Antigravity's `agy` CLI (Google's `gemini` CLI is deprecated — do not use it). Prefer Codex (`spec-critique`) or Claude (`spec-critique-claude`) as the primary non-author reviewer; use this skill when that primary is unavailable or its run fails, or as a legitimate primary when the Gemini family did not author the spec. Never use it to review a spec that a Gemini model authored.

`agy` does not automatically load this repo's `CLAUDE.md` house rules — so the prompt below points it at the durable context explicitly and tells it to read `CLAUDE.md` first.

The critic critiques; it does not rewrite. You (with the user) triage its findings and revise the spec yourself.

## Step 1 — Identify the spec

The spec is a GitHub issue body — use the issue number the user gave, or the issue under discussion. If ambiguous, ask rather than guess. Pull it to the scratchpad:

```bash
gh issue view <n> --json body --jq '.body' > .workspace/issue-<n>.md
```

## Step 2 — Scale scrutiny to complexity

Score the spec's *proposed* complexity — what it would change, not how long the file is. Pick the highest row that matches; when torn between two, take the higher. `agy` has no separate reasoning-effort flag — the reasoning tier is baked into the model name (`(Low)` / `(High)`), so scale by model tier plus timeout and background.

| Tier | When it fits | Model / run |
|---|---|---|
| `low` | Narrow fix or polish; one package; no new contracts, schema, events, or authz surface | `Gemini 3.1 Pro (Low)`, foreground |
| `medium` | Standard single-subsystem slice on established patterns; contained blast radius | `Gemini 3.1 Pro (High)`, foreground |
| `high` | Multi-package; new contracts/events/migrations; authz or trust boundaries; architecture-changing; novel external dependency | `Gemini 3.1 Pro (High)`, background, `--print-timeout 20m` |

The official model is **`gemini-3.1-pro`**, but `agy`'s `--model` flag does not accept that id — it wants its own display string, and `Gemini 3.1 Pro (High)` selects `gemini-3.1-pro` at its top reasoning tier. Confirm the current display strings with `agy models`. State the chosen tier and one-line reason before running.

## Step 3 — Run the critique

Write the prompt to the scratchpad, then run from the repo root. `--mode plan` is `agy`'s read-only mode (it can read and search the repo but cannot edit). The prompt is the **value of `-p`** and must come after the other flags. Redirect stdout to a file:

```bash
agy --mode plan --model "Gemini 3.1 Pro (High)" --print-timeout 20m \
  -p "$(cat .workspace/critique-prompt.md)" \
  > .workspace/spec-critique-antigravity-raw.md
```

Prompt template (adjust the bracketed parts only):

```
You are an independent design reviewer with read-only access to this repository. First
read CLAUDE.md for the project's house rules. Then adversarially critique the
specification in the body of GitHub issue #<n> (paste from .workspace/issue-<n>.md),
checking its claims against the actual code (verify, don't assume).

Hunt specifically for: incorrect assumptions about the existing system; missing edge
cases; security/authorization gaps; state or migration hazards; simpler alternatives
that deliver the same guarantees; anything that would fail in production or during
implementation. Durable context: docs/decisions/ (accepted and proposed ADRs),
docs/architecture/README.md (the architecture map and Design facts).

Do NOT rewrite the spec or produce code or a plan. Output ranked findings, most severe
first, each with: severity (blocker/major/minor), the spec section it targets, the
evidence you verified in the repo, and what question the spec must answer. If the design
is sound, say so plainly rather than inventing objections.
```

Run it in the background (`run_in_background`) for `high` — it can take several minutes — and foreground with a generous timeout (600000) otherwise. This costs real tokens; never loop it or re-run without a changed spec or a changed question.

## Step 4 — Record and triage

1. **Verify every finding before accepting it.** Record the outcome per finding: CONFIRMED, REFUTED (name the contradicting evidence), or OUT-OF-SCOPE (say why).
2. Summarize the findings to the user ranked by severity, with your own accept/reject recommendation per finding — the critic has no product context, so some findings are legitimately out of scope; say why when rejecting.
3. Revise the issue body for accepted findings, and note at the top: `Critiqued <date> (agy/Gemini 3.1 Pro, <tier>): N findings, M accepted — see critique comment.` A spec with all blocker/major findings resolved (fixed or explicitly rejected with reasons) is ready for `writing-plans`.
4. Post the critique to the issue as a comment (`gh issue comment <n> --body-file .workspace/spec-critique-antigravity-raw.md`).
