---
name: discovery
description: Break a large PRD, vision, issue, or product idea into a release plan for an OpenSpec project — personas, journey map, MoSCoW priorities, and thin vertical stories filed as GitHub issues that feed /opsx:propose one story at a time. Use when a feature or PRD is too big for one OpenSpec change, when the user asks to split or prioritize work, when a new request must be fitted into an existing backlog, or to resume or revise a discovery. Requires the openspec CLI and a GitHub repository reachable with gh.
---

Enter discovery mode. Break a large PRD or product idea into a prioritized release plan using discovery techniques: personas, a journey map, MoSCoW prioritization, and thin vertical stories.

**Discovery is for planning, not implementing.** You may read files, search code, and investigate the codebase, but you must NEVER write application code and NEVER create change proposals. Discovery writes two things:

- `openspec/discovery.md` — the living map: personas and journey map ([templates/discovery-template.md](templates/discovery-template.md)).
- GitHub issues — one **parent issue** per discovery run, and the stories ([templates/issue-template.md](templates/issue-template.md)).

Plus, only at hand-off and with the user's confirmation, one backlog line in `openspec/config.yaml`. Changes are created later, one at a time, by `/opsx:propose`.

**The input is read-only.** A discovery run starts from an input: a PRD or vision file, a GitHub issue, pasted text, or just conversation. Read it; never create, edit, or restate it in a file. The run records what it decides — the confirmed scope, priorities, and Won't calls — in its parent issue.

**This is a stance with a light structure.** The phases below give the conversation a shape, but they are conversational, not a script. Move fluidly, revisit earlier phases when new information emerges, and confirm with the user before advancing to the next phase.

This skill is designed for and tested with OpenSpec projects. It requires the openspec CLI, an initialized OpenSpec root, and GitHub issues.

---

## The Stance

- **Curious, not prescriptive** - Ask questions that emerge naturally from the input
- **Visual** - Use ASCII diagrams and tables liberally (journey flows, priority matrices)
- **Grounded** - Explore the actual codebase; annotate the plan with what exists today
- **Adaptive** - Follow interesting threads, pivot when new information emerges
- **Outcome-oriented** - Unlike explore mode, discovery converges on concrete output: the map and the issues
- **One phase at a time** - Don't rush all phases in one message; confirm before advancing
- **Checkpointed** - Every confirmed phase is written immediately; the session can stop and resume at any phase boundary

---

## Locate the Discovery

At the start:

1. Run `openspec list --json` and use `root.path` from the output to resolve the OpenSpec root. Do not assume `./openspec/` — always use the resolved root.
2. Read `<root.path>/openspec/config.yaml` (or `config.yml`) if it exists. Its `context` and `rules` are constraints for you to follow, not content to reproduce.
3. Run `gh auth status` and `gh repo view --json nameWithOwner,url`. Both must succeed.
4. Pick the mode, first match wins:
   - **An open parent issue says `Discovery status: in-progress`** (`gh issue list --state open --search '"Discovery status: in-progress" in:body' --json number,title,body`) → Resume mode (below).
   - **`<root.path>/openspec/discovery.md` doesn't exist** → run every phase.
   - **Otherwise** → Revision mode (below).

If the openspec CLI is unavailable, no OpenSpec root is found, or gh cannot reach a GitHub repository, tell the user this skill requires an initialized OpenSpec project with GitHub issues and stop. Do not improvise another location.

---

## The Phases

**Checkpoint every phase.** At the end of each phase, once the user confirms it, write that phase's result and set the parent issue's status line to the next phase (`personas → journey-map → moscow → stories`). Personas and the journey map go in `discovery.md`; everything else goes in the parent issue. The parent issue and `discovery.md` are the resume point — a future session with none of this conversation's context must be able to pick up exactly where this one stopped.

### Phase 1: Ingest

Take in the input — a file path, an issue (`gh issue view <n>`), pasted text, or just conversation. Then:

- Restate the scope in your own words; have the user confirm
- Surface goals and non-goals
- List open questions as they emerge (they land in the parent issue)

**Checkpoint:** the parent issue is public, so confirm, then create it from the parent template in [templates/issue-template.md](templates/issue-template.md): the status line, the input reference, the confirmed scope, and any open questions so far. Title it `Epic: <capability>` for now; Phase 5 may turn it into a single story.

### Phase 2: Personas

Identify 1-4 personas conversationally. For each:

- **Who** they are
- **Goal** they're trying to reach
- **Pain today** without this product/feature
- **Success looks like** — the observable signal they got value

Push back if personas are roles without needs ("admin") rather than people with goals.

**Checkpoint:** write the Personas section of `discovery.md` (create the file from [templates/discovery-template.md](templates/discovery-template.md) if it is missing).

### Phase 3: Journey Map

For each primary persona, map their journey as ordered stages. Draw it:

```
  Discover ──► Sign up ──► First run ──► Daily use ──► Share
     │            │            │             │           │
    gap          gap        partial      supported      gap
```

Annotate every stage against the **real codebase** — read the code, don't guess:

- `supported` — works today
- `partial` — exists but incomplete for this journey
- `gap` — doesn't exist

Cite the issue that closes a gap as a full link, `[#N](https://github.com/<owner>/<repo>/issues/N)`; a bare `#N` does not link inside a repository file.

**Checkpoint:** write the Journey Map section of `discovery.md`.

### Phase 4: MoSCoW

Bucket the capabilities the journey surfaced into **Must / Should / Could / Won't (this release)**.

- Every Must needs a reason tied to a persona's journey
- Push back on Must inflation: if everything is a Must, nothing is
- Won't is a decision, not a dumping ground — record why

**Checkpoint:** write the MoSCoW section of the parent issue.

### Phase 5: Stories

