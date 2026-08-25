---
name: spec-critique-claude
description: Run the independent adversarial spec critique using Claude Code as the reviewer. Use this when the spec was authored by a DIFFERENT toolset (Codex or Gemini) and you want Claude — a distinct model family with no access to the authoring session — to critique it. Drives the `claude` CLI headless in read-only (plan) mode against a spec. Triggers: "critique this spec with Claude", "spec review", "get an independent review", or before writing-plans on a Codex- or Gemini-authored spec that has no recorded critique.
---

# Spec Critique via Claude CLI

Independent review is the highest-ROI quality step in this process. The property that makes it work is **independence by toolset**: the reviewer must be a different model family than the one that wrote the spec, or it converges on confirming the author's reasoning. Use this skill when **Codex or Gemini authored the spec** and you want **Claude** to review it. (If Claude authored the spec, do not use this — route to `spec-critique` (Codex) or `spec-critique-antigravity` instead.)

The `claude` CLI gives independence for free here: fresh context, read-only access to the real repo so it checks the spec's claims against the code instead of taking them on faith. Claude Code auto-loads this repo's house rules (`CLAUDE.md`), so don't restate them in the prompt.

The critic critiques; it does not rewrite. You (with the user) triage its findings and revise the spec yourself.

## Step 1 — Identify the spec

The spec is a GitHub issue body — use the issue number the user gave, or the issue under discussion. If ambiguous, ask rather than guess. Pull it to the scratchpad:

```bash
gh issue view <n> --json body --jq '.body' > .workspace/issue-<n>.md
```

## Step 2 — Scale scrutiny to complexity

Score the spec's *proposed* complexity — what it would change, not how long the file is. Pick the highest row that matches; when torn between two, take the higher.

| Tier | When it fits | How to run |
|---|---|---|
| `low` | Narrow fix or polish; one package; no new contracts, schema, events, or authz surface | foreground, default model |
| `medium` | Standard single-subsystem slice on established patterns; contained blast radius | foreground, `--model opus` |
| `high` | Multi-package; new contracts/events/migrations; authz or trust boundaries; architecture-changing (new-ADR territory); novel external dependency | background, `--model opus`, ask for deeper analysis in the prompt |

Claude Code has no CLI "reasoning effort" flag; scale scrutiny by model tier, by running in the background for long high-complexity reviews, and by how hard the prompt tells it to dig. State the chosen tier and the one-line reason before running.

## Step 3 — Run the critique

Write the prompt to the scratchpad, then run from the repo root. `--permission-mode plan` makes the session read-only (it can read and search the repo but cannot edit). The critique is Claude's final message on stdout — redirect it to a file:

```bash
claude -p --permission-mode plan --model opus \
  < .workspace/critique-prompt.md \
  > .workspace/spec-critique-claude-raw.md
```

Prompt template (adjust the bracketed parts only):

```
You are an independent design reviewer. Adversarially critique the specification in
the body of GitHub issue #<n> (paste from .workspace/issue-<n>.md), checking its claims
against the actual code in this repository (you have read-only access — verify, don't
assume).

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

If the `claude` CLI is unavailable or the run errors out, fall back to `spec-critique-antigravity` (the Antigravity/`agy` Gemini reviewer is the last-resort reviewer when the primary non-author toolset fails).

## Step 4 — Record and triage

1. **Verify every finding before accepting it.** Record the outcome per finding: CONFIRMED, REFUTED (name the contradicting evidence), or OUT-OF-SCOPE (say why).
2. Summarize the findings to the user ranked by severity, with your own accept/reject recommendation per finding — the critic has no product context, so some findings are legitimately out of scope; say why when rejecting.
3. Revise the issue body for accepted findings, and note at the top: `Critiqued <date> (claude/opus, <tier>): N findings, M accepted — see critique comment.` A spec with all blocker/major findings resolved (fixed or explicitly rejected with reasons) is ready for `writing-plans`.
4. Post the critique to the issue as a comment (`gh issue comment <n> --body-file .workspace/spec-critique-claude-raw.md`).
