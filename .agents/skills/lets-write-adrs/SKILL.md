---
name: lets-write-adrs
description: Use when deciding which decisions in a design deserve an architecture decision record (ADR), and when writing one that stands alone. Says what qualifies, what does not, and how records relate to each other. The schema says where they live.
---

# Writing ADRs

An architecture decision record (ADR) keeps the why of a decision that is
costly to reverse, so that no one reverses it by accident. Most decisions do
not need one.

## What qualifies

Write an ADR for a real fork: a choice among viable approaches where
changing your mind later is expensive. The test has two parts. Could an
engineer reading only the code plausibly reverse this decision without
knowing why? Would that reversal be costly? If yes to both, the why belongs
in an ADR.

Typical categories: a stack, framework or library choice; a data model or
schema; an authentication strategy; an API architecture; an infrastructure,
hosting or provider choice; a privacy posture.

Examples that qualify:

- **A stack forced by a constraint.** "Use an embedded datastore, not a networked database, because the app must run offline." Reversing it touches every storage call.
- **A canonical data model.** "Convert every input to one internal shape at its edge." Many consumers bind to that shape, so changing it later is costly.
- **A system boundary.** "Keep the core a pure library that emits one document, and make the integration a swappable adapter." The core stays testable on its own.

## What does not

- **Recoverable from code.** "Set the retry limit to 3." It lives in code, costs nothing to change, and you learn it by reading it.
- **Roadmap sequencing.** "Build reporting after billing." A roadmap decision, not an architectural fork.
- **A requirement.** "Reject uploads over 10 MB." The spec owns that behavior; there is no fork between approaches.

When nothing qualifies, say so. Never invent a record to have one.

## Reading the existing records

Before writing, read every existing ADR. Note the highest number in use, and
which records supersede which, so a new record names the one it replaces or
builds on. Rewrite a record that is hard to read; never delete it. The
decision still stands.

## Writing one

- **One decision per record.** Two forks are two records.
- **Stand alone.** Give the context the reader needs here. Do not send them to an issue, a pull request or a roadmap; those change, and the decision does not.
- **Link at most one other ADR:** the one this supersedes or builds on.
- **Name the alternatives,** and say specifically why each lost. "Too complex" is not a reason; "needs a running server, and the app must work offline" is.
- **Consequences both ways.** What becomes easier, and what becomes harder.
- **Plain language.** Short sentences, active voice, the everyday word.
