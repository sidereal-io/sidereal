---
name: choose-an-adversary
description: Use when a review must be run by a different model family than the one that authored the work — an independent adversary, not a self-review. Called by critique skills that need real independence.
---

# Choosing an Adversary

A critic that inherits the author's reasoning just confirms it. Independence is a property of the model family that answers, not the CLI that launches it. This skill picks an independent family, runs the critique read-only, and hands the raw findings back. The caller verifies and presents them.

## Establish the author family

Find who wrote the work. First answer wins:

- **A provenance note** on the artifact.
- **This session** — you wrote or shaped the work here.
- **Ask the author.** Do not guess. A wrong answer turns the review into a rubber stamp.

On this machine the author is almost always `claude` — this session or a Claude subagent.

## Pick the reviewer family

Take the first family that is not the author's and whose executor is installed (`command -v`).

| Family | Executor | Reasoning dial |
|---|---|---|
| `gpt` | `codex` | `-c model_reasoning_effort=` |
| `claude` | `claude` | model choice |
| `gemini` | `agy` | folded into model choice |

Order: `gpt` → `claude` → `gemini`. If the only installed family is the author's, stop and report "no independent adversary available" so the caller can fall back to an in-session critique.

## Scale the scrutiny

Score what the work changes, not its length.

| Tier | Fits |
|---|---|
| `low` | Narrow fix or polish; one unit; no new contracts |
| `medium` | Single subsystem on known patterns; contained blast radius |
| `high` | Multiple units; new contracts, migrations, or trust boundaries; architecture-changing |

Use the reviewer's strongest reasoning model. Run `low` and `medium` in the foreground. Run `high` in the background with a long timeout. Discover current model ids at run time (for example, `agy models`) rather than trusting names written here — ids rot.

## Run it read-only

Write the critique prompt to a scratch file. Run from the work's directory. Every recipe below is read-only.

```bash
# codex — read-only sandbox; reasoning is its own dial
codex exec -s read-only -m <model> -c model_reasoning_effort="<tier>" \
  -o <scratch>/critique-raw.md - < <scratch>/critique-prompt.md

# claude — plan mode is read-only
claude -p --permission-mode plan --model <model> \
  < <scratch>/critique-prompt.md > <scratch>/critique-raw.md

# agy — plan mode is read-only; the prompt is the value of -p and comes last
agy --mode plan --model "<a gemini model from agy models>" --print-timeout <timeout> \
  -p "$(cat <scratch>/critique-prompt.md)" > <scratch>/critique-raw.md
```

This costs real tokens. Never loop it. Re-run only when the work or the question changed. If an executor errors or runs out of credit, fall to the next family and tell the caller about the substitution.

## Hand back the raw findings

Return the raw findings file to the caller. Do not triage here. The caller has the product context to judge which findings matter.
