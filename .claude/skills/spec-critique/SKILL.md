---
name: spec-critique
description: Run the independent adversarial spec critique before any implementation plan is written. Portable across harnesses — detects which toolset is running it and dispatches to a different model family for the review. Use whenever a spec is newly written or materially revised, whenever the user says "critique this spec", "spec review", or asks whether a design is ready to plan.
---

# Spec Critique

Independent review is the highest-ROI quality step in this process. The property that makes it work
is **independence by toolset**: the critic must not inherit the authoring session's reasoning, or it
converges on confirming it. A different model family, with fresh context and read-only access to the
real repo, checks the spec's claims against the code instead of taking them on faith.

**You are the author.** Whatever harness is executing this skill wrote (or is shepherding) the spec,
so it cannot be the reviewer. Steps 1–2 exist to route the work to someone else.

The critic critiques; it does not rewrite. You (with the user) triage its findings and revise the
spec yourself.

## Step 1 — Identify yourself

Name your own model family — you know which harness you are running in. If genuinely unsure, probe:

| Signal | Family |
|---|---|
| `$CLAUDECODE` is `1`, or `$CLAUDE_CODE_ENTRYPOINT` is set | `claude` |
| `$CODEX_*` env vars present, or the session loaded `AGENTS.md` as its own house rules with no `CLAUDE.md` | `codex` |
| Running under Antigravity | `agy` (Gemini family) |
| None of the above | `other` — treat every reviewer below as eligible |

State which you are before continuing. Getting this wrong silently destroys the whole point of the
step: a spec reviewed by its own family is a rubber stamp.

## Step 2 — Pick the reviewer

Preference order is **`codex` → `claude` → `agy`**. Walk it top to bottom and take the first entry
that is *not your own family* and *is available* (`command -v <tool>` succeeds, and it is not out of
credit). That resolves to:

| You are | Reviewer | Then | Last resort |
|---|---|---|---|
| `claude` | `codex` | `agy` | — |
| `codex` | `claude` | `agy` | — |
| `agy` | `codex` | `claude` | — |
| `other` | `codex` | `claude` | `agy` |

`agy` sits last everywhere because it needs the house rules pointed at explicitly (below) and has no
independent effort dial. It is still a legitimate *primary* when the Gemini family did not author the
spec and you specifically want its point of view — the order is a default, not a prohibition.

**If the only available tool is your own family, STOP** and tell the user. Do not review your own
spec and label it independent; a self-review is worth less than an honest "no reviewer available."

State the reviewer and why it won before running.

## Step 3 — Identify the spec

The spec is a GitHub issue body — use the issue number the user gave, or the issue under discussion.
If ambiguous, ask rather than guess. Pull it to the scratchpad for the critic to read alongside the
live repo:

```bash
gh issue view <n> --json body --jq '.body' > .workspace/issue-<n>.md
```

## Step 4 — Scale effort to complexity

Read the spec and score its *proposed* complexity — what it would change, not how long the file is.
Pick the highest row that matches; when torn between two, take the higher (an under-reviewed design
costs far more than extra reasoning tokens). `high` is the ceiling — never go above it.

| Tier | When it fits |
|---|---|
| `low` | Narrow fix or polish; one package; no new contracts, schema, events, or authz surface |
| `medium` | Standard single-subsystem slice on established patterns; contained blast radius |
| `high` | Multi-package; new contracts/events/migrations; authz or trust boundaries; architecture-changing (new-ADR territory); novel external dependency |

Only Codex has a real reasoning-effort dial. The other two scale by model tier plus how hard the
prompt pushes:

| Tier | `codex` | `claude` | `agy` |
|---|---|---|---|
| `low` | `model_reasoning_effort=low` | default model, foreground | `Gemini 3.1 Pro (Low)`, foreground |
| `medium` | `model_reasoning_effort=medium` | `--model opus`, foreground | `Gemini 3.1 Pro (High)`, foreground |
| `high` | `model_reasoning_effort=high` | `--model opus`, background, ask for deeper analysis in the prompt | `Gemini 3.1 Pro (High)`, background, `--print-timeout 20m` |

State the chosen tier and the one-line reason before running.

## Step 5 — Run the critique

Write the prompt to `.workspace/critique-prompt.md`, then run from the repo root. Every recipe below
is read-only: the critic can read and search the repo but cannot edit it.

**Prompt template** — adjust the bracketed parts only:

```
You are an independent design reviewer with read-only access to this repository.
[agy only: First read AGENTS.md for the project's house rules.]
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

The house-rules line is conditional because auto-loading differs: **codex** reads `AGENTS.md` and
**claude** reads `CLAUDE.md` (a symlink to `AGENTS.md` here) on their own — don't restate house rules
for them. **agy** loads neither, so it needs the explicit pointer.

**codex:**
```bash
codex exec -s read-only -m gpt-5.6-sol \
  -c model_reasoning_effort="<tier>" \
  -o .workspace/spec-critique-raw.md \
  - < .workspace/critique-prompt.md
```

**claude** — `--permission-mode plan` is the read-only mode; the critique is the final message on stdout:
```bash
claude -p --permission-mode plan --model opus \
  < .workspace/critique-prompt.md \
  > .workspace/spec-critique-raw.md
```

**agy** — `--mode plan` is the read-only mode; the prompt is the *value* of `-p` and must come after
the other flags (flags placed after it get swallowed into the prompt):
```bash
agy --mode plan --model "Gemini 3.1 Pro (High)" --print-timeout 20m \
  -p "$(cat .workspace/critique-prompt.md)" \
  > .workspace/spec-critique-raw.md
```

`agy`'s `--model` wants its own display string, not the `gemini-3.1-pro` model id; confirm the
current strings with `agy models`.

Run in the background (`run_in_background`) for `high` — it can take several minutes — and in the
foreground with a generous timeout (600000) otherwise. This costs real tokens: never loop it, and
never re-run without a changed spec or a changed question. If the chosen reviewer errors out or is
out of credit, fall to the next entry in Step 2's order and note the substitution.

## Step 6 — Record and triage

1. **Verify every finding before accepting it.** Check each claim against the file, doc, or behavior
   it cites; a "likely" asserted over something checkable is a question, not a finding. Record the
   outcome per finding: CONFIRMED, REFUTED (name the contradicting evidence), or OUT-OF-SCOPE (say why).
2. Summarize the verified findings to the user ranked by severity, with your own accept/reject
   recommendation per finding — the critic has no product context, so some findings are legitimately
   out of scope; say why when rejecting.
3. Revise the issue body for accepted findings, and note at the top:
   `Critiqued <date> (<tool>/<model>, <tier>): N findings, M accepted — see critique comment.`
   Record the reviewer that actually ran, so a later reader can tell whether independence held. A
   spec with all blocker/major findings resolved (fixed or explicitly rejected with reasons) is ready
   for `writing-plans` and the human approval gate (`status/design` → `status/ready`).
4. Post the critique to the issue as a comment
   (`gh issue comment <n> --body-file .workspace/spec-critique-raw.md`).