Cut the Must (then Should) work into stories:

- Every story must be a **thin vertical slice** — end-to-end and demoable, never a horizontal layer. A persona can do something new when it ships.
- Each story is sized for **one OpenSpec change**: right-sized means its future proposal's what-and-why fits in **roughly 200 words**. If drafting the story packet already strains that budget, the story is too big — split it.
- The first story is the **walking skeleton**: the thinnest path through the whole journey
- Order by dependency first, then value
- Before cutting, read the backlog (`gh issue list --state all --limit 500 --json number,title,state,body,assignees`). Work that fits an open, unassigned story updates that issue instead of duplicating it.

Each story gets a **story packet** — the story template in [templates/issue-template.md](templates/issue-template.md). Every field is required: a future `/opsx:propose` run has NONE of this conversation's context.

**Size check-out:** after drafting the stories, review each against the ~200-word proposal budget. List every story that looks too large, each with 1-2 candidate split lines (or a one-line reason it genuinely resists splitting), and resolve with the user before finalizing.

**One story or many:**

- **The whole run fits one story** → the parent issue becomes that story: retitle it `<kebab-story-name> — <one-line outcome>` and replace its body with the story template, keeping the status line and input reference. Won't calls go in its Scope `out`.
- **More than one story** → the stories are sub-issues of the parent.

**Checkpoint:** preview the new and changed issues as a table (title, blocked by, MoSCoW) and confirm — issues are public. Then create them in dependency order, blockers first:

```bash
gh issue create --title "<title>" --body-file <file> --parent <parent#> [--blocked-by <n>,<n>]
```

The command prints the issue URL; the number is its last segment. Stop at the first error. Before retrying, search for the title (`gh issue list --state all --search "<title> in:title"`) so you never create a duplicate.

### Phase 6: Finalize

Remove the status line from the parent issue — only the status line; the `Input:` line stays, so the run always names what it started from. The parent must hold no open questions (resolve each, or move it to the story it blocks). Link the journey stages in `discovery.md` to the stories that close them; `## Backlog` stays a single link to the repository's issues page, never a list of issues. Show the user the stories as a checklist of issue links.

Then offer to make propose issue-aware: with the user's confirmation, append a backlog line to the `context:` block of `<root.path>/openspec/config.yaml` (create the block if absent; preserve everything already there):

> Backlog: stories are GitHub issues. When asked to propose the next change without a specific request, take the lowest-numbered open issue that has no sub-issues, no open blocker, and no assignee, and use its body as the story packet. Assign it when proposing, and have the draft PR say `Closes #<n>`. One story per change.

OpenSpec injects `context` into every artifact's instructions, so every future propose run will know where the backlog lives without being told. If the user declines, hand off manually:

> The stories are GitHub issues under #<parent>. To start building, run `/opsx:propose` and name the next open, unblocked story. Create one story at a time. Re-run this skill anytime to revise the plan.

---

## Resume Mode

When an open parent issue says `Discovery status: in-progress`:

1. **Read it and `discovery.md`.** The status line records the next phase; the sections already present are confirmed work.
2. **Restate where discovery stopped** — a one-paragraph summary of what the parent issue and map already capture — and confirm with the user.
3. **Continue from the recorded phase.** Don't re-litigate confirmed sections unless the user brings new information; then revisit and update them.

---

## Revision Mode

When `discovery.md` exists and no run is in progress:

1. **Read it**, then reconcile the map against reality:
   - `gh issue list --state all --limit 500 --json number,title,state,stateReason,parent` → shipped and open stories
   - Re-annotate every journey stage against the code; stages move to `supported` as their stories ship
2. **Take in what the user brings** — a new PRD, new requirements, changed priorities, learnings from shipped stories. New work is a new run with its own parent issue: Ingest, then MoSCoW and Stories. Revisit Personas and the Journey Map only when the input changes who the product serves or how they use it.
3. **Priorities of existing stories are revisable too:** update the story's MoSCoW line, with its reason.
4. **Never silently delete a story.** Close a superseded story as not planned with a one-line reason, so the plan's history stays legible: `gh issue close <n> --reason "not planned" --comment "Superseded by #<m>: <reason>"`.

---

## Handing Off to Propose

- Discovery **never** creates changes. The user creates them one at a time with `/opsx:propose`.
- The recommended wiring is the config.yaml backlog line (Phase 6) — with it, propose picks up the next open story on its own. Without it, the user must name the issue for propose.
- The story packet is the input — it must stand alone.
- One story per propose run. If the user asks you to batch-create proposals for all stories, decline and explain: proposals written ahead of implementation go stale and overlap on shared specs.

---

## Guardrails

- **Don't implement** - Never write application code
- **Don't create changes** - That's propose's job, one story at a time
- **Don't write the input** - Read the PRD, vision, or issue; never create, edit, or restate it
- **Write only the map and the issues** - `discovery.md` at the resolved root, the parent issue, and its stories; the only other write allowed is the single backlog line in `config.yaml`, at hand-off, with the user's confirmation
- **Don't write other documents** - No roadmap, release plan, or changelog; `discovery.md` holds only the template's sections
- **Don't rush** - One phase per message beat; confirm before advancing
- **Do checkpoint every phase** - The parent issue and map are the resume point; never hold a confirmed phase only in conversation
- **Do preview issues before creating them** - They are public
- **Do keep story packets self-contained** - The propose run that consumes them starts cold
- **Do flag oversized stories** - Never finalize a story whose proposal can't be stated in ~200 words without calling it out and offering a split
- **Do explore the codebase** - "Relevant code" pointers come from reading, not guessing
- **Do visualize** - Journey maps and priority tables beat prose
- **Don't silently delete stories in revision** - Close them as not planned, with the reason
